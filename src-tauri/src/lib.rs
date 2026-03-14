#[cfg(desktop)]
use tauri::Manager;

mod adapter;
mod commands;
mod error;
mod provider;
mod proxy;
mod storage;
#[cfg(desktop)]
mod tray;
mod watcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(watcher::SelfWriteTracker::new())
        .manage(proxy::ProxyService::new())
        .manage(commands::proxy::ProxyGlobalToggleLock::new())
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
            commands::provider::refresh_tray_menu,
            commands::onboarding::scan_cli_configs,
            commands::onboarding::import_provider,
            commands::proxy::proxy_start,
            commands::proxy::proxy_stop,
            commands::proxy::proxy_status,
            commands::proxy::proxy_update_upstream,
            commands::proxy::proxy_enable,
            commands::proxy::proxy_disable,
            commands::proxy::proxy_set_global,
            commands::proxy::proxy_get_mode_status,
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

            // 崩溃恢复：检测并还原遗留 takeover 标志
            let providers_dir =
                crate::storage::icloud::get_icloud_providers_dir().map_err(|e| e.to_string())?;
            let local_settings_path = crate::storage::local::get_local_settings_path();
            if let Err(e) =
                commands::proxy::recover_on_startup(&providers_dir, &local_settings_path)
            {
                log::error!("崩溃恢复失败: {}", e);
            }

            // 自动恢复代理状态（UX-02）：根据持久化的开关状态重新开启代理
            let handle_for_restore = app.handle().clone();
            let providers_dir_clone = providers_dir.clone();
            let local_settings_path_clone = local_settings_path.clone();
            tauri::async_runtime::spawn(async move {
                let proxy_service = handle_for_restore.state::<proxy::ProxyService>();
                if let Err(e) = commands::proxy::restore_proxy_state(
                    &providers_dir_clone,
                    &local_settings_path_clone,
                    &proxy_service,
                )
                .await
                {
                    log::error!("代理状态恢复失败: {}", e);
                }
            });

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
    // 正常退出时还原所有 CLI 配置并停止代理。
    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            // 1. 同步还原 CLI 配置（先于代理停止，避免 CLI 指向已关闭的 localhost）
            let providers_dir = crate::storage::icloud::get_icloud_providers_dir().ok();
            let local_settings_path = crate::storage::local::get_local_settings_path();
            if let Some(ref providers_dir) = providers_dir {
                commands::proxy::cleanup_on_exit_sync(providers_dir, &local_settings_path);
            }

            // 2. 异步停止所有代理
            let proxy_service = app_handle.state::<proxy::ProxyService>();
            tauri::async_runtime::block_on(async {
                proxy_service.stop_all().await;
            });
        }
    });
}
