use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Generate a systemd user service file in the app directory.
pub fn generate_service(
    app_dir: &Path,
    app_name: &str,
    bin_rel_path: &str,
) -> Result<PathBuf, String> {
    let service_path = app_dir.join(format!("{}.service", app_name));
    let exec_path = app_dir.join(bin_rel_path);

    let content = format!(
        "[Unit]
Description={}

[Service]
ExecStart={}
WorkingDirectory={}
Restart=on-failure

[Install]
WantedBy=default.target
",
        app_name,
        exec_path.display(),
        app_dir.display(),
    );

    fs::write(&service_path, &content)
        .map_err(|e| format!("Failed to write .service: {}", e))?;

    // chmod 0o644
    let metadata = fs::metadata(&service_path)
        .map_err(|e| format!("Failed to get .service metadata: {}", e))?;
    let mut perms = metadata.permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&service_path, perms)
        .map_err(|e| format!("Failed to set .service permissions: {}", e))?;

    Ok(service_path)
}

/// Enable and reload systemd user service. Failures are non-fatal (logged to stderr).
pub fn enable_service(app_name: &str) {
    let commands: &[&[&str]] = &[
        &["--user", "daemon-reload"],
        &["--user", "enable", &format!("{}.service", app_name)],
    ];

    for args in commands {
        match Command::new("systemctl").args(*args).output() {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("[!] systemctl {} failed (non-fatal): {}",
                        args.join(" "),
                        stderr.trim());
                }
            }
            Err(e) => {
                eprintln!("[!] systemctl command failed (non-fatal): {}", e);
                return;
            }
        }
    }
}