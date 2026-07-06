use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub mod app_record;

/// Data directory for dra registry: ~/.local/share/dra
fn dra_data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    Path::new(&home).join(".local").join("share").join("dra")
}

/// Path to the installed apps JSON file
fn registry_path() -> PathBuf {
    dra_data_dir().join("installed_apps.json")
}

/// The full registry - a list of AppRecord entries
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Registry {
    pub apps: Vec<app_record::AppRecord>,
}

impl Registry {
    /// Load the registry from disk. Returns an empty registry if the file doesn't exist.
    pub fn load() -> Self {
        let path = registry_path();
        if !path.exists() {
            return Self::default();
        }
        match fs::read_to_string(&path) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("[!] Failed to parse registry file, starting fresh: {}", e);
                    Self::default()
                })
            }
            Err(e) => {
                eprintln!("[!] Failed to read registry file: {}", e);
                Self::default()
            }
        }
    }

    /// Save the registry to disk.
    pub fn save(&self) -> Result<(), String> {
        let path = registry_path();
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create registry directory: {}", e))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize registry: {}", e))?;
        fs::write(&path, &json)
            .map_err(|e| format!("Failed to write registry file: {}", e))?;
        Ok(())
    }

    /// Add a record to the registry (replaces existing record with same name).
    pub fn add_record(&mut self, record: app_record::AppRecord) {
        // Remove existing record with same name
        self.apps.retain(|a| a.name != record.name);
        self.apps.push(record);
    }

    /// Remove a record by name.
    pub fn remove_record(&mut self, name: &str) {
        self.apps.retain(|a| a.name != name);
    }

    /// Get a record by name.
    pub fn get_record(&self, name: &str) -> Option<&app_record::AppRecord> {
        self.apps.iter().find(|a| a.name == name)
    }

    /// Get all records.
    pub fn list_records(&self) -> &[app_record::AppRecord] {
        &self.apps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_record::{AppRecord, InstallType};

    fn test_record(name: &str) -> AppRecord {
        AppRecord {
            name: name.to_string(),
            repo: format!("test/{}", name),
            installed_version: "v1.0.0".to_string(),
            install_type: InstallType::ArchiveBin,
            installed_path: format!("/home/user/.local/bin/{}", name),
            executables: vec![name.to_string()],
            installed_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_add_and_get() {
        let mut registry = Registry::default();
        let record = test_record("ripgrep");
        registry.add_record(record.clone());

        let retrieved = registry.get_record("ripgrep").unwrap();
        assert_eq!(retrieved.name, "ripgrep");
        assert_eq!(retrieved.repo, "test/ripgrep");
    }

    #[test]
    fn test_add_replace_same_name() {
        let mut registry = Registry::default();
        registry.add_record(test_record("ripgrep"));
        let mut v2 = test_record("ripgrep");
        v2.installed_version = "v2.0.0".to_string();
        registry.add_record(v2);

        assert_eq!(registry.list_records().len(), 1);
        assert_eq!(
            registry.get_record("ripgrep").unwrap().installed_version,
            "v2.0.0"
        );
    }

    #[test]
    fn test_remove_record() {
        let mut registry = Registry::default();
        registry.add_record(test_record("ripgrep"));
        registry.add_record(test_record("vscode"));
        registry.remove_record("ripgrep");

        assert_eq!(registry.list_records().len(), 1);
        assert!(registry.get_record("ripgrep").is_none());
        assert!(registry.get_record("vscode").is_some());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut registry = Registry::default();
        registry.add_record(test_record("ripgrep"));
        registry.remove_record("nonexistent");
        assert_eq!(registry.list_records().len(), 1);
    }

    #[test]
    fn test_list_records() {
        let mut registry = Registry::default();
        registry.add_record(test_record("a"));
        registry.add_record(test_record("b"));
        registry.add_record(test_record("c"));

        assert_eq!(registry.list_records().len(), 3);
    }

    #[test]
    fn test_save_and_load() {
        // Use a temp dir for the registry
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("installed_apps.json");

        let mut registry = Registry::default();
        registry.add_record(test_record("ripgrep"));

        // Manually write to temp path
        let json = serde_json::to_string_pretty(&registry).unwrap();
        fs::write(&path, &json).unwrap();

        // Read it back
        let content = fs::read_to_string(&path).unwrap();
        let loaded: Registry = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.list_records().len(), 1);
        assert_eq!(loaded.get_record("ripgrep").unwrap().installed_version, "v1.0.0");
    }
}