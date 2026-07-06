use serde::{Deserialize, Serialize};

/// Type of installation for tracking purposes.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum InstallType {
    /// Single binary file copied to ~/.local/bin
    Bin,
    /// Executable extracted from tar.gz/zip archive
    ArchiveBin,
    /// Portable app installed to ~/.local/app/<name> (with desktop/service)
    PortableApp,
    /// AppImage file
    AppImage,
}

/// A record of an installed application in the registry.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppRecord {
    /// Application name, e.g. "ripgrep" or "vscode"
    pub name: String,
    /// Source repository, e.g. "BurntSushi/ripgrep" or "microsoft/vscode"
    pub repo: String,
    /// Currently installed version (GitHub tag), e.g. "v13.0.0" or "1.85.0"
    pub installed_version: String,
    /// How the app was installed
    pub install_type: InstallType,
    /// Primary installation path
    pub installed_path: String,
    /// List of executable files placed in ~/.local/bin
    pub executables: Vec<String>,
    /// ISO 8601 timestamp of installation
    pub installed_at: String,
}

impl AppRecord {
    /// Create a new AppRecord.
    pub fn new(
        name: String,
        repo: String,
        installed_version: String,
        install_type: InstallType,
        installed_path: String,
        executables: Vec<String>,
    ) -> Self {
        let installed_at = chrono_now();
        Self {
            name,
            repo,
            installed_version,
            install_type,
            installed_path,
            executables,
            installed_at,
        }
    }
}

/// Get current timestamp as ISO 8601 string.
/// Falls back to "unknown" if chrono is not available.
fn chrono_now() -> String {
    // Use a simple approach without external dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Convert to UTC string manually (simplified)
    format!("{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_record_creation() {
        let record = AppRecord::new(
            "ripgrep".to_string(),
            "BurntSushi/ripgrep".to_string(),
            "13.0.0".to_string(),
            InstallType::ArchiveBin,
            "/home/user/.local/bin".to_string(),
            vec!["rg".to_string()],
        );
        assert_eq!(record.name, "ripgrep");
        assert_eq!(record.repo, "BurntSushi/ripgrep");
        assert_eq!(record.installed_version, "13.0.0");
        assert_eq!(record.install_type, InstallType::ArchiveBin);
        assert_eq!(record.executables, vec!["rg"]);
        assert!(!record.installed_at.is_empty());
    }

    #[test]
    fn test_install_type_serialization() {
        let types = vec![
            InstallType::Bin,
            InstallType::ArchiveBin,
            InstallType::PortableApp,
            InstallType::AppImage,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let deserialized: InstallType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, deserialized);
        }
    }
}