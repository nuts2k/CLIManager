#![cfg(desktop)]

use tauri::menu::{Menu, MenuBuilder, MenuItem};
use tauri::{AppHandle, Manager, Wry};

/// Build the Phase 6 tray menu: "打开主窗口" -> separator -> "退出"
pub fn create_tray_menu(app: &AppHandle) -> Result<Menu<Wry>, Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show_main", "打开主窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    MenuBuilder::new(app)
        .item(&show_item)
        .separator()
        .item(&quit_item)
        .build()
        .map_err(Into::into)
}

/// Handle tray menu item clicks
pub fn handle_tray_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "show_main" => show_main_window(app),
        "quit" => {
            log::info!("Quit from tray menu");
            app.exit(0);
        }
        _ => log::warn!("Unhandled tray menu event: {event_id}"),
    }
}

/// Show and focus the main window, restore Dock presence
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        #[cfg(target_os = "macos")]
        apply_tray_policy(app, true);
    }
}

/// Toggle macOS Dock/Cmd+Tab visibility
#[cfg(target_os = "macos")]
pub fn apply_tray_policy(app: &AppHandle, dock_visible: bool) {
    use tauri::ActivationPolicy;
    let policy = if dock_visible {
        ActivationPolicy::Regular
    } else {
        ActivationPolicy::Accessory
    };
    if let Err(e) = app.set_dock_visibility(dock_visible) {
        log::warn!("Failed to set dock visibility: {e}");
    }
    if let Err(e) = app.set_activation_policy(policy) {
        log::warn!("Failed to set activation policy: {e}");
    }
}
