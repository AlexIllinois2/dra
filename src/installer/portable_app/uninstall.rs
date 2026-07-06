use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Generate an uninstall.sh script in the app directory.
pub fn write_uninstall_script(
    app_dir: &Path,
    app_name: &str,
    symlink_paths: &[PathBuf],
    has_service: bool,
) -> Result<PathBuf, String> {
    let uninstall_path = app_dir.join("uninstall.sh");

    let mut lines = vec![
        "#!/usr/bin/env bash".to_string(),
        "set -e".to_string(),
        String::new(),
    ];

    if has_service {
        lines.push(format!(
            "echo \"[+] Stopping and disabling service: {}.service\"",
            app_name
        ));
        lines.push(format!(
            "systemctl --user stop {}.service 2>/dev/null || true",
            app_name
        ));
        lines.push(format!(
            "systemctl --user disable {}.service 2>/dev/null || true",
            app_name
        ));
        lines.push("systemctl --user daemon-reload 2>/dev/null || true".to_string());
        lines.push(String::new());
    }

    lines.push("echo \"[+] Removing symlinks...\"".to_string());
    for link in symlink_paths {
        lines.push(format!("rm -f \"{}\"", link.display()));
    }
    lines.push(String::new());

    lines.push(format!(
        "echo \"[+] Removing app directory: {}\"",
        app_dir.display()
    ));
    lines.push(format!("rm -rf \"{}\"", app_dir.display()));
    lines.push(String::new());

    lines.push("echo \"[+] Uninstallation completed successfully.\"".to_string());

    let content = lines.join("\n");
    fs::write(&uninstall_path, &content)
        .map_err(|e| format!("Failed to write uninstall.sh: {}", e))?;

    // chmod 0o755
    let metadata = fs::metadata(&uninstall_path)
        .map_err(|e| format!("Failed to get uninstall.sh metadata: {}", e))?;
    let mut perms = metadata.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&uninstall_path, perms)
        .map_err(|e| format!("Failed to set uninstall.sh permissions: {}", e))?;

    Ok(uninstall_path)
}