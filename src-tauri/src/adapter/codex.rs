use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use toml_edit::DocumentMut;

use crate::error::AppError;
use crate::provider::Provider;
use crate::storage::atomic_write;

use super::{create_backup, restore_from_backup, rotate_backups, CliAdapter, PatchResult};

const MAX_BACKUPS: usize = 5;

/// Adapter for Codex CLI (`~/.codex/auth.json` + `~/.codex/config.toml`).
pub struct CodexAdapter {
    config_dir: PathBuf,
    backup_dir: PathBuf,
}

impl CodexAdapter {
    /// Create adapter using default directories.
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("home directory required");
        Self {
            config_dir: home.join(".codex"),
            backup_dir: home.join(".cli-manager").join("backups").join("codex"),
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

impl CliAdapter for CodexAdapter {
    fn cli_name(&self) -> &str {
        "codex"
    }

    fn patch(&self, provider: &Provider) -> Result<PatchResult, AppError> {
        let auth_path = self.config_dir.join("auth.json");
        let config_path = self.config_dir.join("config.toml");
        let mut backups_created = Vec::new();

        // Read existing content or defaults
        let auth_existing = if auth_path.exists() {
            let content = fs::read_to_string(&auth_path).map_err(|e| AppError::Io {
                path: auth_path.display().to_string(),
                source: e,
            })?;
            // Pre-validation: must be valid JSON
            serde_json::from_str::<Value>(&content).map_err(|_| {
                AppError::Validation(format!(
                    "existing {} is not valid JSON",
                    auth_path.display()
                ))
            })?;
            content
        } else {
            "{}".to_string()
        };

        let config_existing = if config_path.exists() {
            let content = fs::read_to_string(&config_path).map_err(|e| AppError::Io {
                path: config_path.display().to_string(),
                source: e,
            })?;
            // Pre-validation: must be valid TOML
            content.parse::<DocumentMut>().map_err(|e| {
                AppError::Toml(format!(
                    "existing {} is not valid TOML: {}",
                    config_path.display(),
                    e
                ))
            })?;
            content
        } else {
            String::new()
        };

        // Backup both files (if they exist) before any writes
        if auth_path.exists() {
            let backup_path = create_backup(&auth_path, &self.backup_dir)?;
            rotate_backups(&self.backup_dir, MAX_BACKUPS)?;
            backups_created.push(backup_path);
        }
        if config_path.exists() {
            let backup_path = create_backup(&config_path, &self.backup_dir)?;
            rotate_backups(&self.backup_dir, MAX_BACKUPS)?;
            backups_created.push(backup_path);
        }

        // Surgical patch auth.json
        let auth_patched = patch_codex_auth_json(&auth_existing, &provider.api_key)?;

        // Post-validate auth.json
        serde_json::from_str::<Value>(&auth_patched).map_err(|_| {
            AppError::Validation("patched auth.json is not valid JSON".to_string())
        })?;

        // Ensure config dir exists
        fs::create_dir_all(&self.config_dir).map_err(|e| AppError::Io {
            path: self.config_dir.display().to_string(),
            source: e,
        })?;

        // Phase 1 write: auth.json
        atomic_write(&auth_path, auth_patched.as_bytes())?;

        // Surgical patch config.toml
        let toml_patched = patch_codex_toml(&config_existing, &provider.base_url)?;

        // Post-validate config.toml
        toml_patched.parse::<DocumentMut>().map_err(|e| {
            AppError::Toml(format!("patched config.toml is not valid TOML: {}", e))
        })?;

        // Phase 2 write: config.toml
        match atomic_write(&config_path, toml_patched.as_bytes()) {
            Ok(()) => {}
            Err(e) => {
                // Rollback auth.json from backup
                let _ = restore_from_backup(&auth_path, &self.backup_dir);
                return Err(e);
            }
        }

        Ok(PatchResult {
            files_written: vec![
                auth_path.display().to_string(),
                config_path.display().to_string(),
            ],
            backups_created,
        })
    }
}

/// Surgically patch Codex auth.json.
/// Only modifies `OPENAI_API_KEY` at top level. All other keys survive.
fn patch_codex_auth_json(existing: &str, api_key: &str) -> Result<String, AppError> {
    let mut root: Value = serde_json::from_str(existing).map_err(|_| {
        AppError::Validation("failed to parse auth.json".to_string())
    })?;

    let root_obj = root.as_object_mut().ok_or_else(|| {
        AppError::Validation("auth.json root is not a JSON object".to_string())
    })?;

    root_obj.insert(
        "OPENAI_API_KEY".to_string(),
        Value::String(api_key.to_string()),
    );

    serde_json::to_string_pretty(&root).map_err(|e| AppError::Json(e))
}

/// Surgically patch Codex config.toml.
/// Only modifies `base_url` at top level. All other keys, tables, and comments survive.
fn patch_codex_toml(existing: &str, base_url: &str) -> Result<String, AppError> {
    let mut doc: DocumentMut = existing.parse().map_err(|e: toml_edit::TomlError| {
        AppError::Toml(format!("failed to parse config.toml: {}", e))
    })?;

    doc["base_url"] = toml_edit::value(base_url);

    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ProtocolType, Provider};
    use tempfile::TempDir;

    fn test_provider() -> Provider {
        Provider {
            id: "test-id".to_string(),
            name: "Test Provider".to_string(),
            protocol_type: ProtocolType::OpenAiCompatible,
            api_key: "sk-openai-new-key-456".to_string(),
            base_url: "https://new-proxy.example.com/v1".to_string(),
            model: "o4-mini".to_string(),
            model_config: None,
            notes: None,
            created_at: 1710000000000,
            updated_at: 1710000000000,
            schema_version: 1,
        }
    }

    #[test]
    fn test_patch_existing_auth_json_only_modifies_openai_api_key() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"{
  "OPENAI_API_KEY": "old-key",
  "auth_mode": "api_key",
  "tokens": {"refresh": "abc123"}
}"#;
        fs::write(config_dir.join("auth.json"), original).unwrap();

        let adapter = CodexAdapter::new_with_paths(config_dir.clone(), backup_dir);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("auth.json")).unwrap())
                .unwrap();

        // Target field updated
        assert_eq!(patched["OPENAI_API_KEY"], "sk-openai-new-key-456");
        // Other fields survive
        assert_eq!(patched["auth_mode"], "api_key");
        assert_eq!(patched["tokens"]["refresh"], "abc123");
    }

    #[test]
    fn test_patch_existing_config_toml_only_modifies_base_url() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"model = "o4-mini"
base_url = "https://old.example.com"
temperature = 0.7
"#;
        fs::write(config_dir.join("config.toml"), original).unwrap();
        // Also need auth.json for patch to succeed
        fs::write(config_dir.join("auth.json"), "{}").unwrap();

        let adapter = CodexAdapter::new_with_paths(config_dir.clone(), backup_dir);
        adapter.patch(&test_provider()).unwrap();

        let content = fs::read_to_string(config_dir.join("config.toml")).unwrap();
        let doc: DocumentMut = content.parse().unwrap();

        // Target field updated
        assert_eq!(
            doc["base_url"].as_str().unwrap(),
            "https://new-proxy.example.com/v1"
        );
        // Other fields survive
        assert_eq!(doc["model"].as_str().unwrap(), "o4-mini");
        assert_eq!(doc["temperature"].as_float().unwrap(), 0.7);
    }

    #[test]
    fn test_toml_comments_survive_patching() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"# This is a user comment
model = "o4-mini"
base_url = "https://old.example.com"

[projects]
# Project settings
sandbox = true
"#;
        fs::write(config_dir.join("config.toml"), original).unwrap();
        fs::write(config_dir.join("auth.json"), "{}").unwrap();

        let adapter = CodexAdapter::new_with_paths(config_dir.clone(), backup_dir);
        adapter.patch(&test_provider()).unwrap();

        let content = fs::read_to_string(config_dir.join("config.toml")).unwrap();
        assert!(
            content.contains("# This is a user comment"),
            "User comment should survive, got: {}",
            content
        );
        assert!(
            content.contains("# Project settings"),
            "Project settings comment should survive, got: {}",
            content
        );
    }

    #[test]
    fn test_toml_table_structure_survives_patching() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"model = "o4-mini"
base_url = "https://old.example.com"

[projects]
sandbox = true

[logging]
level = "info"
"#;
        fs::write(config_dir.join("config.toml"), original).unwrap();
        fs::write(config_dir.join("auth.json"), "{}").unwrap();

        let adapter = CodexAdapter::new_with_paths(config_dir.clone(), backup_dir);
        adapter.patch(&test_provider()).unwrap();

        let content = fs::read_to_string(config_dir.join("config.toml")).unwrap();
        let doc: DocumentMut = content.parse().unwrap();

        assert_eq!(doc["projects"]["sandbox"].as_bool().unwrap(), true);
        assert_eq!(doc["logging"]["level"].as_str().unwrap(), "info");
    }

    #[test]
    fn test_patch_creates_new_auth_json_when_missing() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        // Don't create config_dir -- adapter should create it

        let adapter = CodexAdapter::new_with_paths(config_dir.clone(), backup_dir);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("auth.json")).unwrap())
                .unwrap();

        assert_eq!(patched["OPENAI_API_KEY"], "sk-openai-new-key-456");
        // Should only have the one key we set
        assert_eq!(patched.as_object().unwrap().len(), 1);
    }

    #[test]
    fn test_patch_creates_new_config_toml_when_missing() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");

        let adapter = CodexAdapter::new_with_paths(config_dir.clone(), backup_dir);
        adapter.patch(&test_provider()).unwrap();

        let content = fs::read_to_string(config_dir.join("config.toml")).unwrap();
        let doc: DocumentMut = content.parse().unwrap();

        assert_eq!(
            doc["base_url"].as_str().unwrap(),
            "https://new-proxy.example.com/v1"
        );
    }

    #[test]
    fn test_backups_created_for_both_files_when_they_exist() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        fs::write(config_dir.join("auth.json"), r#"{"OPENAI_API_KEY":"old"}"#).unwrap();
        fs::write(config_dir.join("config.toml"), "base_url = \"old\"\n").unwrap();

        let adapter = CodexAdapter::new_with_paths(config_dir, backup_dir);
        let result = adapter.patch(&test_provider()).unwrap();

        assert_eq!(result.backups_created.len(), 2);
        assert!(result.backups_created[0].contains("auth.json"));
        assert!(result.backups_created[1].contains("config.toml"));
        // Both backups exist on disk
        assert!(std::path::Path::new(&result.backups_created[0]).exists());
        assert!(std::path::Path::new(&result.backups_created[1]).exists());
    }

    #[test]
    fn test_rollback_auth_json_when_config_toml_write_fails() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original_auth = r#"{"OPENAI_API_KEY": "original-key"}"#;
        fs::write(config_dir.join("auth.json"), original_auth).unwrap();

        // Make config.toml a directory so atomic_write will fail
        fs::create_dir_all(config_dir.join("config.toml")).unwrap();

        let adapter = CodexAdapter::new_with_paths(config_dir.clone(), backup_dir);
        let result = adapter.patch(&test_provider());

        assert!(result.is_err(), "Patch should fail when config.toml write fails");

        // auth.json should be restored from backup to original content
        let restored: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("auth.json")).unwrap())
                .unwrap();
        assert_eq!(
            restored["OPENAI_API_KEY"], "original-key",
            "auth.json should be rolled back to original content"
        );
    }

    #[test]
    fn test_pre_validation_fails_on_unparseable_json() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        fs::write(config_dir.join("auth.json"), "{ invalid json }").unwrap();

        let adapter = CodexAdapter::new_with_paths(config_dir, backup_dir);
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
    fn test_pre_validation_fails_on_unparseable_toml() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        fs::write(config_dir.join("auth.json"), "{}").unwrap();
        fs::write(config_dir.join("config.toml"), "[invalid toml\nfoo =").unwrap();

        let adapter = CodexAdapter::new_with_paths(config_dir, backup_dir);
        let result = adapter.patch(&test_provider());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("TOML"),
            "Expected TOML error, got: {}",
            err
        );
    }

    #[test]
    fn test_patch_result_contains_correct_files_and_backups() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        fs::write(config_dir.join("auth.json"), "{}").unwrap();
        fs::write(config_dir.join("config.toml"), "").unwrap();

        let adapter = CodexAdapter::new_with_paths(config_dir, backup_dir);
        let result = adapter.patch(&test_provider()).unwrap();

        assert_eq!(result.files_written.len(), 2);
        assert!(result.files_written[0].contains("auth.json"));
        assert!(result.files_written[1].contains("config.toml"));

        assert_eq!(result.backups_created.len(), 2);
        assert!(result.backups_created[0].ends_with(".bak"));
        assert!(result.backups_created[1].ends_with(".bak"));
    }
}
