#[cfg(desktop)]
use tauri::Manager;

mod adapter;
mod commands;
mod error;
mod provider;
mod storage;
#[cfg(desktop)]
mod tray;
mod watcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
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
            commands::onboarding::import_provider,
        ]);

    #[cfg(desktop)]
    let builder = builder.on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
            #[cfg(target_os = "macos")]
            tray::apply_tray_policy(window.app_handle(), false);
        }
    });

    let app = builder
        .setup(|app| {
            // Existing file watcher setup
            let handle = app.handle().clone();
            watcher::start_file_watcher(handle)?;

            #[cfg(desktop)]
            {
                let menu = tray::create_tray_menu(app.handle()).map_err(|e| e.to_string())?;

                let icon_bytes: &[u8] = include_bytes!("../icons/tray/tray-icon-template.png");
                let icon =
                    tauri::image::Image::from_bytes(icon_bytes).map_err(|e| e.to_string())?;

                use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};

                let _tray = TrayIconBuilder::with_id("main")
                    .icon(icon)
                    .icon_as_template(true)
                    .menu(&menu)
                    .show_menu_on_left_click(true)
                    .on_tray_icon_event(|tray_icon, event| {
                        if let TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            ..
                        } = event
                        {
                            tray::show_main_window(tray_icon.app_handle());
                        }
                    })
                    .on_menu_event(|app, event| {
                        tray::handle_tray_menu_event(app, &event.id.0);
                    })
                    .build(app)?;
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Cmd+Q and programmatic exit pass through here.
    // Default behavior: allow exit (no api.prevent_exit() call).
    app.run(|_app_handle, _event| {});
}
