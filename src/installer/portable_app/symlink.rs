use std::os::unix::fs;
use std::path::Path;

/// Create a symlink `target -> source`, handling conflicts.
/// Returns `true` on success, `false` on conflict.
pub fn create_symlink(source: &Path, target: &Path, description: &str) -> bool {
    if target.is_symlink() || target.exists() {
        let resolved = target
            .canonicalize()
            .unwrap_or_else(|_| target.to_path_buf());
        if resolved == source.canonicalize().unwrap_or_else(|_| source.to_path_buf()) {
            eprintln!("[!] {} already points to the correct target, skipping", description);
            return true;
        } else {
            eprintln!("[-] Conflict: {} already exists and points to {}", description, resolved.display());
            eprintln!("    expected: {}", source.display());
            return false;
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    fs::symlink(source, target).map_err(|e| {
        eprintln!("[-] Failed to create symlink {} -> {}: {}", target.display(), source.display(), e);
    }).is_ok()
}