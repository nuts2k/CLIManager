mod adapter;
mod commands;
mod error;
mod provider;
mod storage;
mod watcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(watcher::SelfWriteTracker::new())
        .invoke_handler(tauri::generate_handler![
            commands::provider::list_providers,
            commands::provider::get_provider,
            commands::provider::create_provider,
            commands::provider::update_provider,
            commands::provider::delete_provider,
            commands::provider::get_local_settings,
            commands::provider::set_active_provider,
            commands::provider::update_local_settings,
            commands::provider::sync_active_providers,
            commands::provider::test_provider,
            commands::onboarding::scan_cli_configs,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            watcher::start_file_watcher(handle).map_err(|e| e.into())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
