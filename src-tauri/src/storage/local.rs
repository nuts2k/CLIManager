use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::atomic_write;
use crate::error::AppError;

/// 代理开关设置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProxySettings {
    /// 全局总开关
    #[serde(default)]
    pub global_enabled: bool,
    /// 每 CLI 独立开关状态 {"claude": true, "codex": false}
    #[serde(default)]
    pub cli_enabled: HashMap<String, bool>,
}

impl Default for ProxySettings {
    fn default() -> Self {
        Self {
            global_enabled: false,
            cli_enabled: HashMap::new(),
        }
    }
}

/// 代理接管标志（崩溃恢复用）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProxyTakeover {
    /// 当前被接管的 CLI IDs
    #[serde(default)]
    pub cli_ids: Vec<String>,
}

impl Default for ProxyTakeover {
    fn default() -> Self {
        Self {
            cli_ids: Vec::new(),
        }
    }
}

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
pub struct TestConfig {
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_model: Option<String>,
}

fn default_timeout_secs() -> u32 {
    10
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 10,
            test_model: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalSettings {
    #[serde(default, skip_serializing)]
    pub active_provider_id: Option<String>,
    #[serde(default)]
    pub active_providers: HashMap<String, Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icloud_dir_override: Option<String>,
    #[serde(default)]
    pub cli_paths: CliPaths,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_config: Option<TestConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxySettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_takeover: Option<ProxyTakeover>,
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
            active_providers: HashMap::new(),
            icloud_dir_override: None,
            cli_paths: CliPaths::default(),
            language: None,
            test_config: None,
            proxy: None,
            proxy_takeover: None,
            schema_version: 1,
        }
    }
}

impl LocalSettings {
    fn migrate_legacy_active_provider(&mut self) {
        if self.active_providers.contains_key("claude") {
            return;
        }

        if let Some(active_provider_id) = self.active_provider_id.clone() {
            self.active_providers
                .insert("claude".to_string(), Some(active_provider_id));
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
    let mut settings: LocalSettings = serde_json::from_str(&content)?;
    settings.migrate_legacy_active_provider();
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
        assert!(settings.active_providers.is_empty());
        assert_eq!(settings.icloud_dir_override, None);
        assert_eq!(settings.cli_paths.claude_config_dir, None);
        assert_eq!(settings.cli_paths.codex_config_dir, None);
        assert_eq!(settings.language, None);
        assert_eq!(settings.test_config, None);
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
        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("provider-123".to_string()));
        let settings = LocalSettings {
            active_providers,
            icloud_dir_override: Some("/custom/icloud".to_string()),
            cli_paths: CliPaths {
                claude_config_dir: Some("/home/user/.claude".to_string()),
                codex_config_dir: Some("/home/user/.codex".to_string()),
            },
            ..LocalSettings::default()
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

        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("test-provider".to_string()));
        let settings = LocalSettings {
            active_providers,
            cli_paths: CliPaths {
                claude_config_dir: Some("/test/.claude".to_string()),
                codex_config_dir: None,
            },
            ..LocalSettings::default()
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
        assert!(settings.active_providers.is_empty());

        // Set active provider per CLI
        settings.active_providers.insert(
            "claude".to_string(),
            Some("new-active-provider".to_string()),
        );
        write_local_settings_to(&path, &settings).unwrap();

        // Read back and verify
        let loaded = read_local_settings_from(&path).unwrap();
        assert_eq!(
            loaded.active_providers.get("claude"),
            Some(&Some("new-active-provider".to_string()))
        );
    }

    #[test]
    fn test_local_settings_path_is_in_cli_manager_dir() {
        let path = get_local_settings_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains(".cli-manager"),
            "Path should contain .cli-manager: {}",
            path_str
        );
        assert!(
            path_str.ends_with("local.json"),
            "Path should end with local.json: {}",
            path_str
        );
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
        assert!(
            !path_str.contains("Mobile Documents"),
            "Local path must not reference iCloud: {}",
            path_str
        );
        assert!(
            !path_str.contains("CloudDocs"),
            "Local path must not reference CloudDocs: {}",
            path_str
        );
    }

    #[test]
    fn test_active_providers_hashmap_round_trip() {
        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("provider-1".to_string()));
        active_providers.insert("codex".to_string(), Some("provider-2".to_string()));

        let settings = LocalSettings {
            active_providers,
            ..LocalSettings::default()
        };

        let json = serde_json::to_string_pretty(&settings).unwrap();
        assert!(json.contains("active_providers"));
        let deserialized: LocalSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.active_providers.get("claude"),
            Some(&Some("provider-1".to_string()))
        );
        assert_eq!(
            deserialized.active_providers.get("codex"),
            Some(&Some("provider-2".to_string()))
        );
    }

    #[test]
    fn test_old_active_provider_id_field_still_deserializes() {
        // JSON with old active_provider_id field should still deserialize (backward compat)
        let json = r#"{"active_provider_id": "old-provider-123", "cli_paths": {}}"#;
        let settings: LocalSettings = serde_json::from_str(json).unwrap();
        assert_eq!(
            settings.active_provider_id,
            Some("old-provider-123".to_string())
        );
        // But when we serialize, active_provider_id should NOT appear (skip_serializing)
        let re_serialized = serde_json::to_string_pretty(&settings).unwrap();
        assert!(!re_serialized.contains("active_provider_id"));
    }

    #[test]
    fn test_read_local_settings_migrates_legacy_active_provider_id() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("local.json");
        fs::write(
            &path,
            r#"{"active_provider_id":"old-provider-123","cli_paths":{}}"#,
        )
        .unwrap();

        let settings = read_local_settings_from(&path).unwrap();
        assert_eq!(
            settings.active_providers.get("claude"),
            Some(&Some("old-provider-123".to_string()))
        );

        write_local_settings_to(&path, &settings).unwrap();
        let written = fs::read_to_string(&path).unwrap();
        assert!(written.contains("active_providers"));
        assert!(!written.contains("active_provider_id"));
    }

    #[test]
    fn test_test_config_serializes_with_defaults() {
        let config = TestConfig::default();
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.test_model, None);
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: TestConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.timeout_secs, 10);
    }

    #[test]
    fn test_language_field_persists() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("local.json");

        let settings = LocalSettings {
            language: Some("zh-CN".to_string()),
            ..LocalSettings::default()
        };

        write_local_settings_to(&path, &settings).unwrap();
        let loaded = read_local_settings_from(&path).unwrap();
        assert_eq!(loaded.language, Some("zh-CN".to_string()));
    }

    #[test]
    fn test_proxy_settings_default() {
        let ps = ProxySettings::default();
        assert!(!ps.global_enabled);
        assert!(ps.cli_enabled.is_empty());
    }

    #[test]
    fn test_proxy_takeover_default() {
        let pt = ProxyTakeover::default();
        assert!(pt.cli_ids.is_empty());
    }

    #[test]
    fn test_local_settings_with_proxy_round_trip() {
        let mut cli_enabled = HashMap::new();
        cli_enabled.insert("claude".to_string(), true);
        cli_enabled.insert("codex".to_string(), false);

        let settings = LocalSettings {
            proxy: Some(ProxySettings {
                global_enabled: true,
                cli_enabled,
            }),
            proxy_takeover: Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()],
            }),
            ..LocalSettings::default()
        };

        let json = serde_json::to_string_pretty(&settings).unwrap();
        assert!(json.contains("proxy"));
        assert!(json.contains("proxy_takeover"));
        assert!(json.contains("global_enabled"));
        assert!(json.contains("cli_ids"));

        let deserialized: LocalSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(settings, deserialized);
    }

    #[test]
    fn test_local_settings_backward_compat_without_proxy() {
        // 旧 JSON 不含 proxy 和 proxy_takeover 字段，应正常反序列化为 None
        let json = r#"{"cli_paths": {}, "active_providers": {}}"#;
        let settings: LocalSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.proxy, None);
        assert_eq!(settings.proxy_takeover, None);
    }

    #[test]
    fn test_local_settings_with_proxy_file_round_trip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("local.json");

        let mut cli_enabled = HashMap::new();
        cli_enabled.insert("claude".to_string(), true);

        let settings = LocalSettings {
            proxy: Some(ProxySettings {
                global_enabled: true,
                cli_enabled,
            }),
            proxy_takeover: Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()],
            }),
            ..LocalSettings::default()
        };

        write_local_settings_to(&path, &settings).unwrap();
        let loaded = read_local_settings_from(&path).unwrap();
        assert_eq!(settings, loaded);
    }
}
