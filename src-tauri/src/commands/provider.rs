use std::path::Path;
use std::time::Instant;

use serde::Serialize;

use crate::adapter::claude::ClaudeAdapter;
use crate::adapter::codex::CodexAdapter;
use crate::adapter::CliAdapter;
use crate::error::AppError;
use crate::provider::{Provider, ProtocolType};
use crate::storage::local::{LocalSettings, read_local_settings, write_local_settings};

#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub success: bool,
    pub elapsed_ms: u64,
    pub error: Option<String>,
}

// --- Internal helpers for testability ---

fn get_adapter_for_cli(
    cli_id: &str,
    settings: &LocalSettings,
) -> Result<Box<dyn CliAdapter>, AppError> {
    match cli_id {
        "claude" => {
            if let Some(ref dir) = settings.cli_paths.claude_config_dir {
                let home = dirs::home_dir().expect("home directory required");
                Ok(Box::new(ClaudeAdapter::new_with_paths(
                    dir.into(),
                    home.join(".cli-manager").join("backups").join("claude"),
                )))
            } else {
                Ok(Box::new(ClaudeAdapter::new()))
            }
        }
        "codex" => {
            if let Some(ref dir) = settings.cli_paths.codex_config_dir {
                let home = dirs::home_dir().expect("home directory required");
                Ok(Box::new(CodexAdapter::new_with_paths(
                    dir.into(),
                    home.join(".cli-manager").join("backups").join("codex"),
                )))
            } else {
                Ok(Box::new(CodexAdapter::new()))
            }
        }
        _ => Err(AppError::Validation(format!(
            "Unknown CLI: {}",
            cli_id
        ))),
    }
}

/// Internal: list and filter providers
fn _list_providers_in(
    providers_dir: &Path,
    cli_id: Option<String>,
) -> Result<Vec<Provider>, AppError> {
    let all = crate::storage::icloud::list_providers_in(providers_dir)?;
    match cli_id {
        Some(id) => Ok(all.into_iter().filter(|p| p.cli_id == id).collect()),
        None => Ok(all),
    }
}

/// Internal: set active provider with injectable adapter
fn _set_active_provider_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    cli_id: String,
    provider_id: Option<String>,
    adapter: Option<Box<dyn CliAdapter>>,
) -> Result<LocalSettings, AppError> {
    let mut settings =
        crate::storage::local::read_local_settings_from(local_settings_path)?;
    settings
        .active_providers
        .insert(cli_id.clone(), provider_id.clone());
    crate::storage::local::write_local_settings_to(local_settings_path, &settings)?;

    if let Some(pid) = provider_id {
        let provider = crate::storage::icloud::get_provider_in(providers_dir, &pid)?;
        if let Some(adapter) = adapter {
            adapter.patch(&provider)?;
        } else {
            let real_adapter = get_adapter_for_cli(&cli_id, &settings)?;
            real_adapter.patch(&provider)?;
        }
    }

    Ok(settings)
}

/// Internal: delete provider with auto-switch logic
fn _delete_provider_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    id: String,
    adapter: Option<Box<dyn CliAdapter>>,
) -> Result<(), AppError> {
    // Get the provider before deleting to know its cli_id
    let provider = crate::storage::icloud::get_provider_in(providers_dir, &id)?;
    let cli_id = provider.cli_id.clone();

    // Get current settings to check if this is the active provider
    let settings =
        crate::storage::local::read_local_settings_from(local_settings_path)?;
    let is_active = settings
        .active_providers
        .get(&cli_id)
        .map_or(false, |active| active.as_deref() == Some(&id));

    // Delete the provider file
    crate::storage::icloud::delete_provider_in(providers_dir, &id)?;

    // Auto-switch if we just deleted the active provider
    if is_active {
        let remaining: Vec<Provider> = crate::storage::icloud::list_providers_in(providers_dir)?
            .into_iter()
            .filter(|p| p.cli_id == cli_id)
            .collect();

        if remaining.is_empty() {
            // No more providers for this CLI, clear active
            _set_active_provider_in(
                providers_dir,
                local_settings_path,
                cli_id,
                None,
                None,
            )?;
        } else {
            // Pick the first available provider (circular search)
            let next = &remaining[0];
            _set_active_provider_in(
                providers_dir,
                local_settings_path,
                cli_id,
                Some(next.id.clone()),
                adapter,
            )?;
        }
    }

    Ok(())
}

// --- Tauri command wrappers ---

#[tauri::command]
pub fn list_providers(cli_id: Option<String>) -> Result<Vec<Provider>, AppError> {
    let dir = crate::storage::icloud::get_icloud_providers_dir()?;
    _list_providers_in(&dir, cli_id)
}

#[tauri::command]
pub fn get_provider(id: String) -> Result<Provider, AppError> {
    crate::storage::icloud::get_provider(&id)
}

#[tauri::command]
pub fn create_provider(
    name: String,
    protocol_type: ProtocolType,
    api_key: String,
    base_url: String,
    model: String,
    cli_id: String,
) -> Result<Provider, AppError> {
    let provider = Provider::new(name, protocol_type, api_key, base_url, model, cli_id);
    crate::storage::icloud::save_provider(&provider)?;
    Ok(provider)
}

#[tauri::command]
pub fn update_provider(mut provider: Provider) -> Result<Provider, AppError> {
    provider.updated_at = chrono::Utc::now().timestamp_millis();
    crate::storage::icloud::save_existing_provider(&provider)?;
    Ok(provider)
}

#[tauri::command]
pub fn delete_provider(id: String) -> Result<(), AppError> {
    let dir = crate::storage::icloud::get_icloud_providers_dir()?;
    let settings_path = crate::storage::local::get_local_settings_path();
    _delete_provider_in(&dir, &settings_path, id, None)
}

#[tauri::command]
pub fn get_local_settings() -> Result<LocalSettings, AppError> {
    read_local_settings()
}

#[tauri::command]
pub fn set_active_provider(
    cli_id: String,
    provider_id: Option<String>,
) -> Result<LocalSettings, AppError> {
    let dir = crate::storage::icloud::get_icloud_providers_dir()?;
    let settings_path = crate::storage::local::get_local_settings_path();
    _set_active_provider_in(&dir, &settings_path, cli_id, provider_id, None)
}

#[tauri::command]
pub fn update_local_settings(settings: LocalSettings) -> Result<LocalSettings, AppError> {
    write_local_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub async fn test_provider(provider_id: String) -> Result<TestResult, AppError> {
    let provider = crate::storage::icloud::get_provider(&provider_id)?;

    // Read timeout from settings
    let settings = read_local_settings().unwrap_or_default();
    let timeout_secs = settings
        .test_config
        .as_ref()
        .map(|c| c.timeout_secs)
        .unwrap_or(10);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs as u64))
        .build()
        .map_err(|e| AppError::Http(e.to_string()))?;

    let start = Instant::now();

    let result = match provider.protocol_type {
        ProtocolType::Anthropic => {
            let url = format!("{}/v1/messages", provider.base_url.trim_end_matches('/'));
            client
                .post(&url)
                .header("x-api-key", &provider.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&serde_json::json!({
                    "model": provider.model,
                    "messages": [{"role": "user", "content": "hi"}],
                    "max_tokens": 1
                }))
                .send()
                .await
        }
        ProtocolType::OpenAiCompatible => {
            let url = format!(
                "{}/v1/chat/completions",
                provider.base_url.trim_end_matches('/')
            );
            client
                .post(&url)
                .header("Authorization", format!("Bearer {}", provider.api_key))
                .header("content-type", "application/json")
                .json(&serde_json::json!({
                    "model": provider.model,
                    "messages": [{"role": "user", "content": "hi"}],
                    "max_tokens": 1
                }))
                .send()
                .await
        }
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                Ok(TestResult {
                    success: true,
                    elapsed_ms,
                    error: None,
                })
            } else {
                let body = resp.text().await.unwrap_or_default();
                Ok(TestResult {
                    success: false,
                    elapsed_ms,
                    error: Some(format!("HTTP {}: {}", status.as_u16(), body)),
                })
            }
        }
        Err(e) => Ok(TestResult {
            success: false,
            elapsed_ms,
            error: Some(e.to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProtocolType;
    use crate::storage::icloud::{list_providers_in, save_provider_to};
    use crate::storage::local::{read_local_settings_from, write_local_settings_to};
    use tempfile::TempDir;

    fn make_provider(id: &str, name: &str, cli_id: &str) -> Provider {
        Provider {
            id: id.to_string(),
            cli_id: cli_id.to_string(),
            name: name.to_string(),
            protocol_type: ProtocolType::Anthropic,
            api_key: "sk-ant-test".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            model_config: None,
            notes: None,
            created_at: 1710000000000,
            updated_at: 1710000000000,
            schema_version: 1,
        }
    }

    #[test]
    fn test_list_providers_filters_by_cli_id() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        save_provider_to(dir, &make_provider("p1", "Claude Provider", "claude")).unwrap();
        save_provider_to(dir, &make_provider("p2", "Codex Provider", "codex")).unwrap();
        save_provider_to(dir, &make_provider("p3", "Another Claude", "claude")).unwrap();

        let claude_only = _list_providers_in(dir, Some("claude".to_string())).unwrap();
        assert_eq!(claude_only.len(), 2);
        assert!(claude_only.iter().all(|p| p.cli_id == "claude"));

        let codex_only = _list_providers_in(dir, Some("codex".to_string())).unwrap();
        assert_eq!(codex_only.len(), 1);
        assert_eq!(codex_only[0].id, "p2");
    }

    #[test]
    fn test_list_providers_none_returns_all() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        save_provider_to(dir, &make_provider("p1", "Claude Provider", "claude")).unwrap();
        save_provider_to(dir, &make_provider("p2", "Codex Provider", "codex")).unwrap();

        let all = _list_providers_in(dir, None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_create_provider_with_cli_id() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        let provider = Provider::new(
            "Test".to_string(),
            ProtocolType::Anthropic,
            "sk-test".to_string(),
            "https://api.anthropic.com".to_string(),
            "claude-sonnet-4-20250514".to_string(),
            "codex".to_string(),
        );
        save_provider_to(dir, &provider).unwrap();

        let loaded = crate::storage::icloud::get_provider_in(dir, &provider.id).unwrap();
        assert_eq!(loaded.cli_id, "codex");
    }

    #[test]
    fn test_set_active_provider_updates_active_providers_map() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        // Create a provider
        let provider = make_provider("p1", "Test Provider", "claude");
        save_provider_to(&providers_dir, &provider).unwrap();

        // Create a mock adapter (use ClaudeAdapter with temp paths so patch goes to temp dir)
        let adapter_config_dir = tmp.path().join("claude-config");
        let adapter_backup_dir = tmp.path().join("claude-backup");
        std::fs::create_dir_all(&adapter_config_dir).unwrap();
        // Write a minimal settings.json so patch succeeds
        std::fs::write(adapter_config_dir.join("settings.json"), "{}").unwrap();
        let adapter: Box<dyn CliAdapter> = Box::new(ClaudeAdapter::new_with_paths(
            adapter_config_dir.clone(),
            adapter_backup_dir,
        ));

        let settings = _set_active_provider_in(
            &providers_dir,
            &settings_path,
            "claude".to_string(),
            Some("p1".to_string()),
            Some(adapter),
        )
        .unwrap();

        assert_eq!(
            settings.active_providers.get("claude"),
            Some(&Some("p1".to_string()))
        );

        // Verify patch was called: settings.json should have env fields
        let patched: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(adapter_config_dir.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-test");
    }

    #[test]
    fn test_set_active_provider_codex_updates_and_patches() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        let mut provider = make_provider("p2", "Codex Provider", "codex");
        provider.protocol_type = ProtocolType::OpenAiCompatible;
        provider.api_key = "sk-codex-key".to_string();
        provider.base_url = "https://proxy.example.com/v1".to_string();
        save_provider_to(&providers_dir, &provider).unwrap();

        let adapter_config_dir = tmp.path().join("codex-config");
        let adapter_backup_dir = tmp.path().join("codex-backup");
        std::fs::create_dir_all(&adapter_config_dir).unwrap();
        std::fs::write(adapter_config_dir.join("auth.json"), "{}").unwrap();
        let adapter: Box<dyn CliAdapter> = Box::new(CodexAdapter::new_with_paths(
            adapter_config_dir.clone(),
            adapter_backup_dir,
        ));

        let settings = _set_active_provider_in(
            &providers_dir,
            &settings_path,
            "codex".to_string(),
            Some("p2".to_string()),
            Some(adapter),
        )
        .unwrap();

        assert_eq!(
            settings.active_providers.get("codex"),
            Some(&Some("p2".to_string()))
        );

        // Verify codex auth.json was patched
        let patched: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(adapter_config_dir.join("auth.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(patched["OPENAI_API_KEY"], "sk-codex-key");
    }

    #[test]
    fn test_set_active_provider_none_clears_without_patch() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        // First set an active provider
        let mut initial = LocalSettings::default();
        initial
            .active_providers
            .insert("claude".to_string(), Some("old-id".to_string()));
        write_local_settings_to(&settings_path, &initial).unwrap();

        // Set to None (should not call patch)
        let settings = _set_active_provider_in(
            &providers_dir,
            &settings_path,
            "claude".to_string(),
            None,
            None, // No adapter needed since provider_id is None
        )
        .unwrap();

        assert_eq!(
            settings.active_providers.get("claude"),
            Some(&None)
        );
    }

    #[test]
    fn test_delete_active_provider_auto_switches_to_next() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        // Create 3 claude providers
        let p1 = make_provider("p1", "Provider 1", "claude");
        let p2 = make_provider("p2", "Provider 2", "claude");
        let p3 = make_provider("p3", "Provider 3", "claude");
        save_provider_to(&providers_dir, &p1).unwrap();
        save_provider_to(&providers_dir, &p2).unwrap();
        save_provider_to(&providers_dir, &p3).unwrap();

        // Set p2 as active
        let mut settings = LocalSettings::default();
        settings
            .active_providers
            .insert("claude".to_string(), Some("p2".to_string()));
        write_local_settings_to(&settings_path, &settings).unwrap();

        // Create a mock adapter for the auto-switch target
        let adapter_config_dir = tmp.path().join("claude-config");
        let adapter_backup_dir = tmp.path().join("claude-backup");
        std::fs::create_dir_all(&adapter_config_dir).unwrap();
        std::fs::write(adapter_config_dir.join("settings.json"), "{}").unwrap();
        let adapter: Box<dyn CliAdapter> = Box::new(ClaudeAdapter::new_with_paths(
            adapter_config_dir,
            adapter_backup_dir,
        ));

        // Delete p2 (the active one)
        _delete_provider_in(&providers_dir, &settings_path, "p2".to_string(), Some(adapter))
            .unwrap();

        // Should auto-switch to next available (p1 since sorted by created_at)
        let loaded = read_local_settings_from(&settings_path).unwrap();
        let active = loaded.active_providers.get("claude").unwrap();
        assert!(active.is_some(), "Should have auto-switched to another provider");
        // The active should be one of the remaining providers
        let active_id = active.as_ref().unwrap();
        assert!(active_id == "p1" || active_id == "p3");
    }

    #[test]
    fn test_delete_non_active_provider_does_not_change_active() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        let p1 = make_provider("p1", "Provider 1", "claude");
        let p2 = make_provider("p2", "Provider 2", "claude");
        save_provider_to(&providers_dir, &p1).unwrap();
        save_provider_to(&providers_dir, &p2).unwrap();

        // Set p1 as active
        let mut settings = LocalSettings::default();
        settings
            .active_providers
            .insert("claude".to_string(), Some("p1".to_string()));
        write_local_settings_to(&settings_path, &settings).unwrap();

        // Delete p2 (not active)
        _delete_provider_in(&providers_dir, &settings_path, "p2".to_string(), None).unwrap();

        // Active should still be p1
        let loaded = read_local_settings_from(&settings_path).unwrap();
        assert_eq!(
            loaded.active_providers.get("claude"),
            Some(&Some("p1".to_string()))
        );
    }

    #[test]
    fn test_delete_last_provider_clears_active() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        let p1 = make_provider("p1", "Only Provider", "claude");
        save_provider_to(&providers_dir, &p1).unwrap();

        // Set p1 as active
        let mut settings = LocalSettings::default();
        settings
            .active_providers
            .insert("claude".to_string(), Some("p1".to_string()));
        write_local_settings_to(&settings_path, &settings).unwrap();

        // Delete p1 (the only one)
        _delete_provider_in(&providers_dir, &settings_path, "p1".to_string(), None).unwrap();

        // Active should be cleared
        let loaded = read_local_settings_from(&settings_path).unwrap();
        assert_eq!(loaded.active_providers.get("claude"), Some(&None));
    }

    #[test]
    fn test_update_local_settings_persists() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("local.json");

        let mut settings = LocalSettings::default();
        settings.language = Some("zh-CN".to_string());
        settings.test_config = Some(crate::storage::local::TestConfig {
            timeout_secs: 30,
            test_model: Some("claude-haiku-4-20250514".to_string()),
        });

        write_local_settings_to(&path, &settings).unwrap();
        let loaded = read_local_settings_from(&path).unwrap();
        assert_eq!(loaded.language, Some("zh-CN".to_string()));
        assert_eq!(loaded.test_config.as_ref().unwrap().timeout_secs, 30);
        assert_eq!(
            loaded.test_config.as_ref().unwrap().test_model,
            Some("claude-haiku-4-20250514".to_string())
        );
    }
}
