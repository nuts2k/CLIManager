use std::path::Path;

use serde::Serialize;
use tauri::Manager;

use crate::commands::provider::normalize_and_validate_provider;
use crate::error::AppError;
use crate::provider::{suggested_test_model, suggested_upstream_model, ProtocolType, Provider};

#[derive(Debug, Clone, Serialize)]
pub struct DetectedCliConfig {
    pub cli_id: String,
    pub cli_name: String,
    pub api_key: String,
    pub base_url: String,
    pub protocol_type: ProtocolType,
    pub has_api_key: bool,
}

/// Scan Claude config at {home_dir}/.claude/settings.json
pub fn scan_claude_config_in(home_dir: &Path) -> Option<DetectedCliConfig> {
    let settings_path = home_dir.join(".claude").join("settings.json");
    let content = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Failed to parse Claude settings.json: {}", e);
            return None;
        }
    };

    let env = value.get("env");

    let api_key = env
        .and_then(|e| e.get("ANTHROPIC_AUTH_TOKEN"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let base_url = env
        .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
        .and_then(|v| v.as_str())
        .unwrap_or("https://api.anthropic.com")
        .to_string();

    let has_api_key = !api_key.is_empty();

    Some(DetectedCliConfig {
        cli_id: "claude".to_string(),
        cli_name: "Claude Code".to_string(),
        api_key,
        base_url,
        protocol_type: ProtocolType::Anthropic,
        has_api_key,
    })
}

/// Scan Codex config at {home_dir}/.codex/auth.json and {home_dir}/.codex/config.toml
pub fn scan_codex_config_in(home_dir: &Path) -> Option<DetectedCliConfig> {
    let codex_dir = home_dir.join(".codex");
    let auth_path = codex_dir.join("auth.json");
    let config_path = codex_dir.join("config.toml");

    let auth_exists = auth_path.exists();
    let config_exists = config_path.exists();

    // Return None if neither file exists
    if !auth_exists && !config_exists {
        return None;
    }

    // Extract API key from auth.json
    let mut api_key = String::new();
    let mut auth_parsed = false;
    if auth_exists {
        match std::fs::read_to_string(&auth_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    auth_parsed = true;
                    api_key = value
                        .get("OPENAI_API_KEY")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
                Err(e) => {
                    log::warn!("Failed to parse Codex auth.json: {}", e);
                }
            },
            Err(e) => {
                log::warn!("Failed to read Codex auth.json: {}", e);
            }
        }
    }

    // Extract base_url from config.toml
    let mut base_url = String::new();
    let mut config_parsed = false;
    if config_exists {
        match std::fs::read_to_string(&config_path) {
            Ok(content) => match content.parse::<toml_edit::DocumentMut>() {
                Ok(doc) => {
                    config_parsed = true;
                    // Check model_provider field -> model_providers.<active>.base_url -> top-level base_url
                    let provider_scoped = doc
                        .get("model_provider")
                        .and_then(|v| v.as_str())
                        .and_then(|provider_name| {
                            doc.get("model_providers")
                                .and_then(|mp| mp.get(provider_name))
                                .and_then(|p| p.get("base_url"))
                                .and_then(|v| v.as_str())
                        });

                    if let Some(url) = provider_scoped {
                        base_url = url.to_string();
                    } else if let Some(url) = doc.get("base_url").and_then(|v| v.as_str()) {
                        base_url = url.to_string();
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse Codex config.toml: {}", e);
                }
            },
            Err(e) => {
                log::warn!("Failed to read Codex config.toml: {}", e);
            }
        }
    }

    if !auth_parsed && !config_parsed {
        return None;
    }

    let has_api_key = !api_key.is_empty();

    Some(DetectedCliConfig {
        cli_id: "codex".to_string(),
        cli_name: "Codex".to_string(),
        api_key,
        base_url,
        protocol_type: ProtocolType::OpenAiChatCompletions,
        has_api_key,
    })
}

#[tauri::command]
pub fn scan_cli_configs() -> Result<Vec<DetectedCliConfig>, AppError> {
    let home_dir = match dirs::home_dir() {
        Some(dir) => dir,
        None => return Ok(vec![]),
    };

    let mut results = Vec::new();

    if let Some(claude) = scan_claude_config_in(&home_dir) {
        results.push(claude);
    }

    if let Some(codex) = scan_codex_config_in(&home_dir) {
        results.push(codex);
    }

    Ok(results)
}

// --- import_provider internals ---

/// Internal import that writes to a specific directory (for testability).
pub fn import_provider_to(
    dir: &Path,
    name: String,
    protocol_type: ProtocolType,
    api_key: String,
    base_url: String,
    cli_id: String,
) -> Result<Provider, AppError> {
    let mut provider = Provider::new(
        name,
        protocol_type,
        api_key,
        base_url,
        String::new(), // model is empty for imports (only API key + base URL)
        cli_id,
    );
    provider.test_model = Some(suggested_test_model(&provider.protocol_type).to_string());
    provider.upstream_model = suggested_upstream_model(&provider.protocol_type).map(str::to_string);
    provider = normalize_and_validate_provider(provider)?;

    crate::storage::icloud::save_provider_to(dir, &provider)?;
    Ok(provider)
}

#[tauri::command]
pub fn import_provider(
    app_handle: tauri::AppHandle,
    name: String,
    protocol_type: ProtocolType,
    api_key: String,
    base_url: String,
    cli_id: String,
) -> Result<Provider, AppError> {
    let dir = crate::storage::icloud::get_icloud_providers_dir()?;

    let mut provider = Provider::new(
        name,
        protocol_type,
        api_key,
        base_url,
        String::new(),
        cli_id,
    );
    provider.test_model = Some(suggested_test_model(&provider.protocol_type).to_string());
    provider.upstream_model = suggested_upstream_model(&provider.protocol_type).map(str::to_string);
    provider = normalize_and_validate_provider(provider)?;

    // Record self-write BEFORE the file operation so the watcher ignores this change
    let tracker = app_handle.state::<crate::watcher::SelfWriteTracker>();
    tracker.record_write(dir.join(format!("{}.json", provider.id)));

    crate::storage::icloud::save_provider_to(&dir, &provider)?;
    Ok(provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // --- Claude config tests ---

    #[test]
    fn test_scan_claude_valid_config() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("settings.json"),
            r#"{
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": "sk-ant-api03-test-key-12345",
                    "ANTHROPIC_BASE_URL": "https://custom.anthropic.com"
                }
            }"#,
        )
        .unwrap();

        let result = scan_claude_config_in(tmp.path());
        assert!(result.is_some());
        let config = result.unwrap();
        assert_eq!(config.cli_id, "claude");
        assert_eq!(config.cli_name, "Claude Code");
        assert_eq!(config.api_key, "sk-ant-api03-test-key-12345");
        assert_eq!(config.base_url, "https://custom.anthropic.com");
        assert_eq!(config.protocol_type, ProtocolType::Anthropic);
        assert!(config.has_api_key);
    }

    #[test]
    fn test_scan_claude_missing_api_key() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"env": {"ANTHROPIC_BASE_URL": "https://api.anthropic.com"}}"#,
        )
        .unwrap();

        let result = scan_claude_config_in(tmp.path());
        assert!(result.is_some());
        let config = result.unwrap();
        assert!(!config.has_api_key);
        assert_eq!(config.api_key, "");
        assert_eq!(config.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn test_scan_claude_missing_file() {
        let tmp = TempDir::new().unwrap();
        // No .claude directory at all
        let result = scan_claude_config_in(tmp.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_scan_claude_corrupted_json() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join("settings.json"), "not valid json{{{").unwrap();

        let result = scan_claude_config_in(tmp.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_scan_claude_default_base_url() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"env": {"ANTHROPIC_AUTH_TOKEN": "sk-ant-test"}}"#,
        )
        .unwrap();

        let result = scan_claude_config_in(tmp.path());
        assert!(result.is_some());
        let config = result.unwrap();
        assert_eq!(config.base_url, "https://api.anthropic.com");
        assert!(config.has_api_key);
    }

    // --- Codex config tests ---

    #[test]
    fn test_scan_codex_valid_config() {
        let tmp = TempDir::new().unwrap();
        let codex_dir = tmp.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("auth.json"),
            r#"{"OPENAI_API_KEY": "sk-openai-test-key-67890"}"#,
        )
        .unwrap();
        fs::write(
            codex_dir.join("config.toml"),
            r#"base_url = "https://custom.openai.com/v1""#,
        )
        .unwrap();

        let result = scan_codex_config_in(tmp.path());
        assert!(result.is_some());
        let config = result.unwrap();
        assert_eq!(config.cli_id, "codex");
        assert_eq!(config.cli_name, "Codex");
        assert_eq!(config.api_key, "sk-openai-test-key-67890");
        assert_eq!(config.base_url, "https://custom.openai.com/v1");
        assert_eq!(config.protocol_type, ProtocolType::OpenAiChatCompletions);
        assert!(config.has_api_key);
    }

    #[test]
    fn test_scan_codex_provider_scoped_base_url() {
        let tmp = TempDir::new().unwrap();
        let codex_dir = tmp.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("auth.json"),
            r#"{"OPENAI_API_KEY": "sk-test"}"#,
        )
        .unwrap();
        fs::write(
            codex_dir.join("config.toml"),
            r#"
model_provider = "azure"
base_url = "https://fallback.example.com"

[model_providers.azure]
base_url = "https://azure.openai.com/v1"
"#,
        )
        .unwrap();

        let result = scan_codex_config_in(tmp.path());
        assert!(result.is_some());
        let config = result.unwrap();
        assert_eq!(config.base_url, "https://azure.openai.com/v1");
    }

    #[test]
    fn test_scan_codex_missing_files() {
        let tmp = TempDir::new().unwrap();
        // No .codex directory at all
        let result = scan_codex_config_in(tmp.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_scan_codex_partial_config() {
        let tmp = TempDir::new().unwrap();
        let codex_dir = tmp.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        // Only auth.json, no config.toml
        fs::write(
            codex_dir.join("auth.json"),
            r#"{"OPENAI_API_KEY": "sk-partial-test"}"#,
        )
        .unwrap();

        let result = scan_codex_config_in(tmp.path());
        assert!(result.is_some());
        let config = result.unwrap();
        assert_eq!(config.api_key, "sk-partial-test");
        assert_eq!(config.base_url, "");
        assert!(config.has_api_key);
    }

    #[test]
    fn test_scan_codex_corrupted_toml() {
        let tmp = TempDir::new().unwrap();
        let codex_dir = tmp.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("auth.json"),
            r#"{"OPENAI_API_KEY": "sk-valid"}"#,
        )
        .unwrap();
        fs::write(codex_dir.join("config.toml"), "not valid toml [[[").unwrap();

        let result = scan_codex_config_in(tmp.path());
        assert!(result.is_some());
        let config = result.unwrap();
        // Still returns result from auth.json even though TOML is corrupted
        assert_eq!(config.api_key, "sk-valid");
        assert_eq!(config.base_url, "");
    }

    #[test]
    fn test_scan_codex_corrupted_auth_without_valid_config() {
        let tmp = TempDir::new().unwrap();
        let codex_dir = tmp.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(codex_dir.join("auth.json"), "{ invalid json }").unwrap();

        let result = scan_codex_config_in(tmp.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_scan_codex_corrupted_auth_and_config() {
        let tmp = TempDir::new().unwrap();
        let codex_dir = tmp.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(codex_dir.join("auth.json"), "{ invalid json }").unwrap();
        fs::write(codex_dir.join("config.toml"), "not valid toml [[[").unwrap();

        let result = scan_codex_config_in(tmp.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_scan_codex_missing_api_key() {
        let tmp = TempDir::new().unwrap();
        let codex_dir = tmp.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(codex_dir.join("auth.json"), r#"{}"#).unwrap();

        let result = scan_codex_config_in(tmp.path());
        assert!(result.is_some());
        let config = result.unwrap();
        assert!(!config.has_api_key);
        assert_eq!(config.api_key, "");
    }

    // --- import_provider tests ---

    #[test]
    fn test_import_provider_with_empty_api_key() {
        let tmp = TempDir::new().unwrap();
        let result = import_provider_to(
            tmp.path(),
            "Claude Import".to_string(),
            ProtocolType::Anthropic,
            "".to_string(),
            "https://api.anthropic.com".to_string(),
            "claude".to_string(),
        );
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_import_provider_with_empty_base_url() {
        let tmp = TempDir::new().unwrap();
        let result = import_provider_to(
            tmp.path(),
            "Codex Import".to_string(),
            ProtocolType::OpenAiChatCompletions,
            "sk-test-key".to_string(),
            "".to_string(),
            "codex".to_string(),
        );
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_import_provider_rejects_invalid_base_url_scheme() {
        let tmp = TempDir::new().unwrap();
        let result = import_provider_to(
            tmp.path(),
            "Codex Import".to_string(),
            ProtocolType::OpenAiChatCompletions,
            "sk-test-key".to_string(),
            "example.com/v1".to_string(),
            "codex".to_string(),
        );
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_import_provider_rejects_anthropic_base_url_with_path() {
        let tmp = TempDir::new().unwrap();
        let result = import_provider_to(
            tmp.path(),
            "Claude Import".to_string(),
            ProtocolType::Anthropic,
            "sk-test-key".to_string(),
            "https://api.openai.com/v1".to_string(),
            "claude".to_string(),
        );
        assert!(matches!(
            result,
            Err(AppError::Validation(ref message))
                if message == "Provider base URL must not contain a path"
        ));
    }

    #[test]
    fn test_import_provider_allows_openai_base_url_with_path() {
        let tmp = TempDir::new().unwrap();
        let provider = import_provider_to(
            tmp.path(),
            "Codex Import".to_string(),
            ProtocolType::OpenAiChatCompletions,
            "sk-test-key".to_string(),
            "https://gateway.example.com/openai/v1/".to_string(),
            "codex".to_string(),
        )
        .unwrap();

        assert_eq!(provider.base_url, "https://gateway.example.com/openai/v1");
    }

    #[test]
    fn test_import_provider_normalizes_trailing_slash() {
        let tmp = TempDir::new().unwrap();
        let provider = import_provider_to(
            tmp.path(),
            "Slash Import".to_string(),
            ProtocolType::Anthropic,
            "sk-ant-full-key".to_string(),
            "https://custom.api.com/".to_string(),
            "claude".to_string(),
        )
        .unwrap();

        assert_eq!(provider.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_import_provider_with_full_data() {
        let tmp = TempDir::new().unwrap();
        let result = import_provider_to(
            tmp.path(),
            "Full Import".to_string(),
            ProtocolType::Anthropic,
            "sk-ant-full-key".to_string(),
            "https://custom.api.com".to_string(),
            "claude".to_string(),
        );
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.name, "Full Import");
        assert_eq!(provider.api_key, "sk-ant-full-key");
        assert_eq!(provider.base_url, "https://custom.api.com");
        assert_eq!(provider.protocol_type, ProtocolType::Anthropic);
        assert_eq!(provider.cli_id, "claude");
        // Verify saved content is correct
        let file_path = tmp.path().join(format!("{}.json", provider.id));
        let content = fs::read_to_string(&file_path).unwrap();
        let saved: crate::provider::Provider = serde_json::from_str(&content).unwrap();
        assert_eq!(saved.name, "Full Import");
        assert_eq!(saved.api_key, "sk-ant-full-key");
        assert_eq!(saved.test_model.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(saved.upstream_model, None);
    }

    #[test]
    fn test_import_openai_provider_sets_default_test_and_upstream_model() {
        let tmp = TempDir::new().unwrap();
        let provider = import_provider_to(
            tmp.path(),
            "OpenAI Import".to_string(),
            ProtocolType::OpenAiChatCompletions,
            "sk-openai-key".to_string(),
            "https://api.openai.com".to_string(),
            "codex".to_string(),
        )
        .unwrap();

        assert_eq!(provider.test_model.as_deref(), Some("gpt-5.2"));
        assert_eq!(provider.upstream_model.as_deref(), Some("gpt-5.2"));
    }

    #[test]
    fn test_import_provider_rejects_empty_name() {
        let tmp = TempDir::new().unwrap();
        let result = import_provider_to(
            tmp.path(),
            "".to_string(),
            ProtocolType::Anthropic,
            "sk-test".to_string(),
            "https://api.anthropic.com".to_string(),
            "claude".to_string(),
        );
        assert!(result.is_err());
    }
}
