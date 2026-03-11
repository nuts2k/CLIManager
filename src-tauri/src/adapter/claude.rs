use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::AppError;
use crate::provider::Provider;
use crate::storage::atomic_write;

use super::{create_backup, rotate_backups, CliAdapter, PatchResult};

const MAX_BACKUPS: usize = 5;

/// Adapter for Claude Code CLI (`~/.claude/settings.json`).
pub struct ClaudeAdapter {
    config_dir: PathBuf,
    backup_dir: PathBuf,
}

impl ClaudeAdapter {
    /// Create adapter using default directories.
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("home directory required");
        Self {
            config_dir: home.join(".claude"),
            backup_dir: home.join(".cli-manager").join("backups").join("claude"),
        }
    }

    /// Create adapter with explicit paths (for testing).
    pub fn new_with_paths(config_dir: PathBuf, backup_dir: PathBuf) -> Self {
        Self {
            config_dir,
            backup_dir,
        }
    }
}

impl CliAdapter for ClaudeAdapter {
    fn cli_name(&self) -> &str {
        "claude"
    }

    fn patch(&self, provider: &Provider) -> Result<PatchResult, AppError> {
        let settings_path = self.config_dir.join("settings.json");
        let mut backups_created = Vec::new();

        // Read existing or use empty object
        let existing = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path).map_err(|e| AppError::Io {
                path: settings_path.display().to_string(),
                source: e,
            })?;
            // Pre-validation: must be valid JSON
            serde_json::from_str::<Value>(&content).map_err(|_| {
                AppError::Validation(format!(
                    "existing {} is not valid JSON",
                    settings_path.display()
                ))
            })?;
            content
        } else {
            "{}".to_string()
        };

        // Backup (only if file exists)
        if settings_path.exists() {
            let backup_path = create_backup(&settings_path, &self.backup_dir)?;
            rotate_backups(&self.backup_dir, MAX_BACKUPS)?;
            backups_created.push(backup_path);
        }

        // Surgical patch
        let patched = patch_claude_json(&existing, &provider.api_key, &provider.base_url)?;

        // Post-validation: patched result must be valid JSON
        serde_json::from_str::<Value>(&patched)
            .map_err(|_| AppError::Validation("patched JSON is not valid".to_string()))?;

        // Ensure config dir exists for new file creation
        fs::create_dir_all(&self.config_dir).map_err(|e| AppError::Io {
            path: self.config_dir.display().to_string(),
            source: e,
        })?;

        // Atomic write
        atomic_write(&settings_path, patched.as_bytes())?;

        Ok(PatchResult {
            files_written: vec![settings_path.display().to_string()],
            backups_created,
        })
    }

    fn clear(&self) -> Result<PatchResult, AppError> {
        let settings_path = self.config_dir.join("settings.json");

        if !settings_path.exists() {
            return Ok(PatchResult {
                files_written: vec![],
                backups_created: vec![],
            });
        }

        let content = fs::read_to_string(&settings_path).map_err(|e| AppError::Io {
            path: settings_path.display().to_string(),
            source: e,
        })?;
        serde_json::from_str::<Value>(&content).map_err(|_| {
            AppError::Validation(format!(
                "existing {} is not valid JSON",
                settings_path.display()
            ))
        })?;

        let backup_path = create_backup(&settings_path, &self.backup_dir)?;
        rotate_backups(&self.backup_dir, MAX_BACKUPS)?;

        let cleared = clear_claude_json(&content)?;
        serde_json::from_str::<Value>(&cleared)
            .map_err(|_| AppError::Validation("cleared JSON is not valid".to_string()))?;

        atomic_write(&settings_path, cleared.as_bytes())?;

        Ok(PatchResult {
            files_written: vec![settings_path.display().to_string()],
            backups_created: vec![backup_path],
        })
    }
}

/// Surgically patch Claude Code settings JSON.
/// Only modifies `env.ANTHROPIC_AUTH_TOKEN` and `env.ANTHROPIC_BASE_URL`.
/// All other keys, nesting, and ordering survive intact.
fn patch_claude_json(existing: &str, api_key: &str, base_url: &str) -> Result<String, AppError> {
    let mut root: Value = serde_json::from_str(existing)
        .map_err(|_| AppError::Validation("failed to parse settings JSON".to_string()))?;

    let root_obj = root.as_object_mut().ok_or_else(|| {
        AppError::Validation("settings.json root is not a JSON object".to_string())
    })?;

    // Ensure "env" key exists as an object
    let env = root_obj
        .entry("env")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));

    let env_obj = env.as_object_mut().ok_or_else(|| {
        AppError::Validation("settings.json env field is not an object".to_string())
    })?;

    env_obj.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        Value::String(api_key.to_string()),
    );
    env_obj.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        Value::String(base_url.to_string()),
    );

    serde_json::to_string_pretty(&root).map_err(|e| AppError::Json(e))
}

fn clear_claude_json(existing: &str) -> Result<String, AppError> {
    let mut root: Value = serde_json::from_str(existing)
        .map_err(|_| AppError::Validation("failed to parse settings JSON".to_string()))?;

    let root_obj = root.as_object_mut().ok_or_else(|| {
        AppError::Validation("settings.json root is not a JSON object".to_string())
    })?;

    let remove_env = if let Some(env) = root_obj.get_mut("env") {
        let env_obj = env.as_object_mut().ok_or_else(|| {
            AppError::Validation("settings.json env field is not an object".to_string())
        })?;

        env_obj.remove("ANTHROPIC_AUTH_TOKEN");
        env_obj.remove("ANTHROPIC_BASE_URL");
        env_obj.is_empty()
    } else {
        false
    };

    if remove_env {
        root_obj.remove("env");
    }

    serde_json::to_string_pretty(&root).map_err(|e| AppError::Json(e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ProtocolType, Provider};
    use tempfile::TempDir;

    fn test_provider() -> Provider {
        Provider {
            id: "test-id".to_string(),
            cli_id: "claude".to_string(),
            name: "Test Provider".to_string(),
            protocol_type: ProtocolType::Anthropic,
            api_key: "sk-ant-new-key-123".to_string(),
            base_url: "https://proxy.example.com".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            model_config: None,
            notes: None,
            created_at: 1710000000000,
            updated_at: 1710000000000,
            schema_version: 1,
        }
    }

    #[test]
    fn test_patch_existing_only_modifies_env_fields() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        // Write settings with extra keys
        let original = r#"{
  "permissions": {
    "allow": ["Bash", "Read"]
  },
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "old-key",
    "ANTHROPIC_BASE_URL": "https://old.example.com",
    "CUSTOM_VAR": "keep-this"
  },
  "other_setting": true
}"#;
        fs::write(config_dir.join("settings.json"), original).unwrap();

        let adapter = ClaudeAdapter::new_with_paths(config_dir.clone(), backup_dir);
        let result = adapter.patch(&test_provider()).unwrap();

        assert_eq!(result.files_written.len(), 1);

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        // Target fields updated
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-new-key-123");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://proxy.example.com"
        );

        // Other fields survive
        assert_eq!(patched["env"]["CUSTOM_VAR"], "keep-this");
        assert_eq!(patched["other_setting"], true);
        assert!(patched["permissions"]["allow"].is_array());
    }

    #[test]
    fn test_patch_preserves_key_ordering() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"{
  "zzz_last": 1,
  "aaa_first": 2,
  "mmm_middle": 3
}"#;
        fs::write(config_dir.join("settings.json"), original).unwrap();

        let adapter = ClaudeAdapter::new_with_paths(config_dir.clone(), backup_dir);
        adapter.patch(&test_provider()).unwrap();

        let content = fs::read_to_string(config_dir.join("settings.json")).unwrap();
        let zzz_pos = content.find("zzz_last").unwrap();
        let aaa_pos = content.find("aaa_first").unwrap();
        let mmm_pos = content.find("mmm_middle").unwrap();

        // Original insertion order preserved (not alphabetized)
        assert!(zzz_pos < aaa_pos);
        assert!(aaa_pos < mmm_pos);
    }

    #[test]
    fn test_patch_preserves_nested_objects_and_arrays() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"{
  "permissions": {
    "allow": ["Bash", "Read", "Write"],
    "deny": ["Network"]
  },
  "nested": {
    "deep": {
      "value": [1, 2, 3]
    }
  }
}"#;
        fs::write(config_dir.join("settings.json"), original).unwrap();

        let adapter = ClaudeAdapter::new_with_paths(config_dir.clone(), backup_dir);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(
            patched["permissions"]["allow"],
            serde_json::json!(["Bash", "Read", "Write"])
        );
        assert_eq!(
            patched["permissions"]["deny"],
            serde_json::json!(["Network"])
        );
        assert_eq!(
            patched["nested"]["deep"]["value"],
            serde_json::json!([1, 2, 3])
        );
    }

    #[test]
    fn test_patch_creates_env_object_if_missing() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"{"permissions": {"allow": ["Bash"]}}"#;
        fs::write(config_dir.join("settings.json"), original).unwrap();

        let adapter = ClaudeAdapter::new_with_paths(config_dir.clone(), backup_dir);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        assert!(patched["env"].is_object());
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-new-key-123");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://proxy.example.com"
        );
        // Original keys survive
        assert!(patched["permissions"]["allow"].is_array());
    }

    #[test]
    fn test_patch_creates_new_settings_when_file_missing() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        // Don't create config_dir -- adapter should create it

        let adapter = ClaudeAdapter::new_with_paths(config_dir.clone(), backup_dir.clone());
        let result = adapter.patch(&test_provider()).unwrap();

        assert_eq!(result.files_written.len(), 1);
        assert!(result.backups_created.is_empty()); // no backup when file didn't exist

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-new-key-123");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://proxy.example.com"
        );
    }

    #[test]
    fn test_patch_creates_backup_when_file_exists() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("settings.json"), "{}").unwrap();

        let adapter = ClaudeAdapter::new_with_paths(config_dir, backup_dir.clone());
        let result = adapter.patch(&test_provider()).unwrap();

        assert_eq!(result.backups_created.len(), 1);
        assert!(result.backups_created[0].ends_with(".bak"));

        // Backup file actually exists
        assert!(std::path::Path::new(&result.backups_created[0]).exists());
    }

    #[test]
    fn test_patch_no_backup_when_file_missing() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");

        let adapter = ClaudeAdapter::new_with_paths(config_dir, backup_dir);
        let result = adapter.patch(&test_provider()).unwrap();

        assert!(result.backups_created.is_empty());
    }

    #[test]
    fn test_patch_fails_on_unparseable_json() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("settings.json"), "{ invalid json }").unwrap();

        let adapter = ClaudeAdapter::new_with_paths(config_dir, backup_dir);
        let result = adapter.patch(&test_provider());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Validation"),
            "Expected Validation error, got: {}",
            err
        );
    }

    #[test]
    fn test_patch_result_contains_correct_paths() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("settings.json"), "{}").unwrap();

        let adapter = ClaudeAdapter::new_with_paths(config_dir.clone(), backup_dir);
        let result = adapter.patch(&test_provider()).unwrap();

        assert_eq!(result.files_written.len(), 1);
        assert!(result.files_written[0].contains("settings.json"));

        assert_eq!(result.backups_created.len(), 1);
        assert!(result.backups_created[0].contains("settings.json"));
        assert!(result.backups_created[0].ends_with(".bak"));
    }

    #[test]
    fn test_clear_removes_managed_env_fields_only() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(
            config_dir.join("settings.json"),
            r#"{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "old-key",
    "ANTHROPIC_BASE_URL": "https://old.example.com",
    "CUSTOM_VAR": "keep-me"
  },
  "other_setting": true
}"#,
        )
        .unwrap();

        let adapter = ClaudeAdapter::new_with_paths(config_dir.clone(), backup_dir.clone());
        let result = adapter.clear().unwrap();

        assert_eq!(result.files_written.len(), 1);
        assert_eq!(result.backups_created.len(), 1);
        assert!(std::path::Path::new(&result.backups_created[0]).exists());

        let cleared: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();
        assert!(cleared["env"]["ANTHROPIC_AUTH_TOKEN"].is_null());
        assert!(cleared["env"]["ANTHROPIC_BASE_URL"].is_null());
        assert_eq!(cleared["env"]["CUSTOM_VAR"], "keep-me");
        assert_eq!(cleared["other_setting"], true);
    }
}
