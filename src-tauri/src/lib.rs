mod commands;
mod error;
mod provider;
mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::provider::list_providers,
            commands::provider::get_provider,
            commands::provider::create_provider,
            commands::provider::update_provider,
            commands::provider::delete_provider,
            commands::provider::get_local_settings,
            commands::provider::set_active_provider,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
