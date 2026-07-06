use crate::cli::color::Color;
use crate::cli::result::{HandlerError, HandlerResult};
use crate::github::client::GithubClient;
use crate::github::release::Tag;
use crate::github::repository::Repository;
use crate::registry;
use crate::registry::app_record::InstallType;

/// Handle `dra update` - update all apps or a specific one.
pub fn handle_update(name: Option<String>) -> HandlerResult {
    let registry = registry::Registry::load();
    let records = registry.list_records().to_vec();

    if records.is_empty() {
        println!("{}", Color::new("No installed apps to update.").bold());
        return Ok(());
    }

    let to_update: Vec<_> = if let Some(ref name) = name {
        let record = registry
            .get_record(name)
            .ok_or_else(|| {
                HandlerError::new(format!(
                    "No installed app found with name '{}'. Run 'dra list' to see installed apps.",
                    name
                ))
            })?;
        vec![record.clone()]
    } else {
        records
    };

    let github = GithubClient::from_environment();

    for record in &to_update {
        println!(
            "{}",
            Color::new(&format!("Checking '{}' ({}) ...", record.name, record.repo)).bold()
        );

        match update_single_app(&github, record) {
            Ok(updated) => {
                if updated {
                    println!(
                        "{}",
                        Color::new(&format!("  '{}' updated successfully.", record.name)).green()
                    );
                } else {
                    println!("  Already up-to-date.");
                }
            }
            Err(e) => {
                eprintln!("  {} {:?}", Color::new("Error:").red(), e);
                eprintln!("  Skipping '{}'", record.name);
            }
        }
    }

    Ok(())
}

/// Check and update a single app. Returns true if an update was performed.
fn update_single_app(github: &GithubClient, record: &registry::app_record::AppRecord) -> Result<bool, HandlerError> {
    let repo: Repository = Repository::try_parse(&record.repo)
        .map_err(|e| HandlerError::new(format!("Invalid repository '{}': {}", record.repo, e)))?;

    // Fetch latest release
    let release = crate::cli::github_release::fetch_release_for(github, &repo, None)?;
    let latest_tag = release.tag.0.clone();

    // Normalize version comparison (strip 'v' prefix)
    let current_normalized = record.installed_version.trim_start_matches('v');
    let latest_normalized = latest_tag.trim_start_matches('v');

    if current_normalized == latest_normalized {
        return Ok(false); // Already up to date
    }

    println!(
        "  New version available: {} (current: {})",
        Color::new(&latest_tag).green(),
        record.installed_version
    );

    // Perform update based on install type
    match record.install_type {
        InstallType::Bin | InstallType::ArchiveBin | InstallType::AppImage => {
            update_bin_type(github, &repo, &latest_tag, record)?;
        }
        InstallType::PortableApp => {
            return Err(HandlerError::new(
                "Update for PortableApp type is not yet supported via 'dra update'. \
                 Use 'dra install-app' with the latest version instead."
                    .to_string(),
            ));
        }
    }

    // Update registry
    let mut reg = registry::Registry::load();
    if let Some(existing) = reg.get_record(&record.name) {
        let mut updated = existing.clone();
        updated.installed_version = latest_tag;
        reg.add_record(updated);
    }
    reg.save().map_err(|e| HandlerError::new(e))?;

    Ok(true)
}

/// Update Bin/ArchiveBin/AppImage type by re-downloading and reinstalling.
fn update_bin_type(
    github: &GithubClient,
    repo: &Repository,
    latest_tag: &str,
    record: &registry::app_record::AppRecord,
) -> Result<(), HandlerError> {
    // Fetch release with the latest tag
    let release = crate::cli::github_release::fetch_release_for(github, repo, Some(&Tag(latest_tag.to_string())))?;

    // Try to find the same asset that was originally selected
    // We look for an asset whose name might match the original pattern
    let asset = release
        .assets
        .into_iter()
        .next()
        .ok_or_else(|| HandlerError::new("No assets found in latest release".to_string()))?;

    let temp_dir = crate::temp_file::make_temp_dir()
        .map_err(|e| HandlerError::new(format!("Failed to create temp dir: {}", e)))?;
    let temp_path = temp_dir.join(&asset.name);

    // Download asset
    use std::io::{Read, Write};
    let progress_bar = crate::cli::progress_bar::ProgressBar::download_layout(&asset.name, &temp_path);
    progress_bar.show();
    let (mut stream, maybe_content_length) = github
        .download_asset_stream(&asset)
        .map_err(|e| HandlerError::new(format!("Error downloading asset: {}", e)))?;
    progress_bar.set_length(maybe_content_length);

    let mut destination = std::fs::File::create(&temp_path)
        .map_err(|e| HandlerError::new(format!("Failed to create file: {}", e)))?;
    let mut total_bytes = 0u64;
    let mut buffer = [0u8; 1024];
    while let Ok(bytes) = stream.read(&mut buffer) {
        if bytes == 0 {
            break;
        }
        destination
            .write(&buffer[..bytes])
            .map_err(|e| HandlerError::new(format!("Failed to write: {}", e)))?;
        total_bytes += bytes as u64;
        progress_bar.update_progress(total_bytes);
    }
    progress_bar.finish();

    // Remove old executables first
    let home =
        std::env::var("HOME").map_err(|_| HandlerError::new("HOME not set".into()))?;
    let bin_dir = std::path::Path::new(&home).join(".local").join("bin");
    for exe in &record.executables {
        let path = bin_dir.join(exe);
        if path.exists() || path.is_symlink() {
            std::fs::remove_file(&path).ok();
        }
    }

    // Install new version using the existing installer
    let executables: Vec<_> = record
        .executables
        .iter()
        .map(|name| crate::installer::executable::Executable::Selected(name.clone()))
        .collect();
    let destination = crate::installer::destination::Destination::Directory(bin_dir);

    let output = crate::installer::install(
        asset.name.clone(),
        &temp_path,
        destination,
        executables,
    )
    .map_err(|e| HandlerError::new(e.to_string()))?;

    std::fs::remove_file(&temp_path).ok();

    println!("  {}", output);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_not_needed() {
        // When versions are the same, no update is needed
        let current = "v1.0.0".to_string();
        let latest = "v1.0.0".to_string();
        assert_eq!(
            current.trim_start_matches('v'),
            latest.trim_start_matches('v')
        );
    }

    #[test]
    fn test_update_needed() {
        let current = "v1.0.0".to_string();
        let latest = "v2.0.0".to_string();
        assert_ne!(
            current.trim_start_matches('v'),
            latest.trim_start_matches('v')
        );
    }
}