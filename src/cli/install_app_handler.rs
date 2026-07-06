use std::path::{Path, PathBuf};

use crate::cli::color::Color;
use crate::cli::github_release::fetch_release_for;
use crate::cli::progress_bar::ProgressBar;
use crate::cli::result::{HandlerError, HandlerResult};
use crate::github::client::GithubClient;
use crate::github::release::{Asset, Tag};
use crate::github::repository::Repository;
use crate::installer::portable_app::{PortableAppConfig, PortableAppInstaller};
use crate::system;
use crate::system::System;
use crate::temp_file;

use super::select_assets;

pub struct InstallAppHandler {
    repo: Option<Repository>,
    pkg: Option<PathBuf>,
    bin: String,
    name: Option<String>,
    icon: Option<String>,
    cicon: Option<PathBuf>,
    service: bool,
    autostart: bool,
    yes: bool,
    select: Option<String>,
    automatic: bool,
    tag: Option<String>,
    app_root: Option<PathBuf>,
}

impl InstallAppHandler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repo: Option<Repository>,
        pkg: Option<PathBuf>,
        bin: String,
        name: Option<String>,
        icon: Option<String>,
        cicon: Option<PathBuf>,
        service: bool,
        autostart: bool,
        yes: bool,
        select: Option<String>,
        automatic: bool,
        tag: Option<String>,
        app_root: Option<PathBuf>,
    ) -> Self {
        Self {
            repo,
            pkg,
            bin,
            name,
            icon,
            cicon,
            service,
            autostart,
            yes,
            select,
            automatic,
            tag,
            app_root,
        }
    }

    pub fn run(&self) -> HandlerResult {
        let pkg_path = if let Some(ref pkg) = self.pkg {
            // Local mode: use the provided path directly
            if !pkg.exists() {
                return Err(HandlerError::new(format!(
                    "Package not found: {}",
                    pkg.display()
                )));
            }
            pkg.clone()
        } else {
            // Download mode: fetch from GitHub
            let repo = self
                .repo
                .as_ref()
                .ok_or_else(|| HandlerError::new("Either --pkg or a repository is required".into()))?;

            let github = GithubClient::from_environment();
            let tag = self.tag.as_ref().map(|t| Tag(t.clone()));
            let release = fetch_release_for(&github, repo, tag.as_ref())?;

            // Select asset
            let selected_asset = self.select_asset(release)?;

            // Download to temp file
            let temp_dir = temp_file::make_temp_dir()
                .map_err(|e| HandlerError::new(format!("Failed to create temp dir: {}", e)))?;
            let temp_path = temp_dir.join(&selected_asset.name);

            let progress_bar =
                ProgressBar::download_layout(&selected_asset.name, &temp_path);
            progress_bar.show();

            let (mut stream, maybe_content_length) = github
                .download_asset_stream(&selected_asset)
                .map_err(|e| HandlerError::new(format!("Error downloading asset: {}", e)))?;
            progress_bar.set_length(maybe_content_length);

            let mut destination = std::fs::File::create(&temp_path)
                .map_err(|e| HandlerError::new(format!("Failed to create file: {}", e)))?;
            use std::io::Read;
            use std::io::Write;
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

            temp_path
        };

        // Derive app name from the filename if not provided
        let app_name = self.name.clone().unwrap_or_else(|| {
            pkg_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(derive_name)
                .unwrap_or_else(|| "app".to_string())
        });

        let config = PortableAppConfig {
            app_name: app_name.clone(),
            bin_rel_path: self.bin.clone(),
            icon_rel_path: self.icon.clone(),
            cicon_path: self.cicon.clone(),
            service: self.service,
            autostart: self.autostart,
            yes: self.yes,
            app_root: self.app_root.clone(),
        };

        let result = PortableAppInstaller::run(&pkg_path, config)
            .map_err(|e| HandlerError::new(e))?;

        println!("{}", Color::new("Installation completed!").green());
        println!("{}", result.summary(&app_name));

        Ok(())
    }

    fn select_asset(&self, release: crate::github::release::Release) -> Result<Asset, HandlerError> {
        let mode = match (&self.select, self.automatic) {
            (Some(s), _) => AssetSelectionMode::Pattern(s.clone()),
            (_, true) => AssetSelectionMode::Automatic,
            (None, false) => AssetSelectionMode::Interactive,
        };

        match mode {
            AssetSelectionMode::Interactive => {
                let assets = release.assets;
                select_assets::ask_select_asset(
                    assets,
                    select_assets::Messages {
                        select_prompt: "Pick the asset to download and install",
                        quit_select: "No asset selected",
                    },
                )
            }
            AssetSelectionMode::Pattern(selection) => {
                let asset_name = crate::github::tagged_asset::TaggedAsset::tag(&release.tag, &selection);
                let pattern = wildmatch::WildMatch::new(&asset_name);
                release
                    .assets
                    .into_iter()
                    .find(|x| pattern.matches(&x.name))
                    .ok_or_else(|| HandlerError::new(format!("No asset found for {}", selection)))
            }
            AssetSelectionMode::Automatic => {
                let system = system::from_environment()
                    .map_err(|e| HandlerError::new(format!("System detection error: {}", e)))?;
                system::find_asset_by_system(&system, release.assets).ok_or_else(|| {
                    HandlerError::new(format!(
                        "Cannot find asset that matches your system {} {}",
                        system.os(),
                        system.arch()
                    ))
                })
            }
        }
    }
}

enum AssetSelectionMode {
    Interactive,
    Pattern(String),
    Automatic,
}

/// Derive app name from a filename: take the first segment before '-'.
fn derive_name(filename: &str) -> String {
    filename.split('-').next().unwrap_or(filename).to_string()
}

/// Handle `dra uninstall-app <name>`.
pub fn handle_uninstall_app(name: &str) -> HandlerResult {
    let home = std::env::var("HOME")
        .map_err(|_| HandlerError::new("HOME environment variable not set".into()))?;
    let app_dir = Path::new(&home).join(".local").join("app").join(name);

    if !app_dir.exists() {
        return Err(HandlerError::new(format!(
            "App '{}' not found at {}",
            name,
            app_dir.display()
        )));
    }

    let uninstall_sh = app_dir.join("uninstall.sh");
    if uninstall_sh.exists() {
        // Run the uninstall script
        let status = std::process::Command::new("bash")
            .arg(&uninstall_sh)
            .status()
            .map_err(|e| HandlerError::new(format!("Failed to run uninstall script: {}", e)))?;

        if status.success() {
            println!("{}", Color::new(&format!("Successfully uninstalled '{}'", name)).green());
            Ok(())
        } else {
            Err(HandlerError::new(format!(
                "Uninstall script exited with code: {:?}",
                status.code()
            )))
        }
    } else {
        // Fallback: manual removal
        eprintln!("[!] No uninstall.sh found, removing directory manually");
        std::fs::remove_dir_all(&app_dir)
            .map_err(|e| HandlerError::new(format!("Failed to remove {}: {}", app_dir.display(), e)))?;
        println!("{}", Color::new(&format!("Removed {}", app_dir.display())).green());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_name_from_asset() {
        assert_eq!(derive_name("vscode-1.80.0-linux-x64.tar.gz"), "vscode");
        assert_eq!(derive_name("code-server-4.0.0.tar.gz"), "code");
        assert_eq!(derive_name("myapp"), "myapp");
        assert_eq!(derive_name("simple-name-v2.0.tar.gz"), "simple");
    }
}