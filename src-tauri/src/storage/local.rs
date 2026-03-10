use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use super::atomic_write;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CliPaths {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_config_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex_config_dir: Option<String>,
}

impl Default for CliPaths {
    fn default() -> Self {
        Self {
            claude_config_dir: None,
            codex_config_dir: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icloud_dir_override: Option<String>,
    #[serde(default)]
    pub cli_paths: CliPaths,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_schema_version() -> u32 {
    1
}

impl Default for LocalSettings {
    fn default() -> Self {
        Self {
            active_provider_id: None,
            icloud_dir_override: None,
            cli_paths: CliPaths::default(),
            schema_version: 1,
        }
    }
}

/// Get the path to the local settings file: ~/.cli-manager/local.json
pub fn get_local_settings_path() -> PathBuf {
    let home = dirs::home_dir().expect("Could not determine home directory");
    home.join(".cli-manager").join("local.json")
}

/// Read local settings from the default path.
/// Returns Default if file does not exist.
pub fn read_local_settings() -> Result<LocalSettings, AppError> {
    let path = get_local_settings_path();
    read_local_settings_from(&path)
}

/// Read local settings from a specific path (for testing).
/// Returns Default if file does not exist.
pub fn read_local_settings_from(path: &Path) -> Result<LocalSettings, AppError> {
    if !path.exists() {
        return Ok(LocalSettings::default());
    }
    let content = fs::read_to_string(path).map_err(|e| AppError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let settings: LocalSettings = serde_json::from_str(&content)?;
    Ok(settings)
}

/// Write local settings to the default path.
pub fn write_local_settings(settings: &LocalSettings) -> Result<(), AppError> {
    let path = get_local_settings_path();
    write_local_settings_to(&path, settings)
}

/// Write local settings to a specific path (for testing).
pub fn write_local_settings_to(path: &Path, settings: &LocalSettings) -> Result<(), AppError> {
    let json = serde_json::to_string_pretty(settings)?;
    atomic_write(path, json.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_local_settings() {
        let settings = LocalSettings::default();
        assert_eq!(settings.active_provider_id, None);
        assert_eq!(settings.icloud_dir_override, None);
        assert_eq!(settings.cli_paths.claude_config_dir, None);
        assert_eq!(settings.cli_paths.codex_config_dir, None);
        assert_eq!(settings.schema_version, 1);
    }

    #[test]
    fn test_default_cli_paths() {
        let paths = CliPaths::default();
        assert_eq!(paths.claude_config_dir, None);
        assert_eq!(paths.codex_config_dir, None);
    }

    #[test]
    fn test_round_trip_serialization() {
        let settings = LocalSettings {
            active_provider_id: Some("provider-123".to_string()),
            icloud_dir_override: Some("/custom/icloud".to_string()),
            cli_paths: CliPaths {
                claude_config_dir: Some("/home/user/.claude".to_string()),
                codex_config_dir: Some("/home/user/.codex".to_string()),
            },
            schema_version: 1,
        };
        let json = serde_json::to_string_pretty(&settings).unwrap();
        let deserialized: LocalSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(settings, deserialized);
    }

    #[test]
    fn test_read_missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.json");
        let settings = read_local_settings_from(&path).unwrap();
        assert_eq!(settings, LocalSettings::default());
    }

    #[test]
    fn test_write_then_read_round_trip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("local.json");

        let settings = LocalSettings {
            active_provider_id: Some("test-provider".to_string()),
            icloud_dir_override: None,
            cli_paths: CliPaths {
                claude_config_dir: Some("/test/.claude".to_string()),
                codex_config_dir: None,
            },
            schema_version: 1,
        };

        write_local_settings_to(&path, &settings).unwrap();
        let loaded = read_local_settings_from(&path).unwrap();
        assert_eq!(settings, loaded);
    }

    #[test]
    fn test_set_active_provider_persists() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("local.json");

        // Start with defaults
        let mut settings = read_local_settings_from(&path).unwrap();
        assert_eq!(settings.active_provider_id, None);

        // Set active provider
        settings.active_provider_id = Some("new-active-provider".to_string());
        write_local_settings_to(&path, &settings).unwrap();

        // Read back and verify
        let loaded = read_local_settings_from(&path).unwrap();
        assert_eq!(loaded.active_provider_id, Some("new-active-provider".to_string()));
    }

    #[test]
    fn test_local_settings_path_is_in_cli_manager_dir() {
        let path = get_local_settings_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains(".cli-manager"), "Path should contain .cli-manager: {}", path_str);
        assert!(path_str.ends_with("local.json"), "Path should end with local.json: {}", path_str);
    }

    #[test]
    fn test_directory_creation_on_write() {
        let tmp = TempDir::new().unwrap();
        let nested_path = tmp.path().join("nested").join("dir").join("local.json");
        assert!(!nested_path.parent().unwrap().exists());

        let settings = LocalSettings::default();
        write_local_settings_to(&nested_path, &settings).unwrap();

        assert!(nested_path.exists(), "File should exist after write");
    }

    #[test]
    fn test_schema_version_defaults_from_json() {
        // JSON without schema_version should default to 1
        let json = r#"{"cli_paths": {}}"#;
        let settings: LocalSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.schema_version, 1);
    }

    #[test]
    fn test_isolation_from_icloud() {
        // Verify that local.rs path does NOT contain iCloud-related directories
        let path = get_local_settings_path();
        let path_str = path.to_string_lossy();
        assert!(!path_str.contains("Mobile Documents"), "Local path must not reference iCloud: {}", path_str);
        assert!(!path_str.contains("CloudDocs"), "Local path must not reference CloudDocs: {}", path_str);
    }
}
