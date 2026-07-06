use std::path::PathBuf;
use std::env;

pub fn home_dir() -> Result<PathBuf, String> {
    env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME environment variable not set".to_string())
}

/// Check if a directory is in the user's PATH.
pub fn is_in_path(dir: &PathBuf) -> bool {
    env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|p| PathBuf::from(p) == *dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_dir() {
        let result = home_dir();
        assert!(result.is_ok());
        let home = result.unwrap();
        assert!(home.is_absolute());
    }

    #[test]
    fn test_is_in_path_self() {
        let home = home_dir().unwrap();
        // ~/.local/bin is commonly in PATH
        let local_bin = home.join(".local").join("bin");
        // Don't assert on result since it depends on user's setup
        let _ = is_in_path(&local_bin);
    }
}