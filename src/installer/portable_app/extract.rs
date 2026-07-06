use std::fs;
use std::path::Path;

use flate2::read::GzDecoder;
use tar;

/// Compute the common top-level directory prefix for all entries in a tar archive.
/// If all files share the same top-level directory, return its name.
/// Otherwise return None.
fn common_top_level<R: std::io::Read>(archive: &mut tar::Archive<R>) -> Result<Option<String>, String> {
    let entries = archive.entries().map_err(|e| format!("Error reading archive: {}", e))?;
    let mut prefixes: Option<String> = None;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Error reading archive entry: {}", e))?;
        let path = entry.path().map_err(|e| format!("Error reading entry path: {}", e))?.into_owned();

        // Get the first component
        let first = path
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string());

        match (first, &prefixes) {
            (Some(p), None) => prefixes = Some(p),
            (Some(p), Some(expected)) if &p == expected => {}
            (Some(_), Some(_)) => return Ok(None), // different prefixes
            (None, _) => return Ok(None),          // empty path
        }
    }

    Ok(prefixes)
}

/// Extract a tar.gz archive to `target_dir`, stripping the common top-level directory
/// if all files share one (e.g. `VSCode-linux-x64/`).
///
/// Strategy: extract to a temp dir first, then move files stripping the prefix.
pub fn extract_strip_prefix(pkg_path: &Path, target_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(pkg_path)
        .map_err(|e| format!("Error opening {}: {}", pkg_path.display(), e))?;

    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    let prefix = common_top_level(&mut archive)?;

    // Re-open because we consumed the archive in common_top_level
    let file = fs::File::open(pkg_path)
        .map_err(|e| format!("Error opening {}: {}", pkg_path.display(), e))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    if let Some(ref prefix) = prefix {
        // Extract to a temporary directory first
        let temp_dir = tempfile::TempDir::new()
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;
        archive
            .unpack(temp_dir.path())
            .map_err(|e| format!("Error extracting archive: {}", e))?;

        // Move files from temp_dir/prefix/ to target_dir/
        let source_prefix = temp_dir.path().join(prefix);
        if source_prefix.exists() {
            copy_dir_all(&source_prefix, target_dir)?;
        } else {
            // Fallback: just copy everything from temp_dir
            for entry in fs::read_dir(temp_dir.path())
                .map_err(|e| format!("Error reading temp dir: {}", e))?
            {
                let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
                let target = target_dir.join(entry.file_name());
                if entry.file_type().map_err(|e| format!("Error reading file type: {}", e))?.is_dir() {
                    copy_dir_all(&entry.path(), &target)?;
                } else {
                    fs::copy(&entry.path(), &target)
                        .map_err(|e| format!("Error copying {}: {}", entry.path().display(), e))?;
                }
            }
        }
    } else {
        // No common prefix, extract as-is
        archive
            .unpack(target_dir)
            .map_err(|e| format!("Error extracting archive: {}", e))?;
    }

    Ok(())
}

/// Recursively copy a directory from source to destination.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory {}: {}", dst.display(), e))?;

    for entry in fs::read_dir(src).map_err(|e| format!("Error reading directory {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
        let file_type = entry.file_type().map_err(|e| format!("Error reading file type: {}", e))?;
        let target = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(&entry.path(), &target)
                .map_err(|e| format!("Error copying {} -> {}: {}", entry.path().display(), target.display(), e))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a simple tar.gz with a common prefix
    fn create_tar_gz_with_prefix(dir: &Path, prefix: &str) -> std::path::PathBuf {
        let pkg_path = dir.join("test.tar.gz");
        let file = fs::File::create(&pkg_path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut tar_builder = tar::Builder::new(encoder);

        // Add files with prefix
        let content = b"hello";
        let mut header = tar::Header::new_gnu();
        header.set_path(format!("{}/file1.txt", prefix)).unwrap();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar_builder.append(&header, &content[..]).unwrap();

        let mut header = tar::Header::new_gnu();
        header.set_path(format!("{}/sub/file2.txt", prefix)).unwrap();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar_builder.append(&header, &content[..]).unwrap();

        tar_builder.finish().unwrap();
        pkg_path
    }

    #[test]
    fn test_extract_strip_prefix() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = tempfile::TempDir::new().unwrap();

        let pkg = create_tar_gz_with_prefix(dir.path(), "MyApp-1.0");
        extract_strip_prefix(&pkg, target.path()).unwrap();

        // After stripping, files should be directly in target
        assert!(target.path().join("file1.txt").exists());
        assert!(target.path().join("sub/file2.txt").exists());
        // The prefix directory should NOT exist
        assert!(!target.path().join("MyApp-1.0").exists());
    }

    #[test]
    fn test_extract_no_prefix() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = tempfile::TempDir::new().unwrap();

        // Create a tar.gz with files at root level (no common prefix)
        let pkg_path = dir.path().join("test.tar.gz");
        let file = fs::File::create(&pkg_path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut tar_builder = tar::Builder::new(encoder);

        let content = b"hello";
        let mut header = tar::Header::new_gnu();
        header.set_path("file1.txt").unwrap();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar_builder.append(&header, &content[..]).unwrap();

        let mut header = tar::Header::new_gnu();
        header.set_path("sub/file2.txt").unwrap();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar_builder.append(&header, &content[..]).unwrap();

        let _encoder = tar_builder.into_inner().unwrap().finish().unwrap();

        extract_strip_prefix(&pkg_path, target.path()).unwrap();

        assert!(target.path().join("file1.txt").exists());
        assert!(target.path().join("sub/file2.txt").exists());
    }
}