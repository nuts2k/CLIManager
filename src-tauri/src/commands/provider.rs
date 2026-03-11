use crate::error::AppError;
use crate::provider::{Provider, ProtocolType};
use crate::storage::local::{LocalSettings, read_local_settings, write_local_settings};

#[tauri::command]
pub fn list_providers() -> Result<Vec<Provider>, AppError> {
    crate::storage::icloud::list_providers()
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
    crate::storage::icloud::delete_provider(&id)
}

#[tauri::command]
pub fn get_local_settings() -> Result<LocalSettings, AppError> {
    read_local_settings()
}

#[tauri::command]
pub fn set_active_provider(provider_id: Option<String>) -> Result<LocalSettings, AppError> {
    let mut settings = read_local_settings()?;
    settings.active_provider_id = provider_id;
    write_local_settings(&settings)?;
    Ok(settings)
}
