use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::AppError;
use crate::provider::Provider;
use crate::storage::atomic_write;

use super::json_merge::{merge_with_null_delete, strip_protected_fields};
use super::{create_backup, rotate_backups, CliAdapter, PatchResult};

const MAX_BACKUPS: usize = 5;

/// Adapter for Claude Code CLI (`~/.claude/settings.json`).
pub struct ClaudeAdapter {
    config_dir: PathBuf,
    backup_dir: PathBuf,
    /// 测试时注入 overlay 文件路径；None 表示使用全局存储（生产路径）。
    overlay_path_override: Option<PathBuf>,
}

impl ClaudeAdapter {
    /// Create adapter using default directories.
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("home directory required");
        Self {
            config_dir: home.join(".claude"),
            backup_dir: home.join(".cli-manager").join("backups").join("claude"),
            overlay_path_override: None,
        }
    }

    /// Create adapter with explicit paths (for testing).
    pub fn new_with_paths(config_dir: PathBuf, backup_dir: PathBuf) -> Self {
        Self {
            config_dir,
            backup_dir,
            overlay_path_override: None,
        }
    }

    /// Create adapter with explicit paths and a custom overlay file path (for testing).
    pub fn new_with_paths_and_overlay(
        config_dir: PathBuf,
        backup_dir: PathBuf,
        overlay_path: PathBuf,
    ) -> Self {
        Self {
            config_dir,
            backup_dir,
            overlay_path_override: Some(overlay_path),
        }
    }

    /// 读取 overlay 文本内容。
    /// - 有 override 路径：直接读取该路径（文件不存在返回 None）
    /// - 无 override：调用全局存储接口 read_claude_settings_overlay()
    fn read_overlay(&self) -> Result<Option<String>, AppError> {
        if let Some(ref path) = self.overlay_path_override {
            if !path.exists() {
                return Ok(None);
            }
            let content = fs::read_to_string(path).map_err(|e| AppError::Io {
                path: path.display().to_string(),
                source: e,
            })?;
            Ok(crate::storage::icloud::normalize_overlay_text(content))
        } else {
            let (content, _info) = crate::storage::icloud::read_claude_settings_overlay()?;
            Ok(content)
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

        // 读取 overlay（可能为 None）
        let overlay_content = self.read_overlay()?;

        // Surgical patch（含 overlay 合并）
        let patched = patch_claude_json(
            &existing,
            &provider.api_key,
            &provider.base_url,
            overlay_content.as_deref(),
        )?;

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

/// 对 Claude Code settings JSON 执行 surgical patch，可选地深度合并 overlay。
///
/// 执行顺序（保证保护字段最终值始终由 provider 决定）：
/// 1. 解析 existing settings 为 JSON root object
/// 2. 确保 env object 存在
/// 3. 若 overlay_text 不为 None：
///    a. 解析 overlay JSON，root 必须为 object
///    b. strip 保护字段
///    c. deep merge overlay 进 root（含 null 删除）
/// 4. 无论 overlay 是否存在，最后强制回写保护字段
fn patch_claude_json(
    existing: &str,
    api_key: &str,
    base_url: &str,
    overlay_text: Option<&str>,
) -> Result<String, AppError> {
    let mut root: Value = serde_json::from_str(existing)
        .map_err(|_| AppError::Validation("failed to parse settings JSON".to_string()))?;

    let root_obj = root.as_object_mut().ok_or_else(|| {
        AppError::Validation("settings.json root is not a JSON object".to_string())
    })?;

    // 确保 env 对象存在
    let env = root_obj
        .entry("env")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));

    env.as_object_mut().ok_or_else(|| {
        AppError::Validation("settings.json env field is not an object".to_string())
    })?;

    // 重新获取 root 的可变引用进行 overlay 合并
    if let Some(text) = overlay_text {
        // 解析 overlay JSON
        let overlay_val: Value = serde_json::from_str(text)
            .map_err(|_| AppError::Validation("overlay JSON is not valid JSON".to_string()))?;

        // overlay root 必须是 object
        if !overlay_val.is_object() {
            return Err(AppError::Validation(
                "overlay root must be a JSON object".to_string(),
            ));
        }

        // strip 保护字段（忽略 stripped_paths，后续 plan 再用于 UI 提示）
        let strip_result = strip_protected_fields(&overlay_val)?;

        // 深度合并（含 null 删除）
        merge_with_null_delete(&mut root, &strip_result.overlay)?;
    }

    // 无论 overlay 是否存在，最后强制回写保护字段（保证 provider 优先级）
    let root_obj = root.as_object_mut().ok_or_else(|| {
        AppError::Validation("settings.json root is not a JSON object after merge".to_string())
    })?;

    let env = root_obj
        .entry("env")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));

    let env_obj = env.as_object_mut().ok_or_else(|| {
        AppError::Validation("settings.json env field is not an object after merge".to_string())
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
            test_model: None,
            upstream_model: None,
            upstream_model_map: None,
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

    // ----------------------------------------------------------------
    // Overlay 集成测试
    // ----------------------------------------------------------------

    fn write_overlay(config_dir: &std::path::Path, content: &str) -> PathBuf {
        let path = config_dir.join("overlay.json");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_patch_with_overlay_merges_env_fields() {
        // overlay 中普通 env 字段写入最终 settings.json
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"{"env": {"ANTHROPIC_AUTH_TOKEN": "old", "ANTHROPIC_BASE_URL": "https://old.example.com"}}"#;
        fs::write(config_dir.join("settings.json"), original).unwrap();

        let overlay = r#"{"env": {"MY_CUSTOM_VAR": "custom-value"}}"#;
        let overlay_path = write_overlay(&config_dir, overlay);

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        // overlay 的自定义字段写入
        assert_eq!(patched["env"]["MY_CUSTOM_VAR"], "custom-value");
        // 保护字段来自 provider
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-new-key-123");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://proxy.example.com"
        );
    }

    #[test]
    fn test_patch_with_overlay_null_deletes_key() {
        // overlay 中 null 会删除目标 key
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"{"env": {"KEEP": "yes", "DELETE_ME": "gone"}, "top_level_del": true}"#;
        fs::write(config_dir.join("settings.json"), original).unwrap();

        // null 删除 env 字段 和 顶层字段
        let overlay = r#"{"env": {"DELETE_ME": null}, "top_level_del": null}"#;
        let overlay_path = write_overlay(&config_dir, overlay);

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        // null 删除成功
        assert!(patched.as_object().unwrap().get("top_level_del").is_none());
        assert!(patched["env"]
            .as_object()
            .unwrap()
            .get("DELETE_ME")
            .is_none());
        // 非删除字段保留
        assert_eq!(patched["env"]["KEEP"], "yes");
    }

    #[test]
    fn test_patch_overlay_cannot_override_protected_fields() {
        // overlay 试图覆盖保护字段时最终仍以 provider 值为准
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        fs::write(config_dir.join("settings.json"), "{}").unwrap();

        // overlay 试图设置两个保护字段
        let overlay = r#"{"env": {"ANTHROPIC_AUTH_TOKEN": "HACKED", "ANTHROPIC_BASE_URL": "https://evil.example.com"}}"#;
        let overlay_path = write_overlay(&config_dir, overlay);

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        // 最终值来自 provider，overlay 被忽略
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-new-key-123");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://proxy.example.com"
        );
    }

    #[test]
    fn test_patch_overlay_invalid_json_returns_validation_error() {
        // overlay 文件非法 JSON 时 patch 返回 Validation
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        fs::write(config_dir.join("settings.json"), "{}").unwrap();

        let overlay_path = config_dir.join("overlay.json");
        fs::write(&overlay_path, "{ not valid json }").unwrap();

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);
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
    fn test_patch_overlay_root_not_object_returns_validation_error() {
        // overlay root 不是 object 时 patch 返回 Validation
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        fs::write(config_dir.join("settings.json"), "{}").unwrap();

        let overlay_path = config_dir.join("overlay.json");
        fs::write(&overlay_path, "[1, 2, 3]").unwrap();

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);
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
    fn test_patch_without_overlay_behaves_exactly_as_before() {
        // overlay 文件不存在时行为与升级前完全一致
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"{
  "permissions": {"allow": ["Bash"]},
  "env": {"ANTHROPIC_AUTH_TOKEN": "old-key", "CUSTOM": "keep-me"}
}"#;
        fs::write(config_dir.join("settings.json"), original).unwrap();

        // 指向不存在的 overlay 文件
        let overlay_path = config_dir.join("nonexistent-overlay.json");
        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-new-key-123");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://proxy.example.com"
        );
        assert_eq!(patched["env"]["CUSTOM"], "keep-me");
        assert!(patched["permissions"]["allow"].is_array());
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

    // ----------------------------------------------------------------
    // COVL-15 / COVL-16 边界测试
    // ----------------------------------------------------------------

    #[test]
    fn test_patch_overlay_protected_and_custom_coexist() {
        // overlay 同时含保护字段和自定义 env 字段：保护字段被剥离，自定义字段写入成功
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("settings.json"), "{}").unwrap();

        let overlay = r#"{"env": {"ANTHROPIC_AUTH_TOKEN": "HACKED", "MY_VAR": "custom"}}"#;
        let overlay_path = write_overlay(&config_dir, overlay);

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        // 保护字段来自 provider，overlay 的值被忽略
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-new-key-123");
        // 自定义字段写入成功
        assert_eq!(patched["env"]["MY_VAR"], "custom");
    }

    #[test]
    fn test_patch_sequential_different_providers() {
        // 先用 provider_a patch，再用 provider_b patch，保护字段始终与当前 provider 一致
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("settings.json"), "{}").unwrap();

        let provider_a = Provider {
            id: "id-a".to_string(),
            cli_id: "claude".to_string(),
            name: "Provider A".to_string(),
            protocol_type: crate::provider::ProtocolType::Anthropic,
            api_key: "key-a".to_string(),
            base_url: "url-a".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            model_config: None,
            notes: None,
            test_model: None,
            upstream_model: None,
            upstream_model_map: None,
            created_at: 1710000000000,
            updated_at: 1710000000000,
            schema_version: 1,
        };
        let provider_b = Provider {
            id: "id-b".to_string(),
            cli_id: "claude".to_string(),
            name: "Provider B".to_string(),
            protocol_type: crate::provider::ProtocolType::Anthropic,
            api_key: "key-b".to_string(),
            base_url: "url-b".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            model_config: None,
            notes: None,
            test_model: None,
            upstream_model: None,
            upstream_model_map: None,
            created_at: 1710000000000,
            updated_at: 1710000000000,
            schema_version: 1,
        };

        let overlay = r#"{"env": {"EXTRA": "shared"}}"#;
        let overlay_path = write_overlay(&config_dir, overlay);

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);

        // 第一次：patch provider_a
        adapter.patch(&provider_a).unwrap();
        let patched_a: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(patched_a["env"]["ANTHROPIC_AUTH_TOKEN"], "key-a");
        assert_eq!(patched_a["env"]["ANTHROPIC_BASE_URL"], "url-a");
        assert_eq!(patched_a["env"]["EXTRA"], "shared");

        // 第二次：patch provider_b（切换 provider）
        adapter.patch(&provider_b).unwrap();
        let patched_b: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(patched_b["env"]["ANTHROPIC_AUTH_TOKEN"], "key-b");
        assert_eq!(patched_b["env"]["ANTHROPIC_BASE_URL"], "url-b");
        assert_eq!(patched_b["env"]["EXTRA"], "shared");
    }

    #[test]
    fn test_patch_then_clear_overlay_fields_survive() {
        // patch with overlay 注入自定义字段后，clear 只删除保护字段，overlay 注入的字段保留
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("settings.json"), "{}").unwrap();

        let overlay = r#"{"env": {"MY_OVERLAY_VAR": "injected"}}"#;
        let overlay_path = write_overlay(&config_dir, overlay);

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);

        // patch 后验证保护字段和自定义字段都存在
        adapter.patch(&test_provider()).unwrap();
        let after_patch: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(
            after_patch["env"]["ANTHROPIC_AUTH_TOKEN"],
            "sk-ant-new-key-123"
        );
        assert_eq!(after_patch["env"]["MY_OVERLAY_VAR"], "injected");

        // clear 后验证保护字段被删除，overlay 注入的自定义字段保留
        adapter.clear().unwrap();
        let after_clear: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();
        assert!(after_clear["env"]
            .as_object()
            .unwrap()
            .get("ANTHROPIC_AUTH_TOKEN")
            .is_none());
        assert!(after_clear["env"]
            .as_object()
            .unwrap()
            .get("ANTHROPIC_BASE_URL")
            .is_none());
        assert_eq!(after_clear["env"]["MY_OVERLAY_VAR"], "injected");
    }

    #[test]
    fn test_patch_with_overlay_adds_top_level_keys() {
        // overlay 在 env 之外添加顶层 key（如 permissions），与 surgical patch 行为共存
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"{"env": {}}"#;
        fs::write(config_dir.join("settings.json"), original).unwrap();

        let overlay = r#"{"permissions": {"allow": ["Bash", "Read"]}, "env": {"EXTRA": "val"}}"#;
        let overlay_path = write_overlay(&config_dir, overlay);

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        // overlay 顶层 key 写入
        assert_eq!(
            patched["permissions"]["allow"],
            serde_json::json!(["Bash", "Read"])
        );
        // overlay env 字段写入
        assert_eq!(patched["env"]["EXTRA"], "val");
        // 保护字段来自 provider
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-new-key-123");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://proxy.example.com"
        );
    }

    #[test]
    fn test_patch_with_empty_overlay_object() {
        // overlay 文件内容为 `{}`，patch 行为与无 overlay 完全一致
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"{"permissions": {"allow": ["Bash"]}, "env": {"ANTHROPIC_AUTH_TOKEN": "old", "CUSTOM": "keep"}}"#;
        fs::write(config_dir.join("settings.json"), original).unwrap();

        let overlay = r#"{}"#;
        let overlay_path = write_overlay(&config_dir, overlay);

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        // 保护字段来自 provider
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-new-key-123");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://proxy.example.com"
        );
        // 原有字段保留
        assert_eq!(patched["env"]["CUSTOM"], "keep");
        assert!(patched["permissions"]["allow"].is_array());
    }

    #[test]
    fn test_patch_with_blank_overlay_file_behaves_like_no_overlay() {
        // overlay 文件为空白时，视为“已清空”，行为应等同于无 overlay
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&config_dir).unwrap();

        let original = r#"{"permissions": {"allow": ["Bash"]}, "env": {"ANTHROPIC_AUTH_TOKEN": "old", "CUSTOM": "keep"}}"#;
        fs::write(config_dir.join("settings.json"), original).unwrap();

        let overlay_path = write_overlay(&config_dir, " \n\t ");

        let adapter =
            ClaudeAdapter::new_with_paths_and_overlay(config_dir.clone(), backup_dir, overlay_path);
        adapter.patch(&test_provider()).unwrap();

        let patched: Value =
            serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-new-key-123");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://proxy.example.com"
        );
        assert_eq!(patched["env"]["CUSTOM"], "keep");
        assert!(patched["permissions"]["allow"].is_array());
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
