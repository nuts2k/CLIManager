#[cfg(desktop)]
use tauri::Manager;

mod adapter;
mod commands;
mod error;
mod provider;
mod proxy;
mod storage;
mod traffic;
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
        .manage(commands::claude_settings::ClaudeOverlayStartupNotificationQueue::new())
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
            commands::claude_settings::get_claude_settings_overlay,
            commands::claude_settings::set_claude_settings_overlay,
            commands::claude_settings::apply_claude_settings_overlay_cmd,
            commands::claude_settings::take_claude_overlay_startup_notifications,
            commands::traffic::get_recent_logs,
            commands::traffic::get_provider_stats,
            commands::traffic::get_time_trend,
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

            // 初始化 traffic DB（降级运行：失败不阻断启动）
            // 位置：watcher 启动后（文件系统就绪），proxy 恢复前（尽早可用）
            if let Some(traffic_db) = traffic::init_traffic_db() {
                app.manage(traffic_db);
                log::info!("traffic.db 初始化成功");
            } else {
                log::warn!("traffic.db 不可用，代理将正常工作但不记录流量");
            }

            // 创建 mpsc channel 用于流量日志写入（buffer 1024，fire-and-forget 不阻塞代理）
            let (log_tx, log_rx) =
                tokio::sync::mpsc::channel::<crate::traffic::log::LogEntry>(1024);

            // 注入 log sender 到 ProxyService
            let proxy_service = app.state::<proxy::ProxyService>();
            proxy_service.set_log_sender(log_tx);

            // 注入 AppHandle 到 ProxyService（Phase 28 新增，供后台 task emit 使用）
            proxy_service.set_app_handle(app.handle().clone());

            // 启动后台日志写入 worker
            let app_handle_for_log = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                crate::traffic::log::log_worker(log_rx, app_handle_for_log).await;
            });

            // 启动 rollup_and_prune 定时任务（每小时执行一次，首次立即执行）
            // 使用 tauri::async_runtime::spawn（非 tokio::spawn），Tauri 2 中安全做法
            // rollup_and_prune 使用 std::sync::Mutex，持锁时间短，在 async 内调用安全
            let app_handle_for_rollup = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                use tauri::Manager;
                loop {
                    if let Some(db) = app_handle_for_rollup.try_state::<crate::traffic::TrafficDb>() {
                        match db.rollup_and_prune() {
                            Ok(_) => log::info!("rollup_and_prune 执行完成"),
                            Err(e) => log::warn!("rollup_and_prune 执行失败: {}", e),
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
            });

            // Startup overlay apply（COVL-10：best-effort，不阻断启动）
            // 因为 setup 早于 WebView 事件监听，startup 结果写入缓存队列
            // 由前端 useSyncListener 挂载后主动 take。
            {
                let handle_startup = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = commands::claude_settings::apply_claude_settings_overlay(
                        &handle_startup,
                        commands::claude_settings::ApplySource::Startup,
                    ) {
                        log::error!("startup overlay apply 失败: {}", e);
                    }
                });
            }

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
