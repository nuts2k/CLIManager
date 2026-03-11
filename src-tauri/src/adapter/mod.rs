pub mod claude;
pub mod codex;

use crate::error::AppError;
use crate::provider::Provider;
use std::fs;
use std::path::Path;

/// Result of a patch operation.
#[derive(Debug)]
pub struct PatchResult {
    pub files_written: Vec<String>,
    pub backups_created: Vec<String>,
}

/// Unified interface for CLI config patching.
pub trait CliAdapter {
    /// Human-readable CLI name for error messages.
    fn cli_name(&self) -> &str;

    /// Patch CLI config files with the given provider's credentials.
    fn patch(&self, provider: &Provider) -> Result<PatchResult, AppError>;
}

/// Create a timestamped backup of `source` in `backup_dir`.
/// Returns the backup file path as a String.
pub fn create_backup(source: &Path, backup_dir: &Path) -> Result<String, AppError> {
    fs::create_dir_all(backup_dir).map_err(|e| AppError::Io {
        path: backup_dir.display().to_string(),
        source: e,
    })?;

    let filename = source.file_name().unwrap_or_default().to_string_lossy();
    let timestamp = chrono::Local::now().format("%Y-%m-%dT%H-%M-%S%.3f");
    let backup_name = format!("{}.{}.bak", filename, timestamp);
    let backup_path = backup_dir.join(&backup_name);

    fs::copy(source, &backup_path).map_err(|e| AppError::Io {
        path: backup_path.display().to_string(),
        source: e,
    })?;

    Ok(backup_path.display().to_string())
}

/// Rotate backups in `backup_dir`, keeping at most `max_count` .bak files.
/// Deletes oldest first, ordered by the timestamp suffix in the backup filename.
pub fn rotate_backups(backup_dir: &Path, max_count: usize) -> Result<(), AppError> {
    let entries = fs::read_dir(backup_dir).map_err(|e| AppError::Io {
        path: backup_dir.display().to_string(),
        source: e,
    })?;

    let mut backups: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "bak"))
        .collect();

    backups.sort_by(|a, b| {
        let a_name = a.file_name().to_string_lossy().to_string();
        let b_name = b.file_name().to_string_lossy().to_string();
        backup_sort_key(&a_name).cmp(&backup_sort_key(&b_name))
    });

    while backups.len() > max_count {
        let oldest = backups.remove(0);
        let _ = fs::remove_file(oldest.path()); // best-effort
    }

    Ok(())
}

/// Restore a file from its most recent backup in `backup_dir`.
/// Finds .bak files whose name starts with the target filename, sorts descending, copies the newest.
pub fn restore_from_backup(target: &Path, backup_dir: &Path) -> Result<(), AppError> {
    let target_filename = target
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let entries = fs::read_dir(backup_dir).map_err(|e| AppError::Io {
        path: backup_dir.display().to_string(),
        source: e,
    })?;

    let mut backups: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with(&target_filename) && name.ends_with(".bak")
        })
        .collect();

    backups.sort_by(|a, b| {
        let a_name = a.file_name().to_string_lossy().to_string();
        let b_name = b.file_name().to_string_lossy().to_string();
        backup_sort_key(&b_name).cmp(&backup_sort_key(&a_name))
    }); // descending = newest first

    let newest = backups
        .first()
        .ok_or_else(|| AppError::Validation("No backup found for rollback".to_string()))?;

    fs::copy(newest.path(), target).map_err(|e| AppError::Io {
        path: target.display().to_string(),
        source: e,
    })?;

    Ok(())
}

fn backup_sort_key(filename: &str) -> (String, String) {
    (
        extract_backup_timestamp(filename)
            .unwrap_or_default()
            .to_string(),
        filename.to_string(),
    )
}

fn extract_backup_timestamp(filename: &str) -> Option<&str> {
    let stem = filename.strip_suffix(".bak")?;
    let timestamp_start = stem.len().checked_sub(23)?;
    let (prefix, timestamp) = stem.split_at(timestamp_start);

    if !prefix.ends_with('.') {
        return None;
    }

    Some(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_backup_copies_file_with_timestamp_suffix() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("settings.json");
        fs::write(&source, r#"{"key":"value"}"#).unwrap();

        let backup_dir = tmp.path().join("backups");
        let result = create_backup(&source, &backup_dir).unwrap();

        assert!(result.contains("settings.json."));
        assert!(result.ends_with(".bak"));

        // Backup content matches source
        let backup_content = fs::read_to_string(&result).unwrap();
        assert_eq!(backup_content, r#"{"key":"value"}"#);
    }

    #[test]
    fn test_create_backup_creates_backup_dir_if_missing() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("settings.json");
        fs::write(&source, "{}").unwrap();

        let backup_dir = tmp.path().join("deep").join("nested").join("backups");
        assert!(!backup_dir.exists());

        let result = create_backup(&source, &backup_dir);
        assert!(result.is_ok());
        assert!(backup_dir.exists());
    }

    #[test]
    fn test_rotate_backups_keeps_at_most_n_backups() {
        let tmp = TempDir::new().unwrap();
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        // Create 7 backup files with ordered timestamps
        for i in 0..7 {
            let name = format!("settings.json.2026-03-11T10-00-0{}.000.bak", i);
            fs::write(backup_dir.join(&name), "content").unwrap();
        }

        rotate_backups(&backup_dir, 5).unwrap();

        let remaining: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "bak"))
            .collect();
        assert_eq!(remaining.len(), 5);

        // The oldest two (00, 01) should have been deleted
        assert!(!backup_dir
            .join("settings.json.2026-03-11T10-00-00.000.bak")
            .exists());
        assert!(!backup_dir
            .join("settings.json.2026-03-11T10-00-01.000.bak")
            .exists());
    }

    #[test]
    fn test_rotate_backups_noop_when_under_limit() {
        let tmp = TempDir::new().unwrap();
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        for i in 0..3 {
            let name = format!("settings.json.2026-03-11T10-00-0{}.000.bak", i);
            fs::write(backup_dir.join(&name), "content").unwrap();
        }

        rotate_backups(&backup_dir, 5).unwrap();

        let remaining: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "bak"))
            .collect();
        assert_eq!(remaining.len(), 3);
    }

    #[test]
    fn test_rotate_backups_keeps_newest_when_backup_types_are_mixed() {
        let tmp = TempDir::new().unwrap();
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        for i in 0..5 {
            let name = format!("config.toml.2026-03-11T10-00-0{}.000.bak", i);
            fs::write(backup_dir.join(&name), "config").unwrap();
        }
        fs::write(
            backup_dir.join("auth.json.2026-03-11T10-00-05.000.bak"),
            "auth",
        )
        .unwrap();

        rotate_backups(&backup_dir, 5).unwrap();

        assert!(backup_dir
            .join("auth.json.2026-03-11T10-00-05.000.bak")
            .exists());
        assert!(!backup_dir
            .join("config.toml.2026-03-11T10-00-00.000.bak")
            .exists());
    }

    #[test]
    fn test_apperror_toml_variant_displays_message() {
        let err = AppError::Toml("unexpected token at line 3".to_string());
        assert_eq!(err.to_string(), "TOML error: unexpected token at line 3");
    }

    #[test]
    fn test_apperror_validation_variant_displays_message() {
        let err = AppError::Validation("invalid JSON in settings.json".to_string());
        assert_eq!(
            err.to_string(),
            "Validation failed: invalid JSON in settings.json"
        );
    }

    #[test]
    fn test_restore_from_backup_restores_newest() {
        let tmp = TempDir::new().unwrap();
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        let target = tmp.path().join("auth.json");
        fs::write(&target, "current").unwrap();

        // Create two backups with ordered timestamps
        fs::write(
            backup_dir.join("auth.json.2026-03-11T10-00-01.000.bak"),
            "backup-old",
        )
        .unwrap();
        fs::write(
            backup_dir.join("auth.json.2026-03-11T10-00-02.000.bak"),
            "backup-new",
        )
        .unwrap();

        restore_from_backup(&target, &backup_dir).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "backup-new");
    }

    #[test]
    fn test_restore_from_backup_fails_when_no_backup() {
        let tmp = TempDir::new().unwrap();
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        let target = tmp.path().join("auth.json");
        let result = restore_from_backup(&target, &backup_dir);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No backup found"));
    }

    #[test]
    fn test_integration_both_adapters_together() {
        use crate::adapter::claude::ClaudeAdapter;
        use crate::adapter::codex::CodexAdapter;
        use crate::provider::{ProtocolType, Provider};

        let provider = Provider {
            id: "test-id".to_string(),
            name: "Test Provider".to_string(),
            protocol_type: ProtocolType::OpenAiCompatible,
            api_key: "sk-integration-test-key".to_string(),
            base_url: "https://proxy.integration.test/v1".to_string(),
            model: "o4-mini".to_string(),
            model_config: None,
            notes: None,
            created_at: 1710000000000,
            updated_at: 1710000000000,
            schema_version: 1,
        };

        let tmp = TempDir::new().unwrap();

        // Claude adapter dirs
        let claude_config = tmp.path().join("claude-config");
        let claude_backup = tmp.path().join("claude-backup");
        fs::create_dir_all(&claude_config).unwrap();

        // Write Claude settings with extra keys
        let claude_settings = r#"{
  "permissions": {"allow": ["Bash", "Read"]},
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "old-claude-key",
    "CUSTOM_VAR": "keep-me"
  }
}"#;
        fs::write(claude_config.join("settings.json"), claude_settings).unwrap();

        // Codex adapter dirs
        let codex_config = tmp.path().join("codex-config");
        let codex_backup = tmp.path().join("codex-backup");
        fs::create_dir_all(&codex_config).unwrap();

        // Write Codex files with extra keys and comments
        let codex_auth = r#"{"OPENAI_API_KEY": "old-codex-key", "auth_mode": "api_key"}"#;
        fs::write(codex_config.join("auth.json"), codex_auth).unwrap();

        let codex_toml = r#"# Codex config
model_provider = "openai"

[model_providers.openai]
model = "o4-mini"
base_url = "https://old.example.com"

[projects]
# Project settings
sandbox = true
"#;
        fs::write(codex_config.join("config.toml"), codex_toml).unwrap();

        // Patch both adapters
        let claude_adapter =
            ClaudeAdapter::new_with_paths(claude_config.clone(), claude_backup.clone());
        let claude_result = claude_adapter.patch(&provider).unwrap();

        let codex_adapter =
            CodexAdapter::new_with_paths(codex_config.clone(), codex_backup.clone());
        let codex_result = codex_adapter.patch(&provider).unwrap();

        // Verify Claude results
        assert_eq!(claude_result.files_written.len(), 1);
        assert!(claude_result.files_written[0].contains("settings.json"));
        assert_eq!(claude_result.backups_created.len(), 1);

        // Verify Codex results
        assert_eq!(codex_result.files_written.len(), 2);
        assert!(codex_result.files_written[0].contains("auth.json"));
        assert!(codex_result.files_written[1].contains("config.toml"));
        assert_eq!(codex_result.backups_created.len(), 2);

        // Re-read and verify Claude: only target fields changed
        let claude_patched: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(claude_config.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(
            claude_patched["env"]["ANTHROPIC_AUTH_TOKEN"],
            "sk-integration-test-key"
        );
        assert_eq!(claude_patched["env"]["CUSTOM_VAR"], "keep-me");
        assert!(claude_patched["permissions"]["allow"].is_array());

        // Re-read and verify Codex auth.json: only OPENAI_API_KEY changed
        let codex_auth_patched: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(codex_config.join("auth.json")).unwrap())
                .unwrap();
        assert_eq!(
            codex_auth_patched["OPENAI_API_KEY"],
            "sk-integration-test-key"
        );
        assert_eq!(codex_auth_patched["auth_mode"], "api_key");

        // Re-read and verify Codex config.toml: only base_url changed, comments survive
        let codex_toml_content = fs::read_to_string(codex_config.join("config.toml")).unwrap();
        assert!(codex_toml_content.contains("# Codex config"));
        assert!(codex_toml_content.contains("# Project settings"));
        let doc: toml_edit::DocumentMut = codex_toml_content.parse().unwrap();
        assert_eq!(
            doc["model_providers"]["openai"]["base_url"]
                .as_str()
                .unwrap(),
            "https://proxy.integration.test/v1"
        );
        assert_eq!(
            doc["model_providers"]["openai"]["model"].as_str().unwrap(),
            "o4-mini"
        );
        assert_eq!(doc["projects"]["sandbox"].as_bool().unwrap(), true);

        // Verify backup files exist on disk
        assert!(std::path::Path::new(&claude_result.backups_created[0]).exists());
        assert!(std::path::Path::new(&codex_result.backups_created[0]).exists());
        assert!(std::path::Path::new(&codex_result.backups_created[1]).exists());
    }
}
