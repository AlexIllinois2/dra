use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Generate a .desktop file in the app directory.
pub fn generate_desktop(
    app_dir: &Path,
    app_name: &str,
    bin_rel_path: &str,
    icon_rel_path: Option<&str>,
    cicon_path: Option<&Path>,
) -> Result<PathBuf, String> {
    let desktop_path = app_dir.join(format!("{}.desktop", app_name));
    let exec_path = app_dir.join(bin_rel_path);

    let icon_path: Option<PathBuf> = if let Some(cicon) = cicon_path {
        let icons_dir = app_dir.join("share").join("icons");
        fs::create_dir_all(&icons_dir)
            .map_err(|e| format!("Failed to create icons dir: {}", e))?;
        let ext = cicon
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let icon_name = format!("{}.{}", app_name, ext);
        let icon_target = icons_dir.join(&icon_name);
        fs::copy(cicon, &icon_target)
            .map_err(|e| format!("Failed to copy icon: {}", e))?;
        Some(icon_target)
    } else {
        icon_rel_path.map(|rel| app_dir.join(rel))
    };

    let mut lines = vec![
        "[Desktop Entry]".to_string(),
        "Type=Application".to_string(),
        format!("Name={}", app_name),
        format!("Exec={}", exec_path.display()),
        format!("Path={}", app_dir.display()),
        "Terminal=false".to_string(),
    ];

    if let Some(ref ip) = icon_path {
        if ip.exists() {
            lines.push(format!("Icon={}", ip.display()));
        }
    }

    let content = lines.join("\n") + "\n";
    fs::write(&desktop_path, &content)
        .map_err(|e| format!("Failed to write .desktop: {}", e))?;

    // chmod 0o755
    let metadata = fs::metadata(&desktop_path)
        .map_err(|e| format!("Failed to get .desktop metadata: {}", e))?;
    let mut perms = metadata.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&desktop_path, perms)
        .map_err(|e| format!("Failed to set .desktop permissions: {}", e))?;

    Ok(desktop_path)
}