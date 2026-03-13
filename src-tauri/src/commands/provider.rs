use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use serde::Serialize;
use tauri::Manager;

use crate::adapter::claude::ClaudeAdapter;
use crate::adapter::codex::CodexAdapter;
use crate::adapter::CliAdapter;
use crate::error::AppError;
use crate::provider::{ProtocolType, Provider};
use crate::storage::local::{read_local_settings, write_local_settings, LocalSettings};

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
        _ => Err(AppError::Validation(format!("Unknown CLI: {}", cli_id))),
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

pub(crate) fn normalize_provider_fields(provider: &mut Provider) {
    provider.name = provider.name.trim().to_string();
    provider.api_key = provider.api_key.trim().to_string();
    provider.base_url = provider.base_url.trim().to_string();
    provider.model = provider.model.trim().to_string();
    provider.cli_id = provider.cli_id.trim().to_string();
}

pub(crate) fn validate_provider(provider: &Provider) -> Result<(), AppError> {
    if provider.name.is_empty() {
        return Err(AppError::Validation(
            "Provider name cannot be empty".to_string(),
        ));
    }

    if provider.api_key.is_empty() {
        return Err(AppError::Validation(
            "Provider API key cannot be empty".to_string(),
        ));
    }

    if provider.base_url.is_empty() {
        return Err(AppError::Validation(
            "Provider base URL cannot be empty".to_string(),
        ));
    }

    if !provider.base_url.starts_with("http://") && !provider.base_url.starts_with("https://") {
        return Err(AppError::Validation(
            "Provider base URL must start with http:// or https://".to_string(),
        ));
    }

    Ok(())
}

fn normalize_and_validate_provider(mut provider: Provider) -> Result<Provider, AppError> {
    normalize_provider_fields(&mut provider);
    validate_provider(&provider)?;
    Ok(provider)
}

fn patch_provider_for_cli(
    cli_id: &str,
    settings: &LocalSettings,
    provider: &Provider,
    adapter: Option<Box<dyn CliAdapter>>,
) -> Result<(), AppError> {
    let provider = normalize_and_validate_provider(provider.clone())?;

    if let Some(adapter) = adapter {
        adapter.patch(&provider)?;
    } else {
        let real_adapter = get_adapter_for_cli(cli_id, settings)?;
        real_adapter.patch(&provider)?;
    }

    Ok(())
}

fn clear_provider_for_cli(
    cli_id: &str,
    settings: &LocalSettings,
    adapter: Option<Box<dyn CliAdapter>>,
) -> Result<(), AppError> {
    if let Some(adapter) = adapter {
        adapter.clear()?;
    } else {
        let real_adapter = get_adapter_for_cli(cli_id, settings)?;
        real_adapter.clear()?;
    }

    Ok(())
}

/// Internal: set active provider with injectable adapter
pub(crate) fn _set_active_provider_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    cli_id: String,
    provider_id: Option<String>,
    adapter: Option<Box<dyn CliAdapter>>,
) -> Result<LocalSettings, AppError> {
    let mut settings = crate::storage::local::read_local_settings_from(local_settings_path)?;

    if let Some(pid) = provider_id.as_deref() {
        let provider = crate::storage::icloud::get_provider_in(providers_dir, &pid)?;
        patch_provider_for_cli(&cli_id, &settings, &provider, adapter)?;
    }

    settings
        .active_providers
        .insert(cli_id.clone(), provider_id.clone());
    crate::storage::local::write_local_settings_to(local_settings_path, &settings)?;

    Ok(settings)
}

fn _clear_active_provider_in(
    local_settings_path: &Path,
    cli_id: String,
    adapter: Option<Box<dyn CliAdapter>>,
) -> Result<LocalSettings, AppError> {
    let mut settings = crate::storage::local::read_local_settings_from(local_settings_path)?;

    clear_provider_for_cli(&cli_id, &settings, adapter)?;

    settings.active_providers.insert(cli_id, None);
    crate::storage::local::write_local_settings_to(local_settings_path, &settings)?;

    Ok(settings)
}

fn _reconcile_missing_active_provider_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    cli_id: String,
    missing_provider_id: String,
    adapter: Option<Box<dyn CliAdapter>>,
) -> Result<(), AppError> {
    let remaining: Vec<Provider> = crate::storage::icloud::list_providers_in(providers_dir)?
        .into_iter()
        .filter(|provider| provider.cli_id == cli_id && provider.id != missing_provider_id)
        .collect();

    if let Some(next) = remaining.first() {
        _set_active_provider_in(
            providers_dir,
            local_settings_path,
            cli_id,
            Some(next.id.clone()),
            adapter,
        )?;
    } else {
        _clear_active_provider_in(local_settings_path, cli_id, adapter)?;
    }

    Ok(())
}

fn _reconcile_active_providers_in_with_adapter(
    providers_dir: &Path,
    local_settings_path: &Path,
    changed_provider_ids: Option<&[String]>,
    mut adapter: Option<Box<dyn CliAdapter>>,
) -> Result<bool, AppError> {
    let changed: Option<HashSet<&str>> =
        changed_provider_ids.map(|ids| ids.iter().map(String::as_str).collect());
    let settings = crate::storage::local::read_local_settings_from(local_settings_path)?;

    let active_targets: Vec<(String, String)> = settings
        .active_providers
        .iter()
        .filter_map(|(cli_id, provider_id)| {
            let provider_id = provider_id.as_deref()?;
            if changed
                .as_ref()
                .is_some_and(|changed| !changed.contains(provider_id))
            {
                return None;
            }

            Some((cli_id.clone(), provider_id.to_string()))
        })
        .collect();

    if active_targets.is_empty() {
        return Ok(false);
    }

    for (cli_id, provider_id) in &active_targets {
        match crate::storage::icloud::get_provider_in(providers_dir, provider_id) {
            Ok(provider) => {
                let settings =
                    crate::storage::local::read_local_settings_from(local_settings_path)?;
                patch_provider_for_cli(cli_id, &settings, &provider, adapter.take())?;
            }
            Err(AppError::NotFound(_)) | Err(AppError::Json(_)) => {
                _reconcile_missing_active_provider_in(
                    providers_dir,
                    local_settings_path,
                    cli_id.clone(),
                    provider_id.clone(),
                    adapter.take(),
                )?;
            }
            Err(err) => return Err(err),
        }
    }

    Ok(true)
}

fn _reconcile_active_providers_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    changed_provider_ids: Option<&[String]>,
) -> Result<bool, AppError> {
    _reconcile_active_providers_in_with_adapter(
        providers_dir,
        local_settings_path,
        changed_provider_ids,
        None,
    )
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
    let settings = crate::storage::local::read_local_settings_from(local_settings_path)?;
    let is_active = settings
        .active_providers
        .get(&cli_id)
        .map_or(false, |active| active.as_deref() == Some(&id));

    if is_active {
        let remaining: Vec<Provider> = crate::storage::icloud::list_providers_in(providers_dir)?
            .into_iter()
            .filter(|p| p.cli_id == cli_id && p.id != id)
            .collect();

        if let Some(next) = remaining.first() {
            _set_active_provider_in(
                providers_dir,
                local_settings_path,
                cli_id.clone(),
                Some(next.id.clone()),
                adapter,
            )?;
        }
    }

    // Delete the provider file
    crate::storage::icloud::delete_provider_in(providers_dir, &id)?;

    if is_active
        && crate::storage::icloud::list_providers_in(providers_dir)?
            .into_iter()
            .all(|p| p.cli_id != cli_id)
    {
        _set_active_provider_in(providers_dir, local_settings_path, cli_id, None, None)?;
    }

    Ok(())
}

fn _update_provider_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    mut provider: Provider,
    adapter: Option<Box<dyn CliAdapter>>,
) -> Result<Provider, AppError> {
    provider = normalize_and_validate_provider(provider)?;
    let existing = crate::storage::icloud::get_provider_in(providers_dir, &provider.id)?;
    let settings = crate::storage::local::read_local_settings_from(local_settings_path)?;
    let is_active = settings
        .active_providers
        .get(&provider.cli_id)
        .map_or(false, |active| {
            active.as_deref() == Some(provider.id.as_str())
        });

    provider.updated_at = chrono::Utc::now().timestamp_millis();
    crate::storage::icloud::save_existing_provider_to(providers_dir, &provider)?;

    if is_active {
        if let Err(err) = patch_provider_for_cli(&provider.cli_id, &settings, &provider, adapter) {
            let _ = crate::storage::icloud::save_existing_provider_to(providers_dir, &existing);
            return Err(err);
        }
    }

    Ok(provider)
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
    app_handle: tauri::AppHandle,
    name: String,
    protocol_type: ProtocolType,
    api_key: String,
    base_url: String,
    model: String,
    cli_id: String,
) -> Result<Provider, AppError> {
    let provider = normalize_and_validate_provider(Provider::new(
        name,
        protocol_type,
        api_key,
        base_url,
        model,
        cli_id,
    ))?;

    // Record self-write BEFORE the file operation so the watcher ignores this change
    let dir = crate::storage::icloud::get_icloud_providers_dir()?;
    let tracker = app_handle.state::<crate::watcher::SelfWriteTracker>();
    tracker.record_write(dir.join(format!("{}.json", provider.id)));

    crate::storage::icloud::save_provider(&provider)?;
    Ok(provider)
}

#[tauri::command]
pub fn update_provider(
    app_handle: tauri::AppHandle,
    provider: Provider,
) -> Result<Provider, AppError> {
    let provider = normalize_and_validate_provider(provider)?;
    let dir = crate::storage::icloud::get_icloud_providers_dir()?;
    let settings_path = crate::storage::local::get_local_settings_path();

    // Record self-write BEFORE the file operation so the watcher ignores this change
    let tracker = app_handle.state::<crate::watcher::SelfWriteTracker>();
    tracker.record_write(dir.join(format!("{}.json", provider.id)));

    let result = _update_provider_in(&dir, &settings_path, provider, None)?;
    Ok(result)
}

#[tauri::command]
pub fn delete_provider(app_handle: tauri::AppHandle, id: String) -> Result<(), AppError> {
    let dir = crate::storage::icloud::get_icloud_providers_dir()?;
    let settings_path = crate::storage::local::get_local_settings_path();

    // Record self-write before deletion so the file watcher ignores this change
    let tracker = app_handle.state::<crate::watcher::SelfWriteTracker>();
    tracker.record_write(dir.join(format!("{}.json", id)));

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

pub fn sync_changed_active_providers(changed_provider_ids: &[String]) -> Result<bool, AppError> {
    let dir = crate::storage::icloud::get_icloud_providers_dir()?;
    let settings_path = crate::storage::local::get_local_settings_path();
    _reconcile_active_providers_in(&dir, &settings_path, Some(changed_provider_ids))
}

#[tauri::command]
pub fn sync_active_providers(app: tauri::AppHandle) -> Result<(), AppError> {
    let dir = crate::storage::icloud::get_icloud_providers_dir()?;
    let settings_path = crate::storage::local::get_local_settings_path();
    _reconcile_active_providers_in(&dir, &settings_path, None)?;
    #[cfg(desktop)]
    crate::tray::update_tray_menu(&app);
    Ok(())
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

    // Use test model from settings if available, otherwise fall back to provider's model
    let model = settings
        .test_config
        .as_ref()
        .and_then(|c| c.test_model.as_ref())
        .filter(|m| !m.is_empty())
        .cloned()
        .unwrap_or_else(|| provider.model.clone());

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
                    "model": model,
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
                    "model": model,
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

/// Tauri command: rebuild the tray menu from current storage state.
/// Called by the frontend after provider CRUD operations and language changes.
#[tauri::command]
pub fn refresh_tray_menu(app: tauri::AppHandle) {
    #[cfg(desktop)]
    crate::tray::update_tray_menu(&app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::PatchResult;
    use crate::provider::ProtocolType;
    use crate::storage::icloud::{get_provider_in, save_provider_to};
    use crate::storage::local::{read_local_settings_from, write_local_settings_to, CliPaths};
    use tempfile::TempDir;

    struct FailingAdapter;

    impl CliAdapter for FailingAdapter {
        fn cli_name(&self) -> &str {
            "failing"
        }

        fn patch(&self, _provider: &Provider) -> Result<PatchResult, AppError> {
            Err(AppError::Validation("patch failed".to_string()))
        }

        fn clear(&self) -> Result<PatchResult, AppError> {
            Err(AppError::Validation("patch failed".to_string()))
        }
    }

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

    fn write_claude_local_settings(
        settings_path: &Path,
        active_provider_id: Option<&str>,
        config_dir: &Path,
    ) {
        let mut settings = LocalSettings {
            cli_paths: CliPaths {
                claude_config_dir: Some(config_dir.display().to_string()),
                codex_config_dir: None,
            },
            ..LocalSettings::default()
        };
        settings.active_providers.insert(
            "claude".to_string(),
            active_provider_id.map(|id| id.to_string()),
        );
        write_local_settings_to(settings_path, &settings).unwrap();
    }

    #[test]
    fn test_normalize_and_validate_provider_trims_fields() {
        let provider = Provider {
            name: "  Test Provider  ".to_string(),
            api_key: "  sk-ant-test  ".to_string(),
            base_url: "  https://api.anthropic.com  ".to_string(),
            model: "  claude-sonnet-4-20250514  ".to_string(),
            cli_id: "  claude  ".to_string(),
            ..make_provider("p1", "ignored", "ignored")
        };

        let normalized = normalize_and_validate_provider(provider).unwrap();

        assert_eq!(normalized.name, "Test Provider");
        assert_eq!(normalized.api_key, "sk-ant-test");
        assert_eq!(normalized.base_url, "https://api.anthropic.com");
        assert_eq!(normalized.model, "claude-sonnet-4-20250514");
        assert_eq!(normalized.cli_id, "claude");
    }

    #[test]
    fn test_normalize_and_validate_provider_rejects_base_url_without_scheme() {
        let provider = Provider {
            base_url: "localhost:8080".to_string(),
            ..make_provider("p1", "Test Provider", "claude")
        };

        let err = normalize_and_validate_provider(provider).unwrap_err();
        assert!(matches!(
            err,
            AppError::Validation(ref message)
                if message == "Provider base URL must start with http:// or https://"
        ));
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

        assert_eq!(settings.active_providers.get("claude"), Some(&None));
    }

    #[test]
    fn test_set_active_provider_missing_id_does_not_persist() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        let mut initial = LocalSettings::default();
        initial
            .active_providers
            .insert("claude".to_string(), Some("old-id".to_string()));
        write_local_settings_to(&settings_path, &initial).unwrap();

        let err = _set_active_provider_in(
            &providers_dir,
            &settings_path,
            "claude".to_string(),
            Some("missing".to_string()),
            None,
        )
        .unwrap_err();

        assert!(matches!(err, AppError::NotFound(ref id) if id == "missing"));

        let loaded = read_local_settings_from(&settings_path).unwrap();
        assert_eq!(
            loaded.active_providers.get("claude"),
            Some(&Some("old-id".to_string()))
        );
    }

    #[test]
    fn test_set_active_provider_patch_failure_does_not_persist() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        let provider = make_provider("p1", "Test Provider", "claude");
        save_provider_to(&providers_dir, &provider).unwrap();

        let mut initial = LocalSettings::default();
        initial
            .active_providers
            .insert("claude".to_string(), Some("old-id".to_string()));
        write_local_settings_to(&settings_path, &initial).unwrap();

        let err = _set_active_provider_in(
            &providers_dir,
            &settings_path,
            "claude".to_string(),
            Some("p1".to_string()),
            Some(Box::new(FailingAdapter)),
        )
        .unwrap_err();

        assert!(matches!(err, AppError::Validation(ref msg) if msg == "patch failed"));

        let loaded = read_local_settings_from(&settings_path).unwrap();
        assert_eq!(
            loaded.active_providers.get("claude"),
            Some(&Some("old-id".to_string()))
        );
    }

    #[test]
    fn test_update_active_provider_repatches_cli_config() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        let provider = make_provider("p1", "Test Provider", "claude");
        save_provider_to(&providers_dir, &provider).unwrap();

        let mut settings = LocalSettings::default();
        settings
            .active_providers
            .insert("claude".to_string(), Some("p1".to_string()));
        write_local_settings_to(&settings_path, &settings).unwrap();

        let adapter_config_dir = tmp.path().join("claude-config");
        let adapter_backup_dir = tmp.path().join("claude-backup");
        std::fs::create_dir_all(&adapter_config_dir).unwrap();
        std::fs::write(adapter_config_dir.join("settings.json"), "{}").unwrap();

        let mut updated = provider.clone();
        updated.api_key = "sk-ant-updated".to_string();
        updated.base_url = "https://proxy.example.com".to_string();

        _update_provider_in(
            &providers_dir,
            &settings_path,
            updated.clone(),
            Some(Box::new(ClaudeAdapter::new_with_paths(
                adapter_config_dir.clone(),
                adapter_backup_dir,
            ))),
        )
        .unwrap();

        let patched: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(adapter_config_dir.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-updated");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://proxy.example.com"
        );

        let stored = get_provider_in(&providers_dir, "p1").unwrap();
        assert_eq!(stored.api_key, "sk-ant-updated");
        assert_eq!(stored.base_url, "https://proxy.example.com");
    }

    #[test]
    fn test_update_active_provider_rolls_back_when_patch_fails() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        let provider = make_provider("p1", "Test Provider", "claude");
        save_provider_to(&providers_dir, &provider).unwrap();

        let mut settings = LocalSettings::default();
        settings
            .active_providers
            .insert("claude".to_string(), Some("p1".to_string()));
        write_local_settings_to(&settings_path, &settings).unwrap();

        let mut updated = provider.clone();
        updated.api_key = "sk-ant-updated".to_string();

        let err = _update_provider_in(
            &providers_dir,
            &settings_path,
            updated,
            Some(Box::new(FailingAdapter)),
        )
        .unwrap_err();

        assert!(matches!(err, AppError::Validation(ref msg) if msg == "patch failed"));

        let stored = get_provider_in(&providers_dir, "p1").unwrap();
        assert_eq!(stored.api_key, provider.api_key);
    }

    #[test]
    fn test_update_provider_normalizes_base_url_before_save() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        let provider = make_provider("p1", "Test Provider", "claude");
        save_provider_to(&providers_dir, &provider).unwrap();

        let mut updated = provider.clone();
        updated.base_url = "  https://proxy.example.com  ".to_string();

        let stored = _update_provider_in(&providers_dir, &settings_path, updated, None).unwrap();

        assert_eq!(stored.base_url, "https://proxy.example.com");
        assert_eq!(
            get_provider_in(&providers_dir, "p1").unwrap().base_url,
            "https://proxy.example.com"
        );
    }

    #[test]
    fn test_update_provider_rejects_invalid_base_url_without_overwriting_file() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        let provider = make_provider("p1", "Test Provider", "claude");
        save_provider_to(&providers_dir, &provider).unwrap();

        let mut updated = provider.clone();
        updated.base_url = "localhost:8080".to_string();

        let err = _update_provider_in(&providers_dir, &settings_path, updated, None).unwrap_err();
        assert!(matches!(
            err,
            AppError::Validation(ref message)
                if message == "Provider base URL must start with http:// or https://"
        ));
        assert_eq!(
            get_provider_in(&providers_dir, "p1").unwrap().base_url,
            provider.base_url
        );
    }

    #[test]
    fn test_reconcile_active_providers_skips_non_active_changes() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        let config_dir = tmp.path().join("claude-config");
        let backup_dir = tmp.path().join("claude-backup");
        std::fs::create_dir_all(&providers_dir).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();

        save_provider_to(&providers_dir, &make_provider("p1", "Active", "claude")).unwrap();
        save_provider_to(&providers_dir, &make_provider("p2", "Other", "claude")).unwrap();
        write_claude_local_settings(&settings_path, Some("p1"), &config_dir);
        std::fs::write(
            config_dir.join("settings.json"),
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"old-key","ANTHROPIC_BASE_URL":"https://old.example.com"}}"#,
        )
        .unwrap();

        let repatched = _reconcile_active_providers_in_with_adapter(
            &providers_dir,
            &settings_path,
            Some(&["p2".to_string()]),
            Some(Box::new(ClaudeAdapter::new_with_paths(
                config_dir.clone(),
                backup_dir.clone(),
            ))),
        )
        .unwrap();

        assert!(!repatched);
        let patched: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(config_dir.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "old-key");
        assert_eq!(
            read_local_settings_from(&settings_path)
                .unwrap()
                .active_providers
                .get("claude"),
            Some(&Some("p1".to_string()))
        );
        assert!(!backup_dir.exists());
    }

    #[test]
    fn test_reconcile_active_providers_switches_missing_active_provider() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        let config_dir = tmp.path().join("claude-config");
        let backup_dir = tmp.path().join("claude-backup");
        std::fs::create_dir_all(&providers_dir).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();

        let p1 = make_provider("p1", "Active", "claude");
        let mut p2 = make_provider("p2", "Fallback", "claude");
        p2.api_key = "sk-ant-fallback".to_string();
        p2.base_url = "https://fallback.example.com".to_string();
        save_provider_to(&providers_dir, &p1).unwrap();
        save_provider_to(&providers_dir, &p2).unwrap();
        write_claude_local_settings(&settings_path, Some("p1"), &config_dir);
        std::fs::write(config_dir.join("settings.json"), "{}").unwrap();
        std::fs::remove_file(providers_dir.join("p1.json")).unwrap();

        let repatched = _reconcile_active_providers_in_with_adapter(
            &providers_dir,
            &settings_path,
            None,
            Some(Box::new(ClaudeAdapter::new_with_paths(
                config_dir.clone(),
                backup_dir,
            ))),
        )
        .unwrap();

        assert!(repatched);
        assert_eq!(
            read_local_settings_from(&settings_path)
                .unwrap()
                .active_providers
                .get("claude"),
            Some(&Some("p2".to_string()))
        );
        let patched: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(config_dir.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(patched["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-fallback");
        assert_eq!(
            patched["env"]["ANTHROPIC_BASE_URL"],
            "https://fallback.example.com"
        );
    }

    #[test]
    fn test_reconcile_active_providers_clears_malformed_active_provider() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        let config_dir = tmp.path().join("claude-config");
        let backup_dir = tmp.path().join("claude-backup");
        std::fs::create_dir_all(&providers_dir).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();

        let p1 = make_provider("p1", "Active", "claude");
        save_provider_to(&providers_dir, &p1).unwrap();
        write_claude_local_settings(&settings_path, Some("p1"), &config_dir);
        std::fs::write(
            config_dir.join("settings.json"),
            r#"{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "old-key",
    "ANTHROPIC_BASE_URL": "https://old.example.com",
    "CUSTOM_VAR": "keep-me"
  }
}"#,
        )
        .unwrap();
        std::fs::write(providers_dir.join("p1.json"), "{ invalid json }").unwrap();

        let repatched = _reconcile_active_providers_in_with_adapter(
            &providers_dir,
            &settings_path,
            Some(&["p1".to_string()]),
            Some(Box::new(ClaudeAdapter::new_with_paths(
                config_dir.clone(),
                backup_dir,
            ))),
        )
        .unwrap();

        assert!(repatched);
        assert_eq!(
            read_local_settings_from(&settings_path)
                .unwrap()
                .active_providers
                .get("claude"),
            Some(&None)
        );
        let patched: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(config_dir.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert!(patched["env"]["ANTHROPIC_AUTH_TOKEN"].is_null());
        assert!(patched["env"]["ANTHROPIC_BASE_URL"].is_null());
        assert_eq!(patched["env"]["CUSTOM_VAR"], "keep-me");
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
        _delete_provider_in(
            &providers_dir,
            &settings_path,
            "p2".to_string(),
            Some(adapter),
        )
        .unwrap();

        // Should auto-switch to next available (p1 since sorted by created_at)
        let loaded = read_local_settings_from(&settings_path).unwrap();
        let active = loaded.active_providers.get("claude").unwrap();
        assert!(
            active.is_some(),
            "Should have auto-switched to another provider"
        );
        // The active should be one of the remaining providers
        let active_id = active.as_ref().unwrap();
        assert!(active_id == "p1" || active_id == "p3");
    }

    #[test]
    fn test_delete_active_provider_keeps_original_when_auto_switch_fails() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        let settings_path = tmp.path().join("local.json");
        std::fs::create_dir_all(&providers_dir).unwrap();

        let p1 = make_provider("p1", "Provider 1", "claude");
        let p2 = make_provider("p2", "Provider 2", "claude");
        save_provider_to(&providers_dir, &p1).unwrap();
        save_provider_to(&providers_dir, &p2).unwrap();

        let mut settings = LocalSettings::default();
        settings
            .active_providers
            .insert("claude".to_string(), Some("p2".to_string()));
        write_local_settings_to(&settings_path, &settings).unwrap();

        let err = _delete_provider_in(
            &providers_dir,
            &settings_path,
            "p2".to_string(),
            Some(Box::new(FailingAdapter)),
        )
        .unwrap_err();

        assert!(matches!(err, AppError::Validation(ref msg) if msg == "patch failed"));
        assert!(get_provider_in(&providers_dir, "p2").is_ok());

        let loaded = read_local_settings_from(&settings_path).unwrap();
        assert_eq!(
            loaded.active_providers.get("claude"),
            Some(&Some("p2".to_string()))
        );
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
