use std::path::Path;

use crate::cli::color::Color;
use crate::cli::result::{HandlerError, HandlerResult};
use crate::registry;
use crate::registry::app_record::InstallType;

pub fn handle_remove(app_name: &str) -> HandlerResult {
    let mut registry = registry::Registry::load();

    let record = registry
        .get_record(app_name)
        .map(|r| r.clone())
        .ok_or_else(|| {
            HandlerError::new(format!(
                "No installed app found with name '{}'. Run 'dra list' to see installed apps.",
                app_name
            ))
        })?;

    println!("{}", Color::new(&format!("Removing '{}'...", app_name)).bold());

    match record.install_type {
        InstallType::Bin | InstallType::ArchiveBin | InstallType::AppImage => {
            remove_bin_type(&record.executables)?;
        }
        InstallType::PortableApp => {
            remove_portable_app(app_name)?;
        }
    }

    // Update registry
    registry.remove_record(app_name);
    registry.save().map_err(|e| HandlerError::new(e))?;

    println!(
        "{}",
        Color::new(&format!("Successfully uninstalled '{}'", app_name)).green()
    );
    Ok(())
}

/// Remove executables from ~/.local/bin for Bin/ArchiveBin/AppImage types.
fn remove_bin_type(executables: &[String]) -> Result<(), HandlerError> {
    let home = std::env::var("HOME")
        .map_err(|_| HandlerError::new("HOME environment variable not set".into()))?;
    let bin_dir = Path::new(&home).join(".local").join("bin");

    for exe in executables {
        let path = bin_dir.join(exe);
        if path.exists() || path.is_symlink() {
            std::fs::remove_file(&path)
                .map_err(|e| HandlerError::new(format!("Failed to remove {}: {}", path.display(), e)))?;
            println!("  Removed: {}", path.display());
        } else {
            println!("  Not found, skipping: {}", path.display());
        }
    }
    Ok(())
}

/// Remove a portable app (type PortableApp).
fn remove_portable_app(app_name: &str) -> Result<(), HandlerError> {
    let home = std::env::var("HOME")
        .map_err(|_| HandlerError::new("HOME environment variable not set".into()))?;
    let app_dir = Path::new(&home).join(".local").join("app").join(app_name);
    let bin_dir = Path::new(&home).join(".local").join("bin");
    let desktop_dir = Path::new(&home)
        .join(".local")
        .join("share")
        .join("applications");
    let systemd_dir = Path::new(&home)
        .join(".config")
        .join("systemd")
        .join("user");
    let autostart_dir = Path::new(&home).join(".config").join("autostart");

    // Remove symlinks
    let links = [
        bin_dir.join(app_name),
        desktop_dir.join(format!("{}.desktop", app_name)),
        systemd_dir.join(format!("{}.service", app_name)),
        autostart_dir.join(format!("{}.desktop", app_name)),
    ];
    for link in &links {
        if link.exists() || link.is_symlink() {
            std::fs::remove_file(link)
                .map_err(|e| HandlerError::new(format!("Failed to remove {}: {}", link.display(), e)))?;
            println!("  Removed: {}", link.display());
        }
    }

    // Stop and disable systemd service if it exists
    if systemd_dir.join(format!("{}.service", app_name)).exists() {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", &format!("{}.service", app_name)])
            .output();
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "disable", &format!("{}.service", app_name)])
            .output();
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();
    }

    // Remove app directory
    if app_dir.exists() {
        std::fs::remove_dir_all(&app_dir).map_err(|e| {
            HandlerError::new(format!(
                "Failed to remove app directory {}: {}",
                app_dir.display(),
                e
            ))
        })?;
        println!("  Removed: {}", app_dir.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use registry::app_record::AppRecord;

    /// Helper to create a temp registry with a test record
    fn setup_test_registry(registries: &registry::Registry) -> tempfile::TempDir {
        let tmp = tempfile::TempDir::new().unwrap();
        // Override registry path by writing directly
        let path = tmp.path().join("installed_apps.json");
        let json = serde_json::to_string_pretty(registries).unwrap();
        std::fs::write(&path, &json).unwrap();
        tmp
    }

    #[test]
    fn test_remove_record_from_registry() {
        let mut registry = registry::Registry::default();
        registry.add_record(AppRecord::new(
            "test-app".to_string(),
            "test/repo".to_string(),
            "v1.0.0".to_string(),
            InstallType::ArchiveBin,
            "/tmp/test-bin".to_string(),
            vec!["test-app".to_string()],
        ));

        assert_eq!(registry.list_records().len(), 1);
        registry.remove_record("test-app");
        assert_eq!(registry.list_records().len(), 0);
    }
}