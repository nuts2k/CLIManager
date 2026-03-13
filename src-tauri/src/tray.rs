#![cfg(desktop)]

use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuItem};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::error::AppError;

/// Tray menu i18n labels. Brand names ("Claude Code", "Codex") are
/// identical across languages per CONTEXT.md.
struct TrayTexts {
    show_main: &'static str,
    quit: &'static str,
    claude_header: &'static str,
    codex_header: &'static str,
}

impl TrayTexts {
    fn from_language(lang: &str) -> Self {
        if lang.starts_with("en") {
            Self {
                show_main: "Open Main Window",
                quit: "Quit",
                claude_header: "Claude Code",
                codex_header: "Codex",
            }
        } else {
            // Default to Chinese
            Self {
                show_main: "打开主窗口",
                quit: "退出",
                claude_header: "Claude Code",
                codex_header: "Codex",
            }
        }
    }
}

/// Parse a tray menu event ID into (cli_id, provider_id).
/// Returns None for header items, empty provider IDs, and unrecognized IDs.
fn parse_provider_event(event_id: &str) -> Option<(&str, &str)> {
    for (prefix, cli_id) in [("claude_", "claude"), ("codex_", "codex")] {
        if let Some(provider_id) = event_id.strip_prefix(prefix) {
            if provider_id == "header" || provider_id.is_empty() {
                return None;
            }
            return Some((cli_id, provider_id));
        }
    }
    None
}

/// Helper to convert menu errors into AppError::Validation
fn menu_err(e: impl std::fmt::Display) -> AppError {
    AppError::Validation(format!("menu: {e}"))
}

/// Build the dynamic tray menu from live provider data and language settings.
///
/// Layout per CONTEXT.md locked decision:
/// "Open Main Window" -> separator -> [Claude Code group] -> [Codex group] -> separator -> "Quit"
/// Empty CLI groups are hidden. Active provider sorts first within each group.
pub fn create_tray_menu(app: &AppHandle) -> Result<Menu<Wry>, Box<dyn std::error::Error>> {
    let settings = crate::storage::local::read_local_settings().unwrap_or_default();
    let all_providers = crate::storage::icloud::list_providers().unwrap_or_default();
    let lang = settings.language.as_deref().unwrap_or("zh");
    let texts = TrayTexts::from_language(lang);

    let mut builder = MenuBuilder::new(app);

    // "Open Main Window"
    let show_item =
        MenuItem::with_id(app, "show_main", texts.show_main, true, None::<&str>).map_err(menu_err)?;
    builder = builder.item(&show_item).separator();

    // CLI sections: Claude Code first, then Codex (user decision)
    let mut has_any_providers = false;
    for (cli_id, header_label) in [("claude", texts.claude_header), ("codex", texts.codex_header)] {
        let mut cli_providers: Vec<_> = all_providers.iter().filter(|p| p.cli_id == cli_id).collect();

        // Hide CLI groups with no providers (user decision)
        if cli_providers.is_empty() {
            continue;
        }

        has_any_providers = true;

        let active_id = settings
            .active_providers
            .get(cli_id)
            .and_then(|v| v.as_ref())
            .map(|s| s.as_str());

        // Sort: active first, then by name alphabetically (user decision)
        cli_providers.sort_by(|a, b| {
            let a_active = active_id == Some(a.id.as_str());
            let b_active = active_id == Some(b.id.as_str());
            match (a_active, b_active) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        // Section header (disabled MenuItem)
        let header = MenuItem::with_id(
            app,
            format!("{cli_id}_header"),
            header_label,
            false,
            None::<&str>,
        )
        .map_err(menu_err)?;
        builder = builder.item(&header);

        // Provider items (CheckMenuItem)
        for provider in cli_providers {
            let is_active = active_id == Some(provider.id.as_str());
            let item = CheckMenuItem::with_id(
                app,
                format!("{cli_id}_{}", provider.id),
                &provider.name,
                true,
                is_active,
                None::<&str>,
            )
            .map_err(menu_err)?;
            builder = builder.item(&item);
        }
    }

    // Only add separator before Quit if there were provider sections
    if has_any_providers {
        builder = builder.separator();
    }

    // "Quit"
    let quit =
        MenuItem::with_id(app, "quit", texts.quit, true, None::<&str>).map_err(menu_err)?;
    builder = builder.item(&quit);

    builder.build().map_err(|e| Box::new(menu_err(e)) as Box<dyn std::error::Error>)
}

/// Rebuild the tray menu from current storage state and replace the running menu.
/// Logs errors but never panics.
pub fn update_tray_menu(app: &AppHandle) {
    match create_tray_menu(app) {
        Ok(menu) => {
            if let Some(tray) = app.tray_by_id("main") {
                if let Err(e) = tray.set_menu(Some(menu)) {
                    log::error!("Failed to update tray menu: {e}");
                }
            }
        }
        Err(e) => log::error!("Failed to create tray menu: {e}"),
    }
}

/// Handle a provider click from the tray menu.
/// Runs the switch logic in a blocking thread to avoid blocking the main thread.
fn handle_provider_click(app: &AppHandle, cli_id: &str, provider_id: &str) {
    let app_handle = app.clone();
    let cli_id = cli_id.to_string();
    let provider_id = provider_id.to_string();

    tauri::async_runtime::spawn_blocking(move || {
        let providers_dir = match crate::storage::icloud::get_icloud_providers_dir() {
            Ok(d) => d,
            Err(e) => {
                log::error!("Tray switch failed: {e}");
                update_tray_menu(&app_handle);
                return;
            }
        };
        let settings_path = crate::storage::local::get_local_settings_path();

        match crate::commands::provider::_set_active_provider_in(
            &providers_dir,
            &settings_path,
            cli_id.clone(),
            Some(provider_id.clone()),
            None,
        ) {
            Ok(_) => {
                log::info!("Tray: switched {cli_id} to {provider_id}");
                update_tray_menu(&app_handle);
                // Emit providers-changed event so frontend listeners pick up the change
                let _ = app_handle.emit(
                    "providers-changed",
                    serde_json::json!({
                        "changed_ids": [provider_id],
                        "source": "tray"
                    }),
                );
            }
            Err(e) => {
                log::error!("Tray switch failed: {e}");
                // Rebuild menu to reset CheckMenuItem visual state (pitfall 2)
                update_tray_menu(&app_handle);
            }
        }
    });
}

/// Handle tray menu item clicks
pub fn handle_tray_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "show_main" => show_main_window(app),
        "quit" => {
            log::info!("Quit from tray menu");
            app.exit(0);
        }
        id => {
            if let Some((cli_id, provider_id)) = parse_provider_event(id) {
                handle_provider_click(app, cli_id, provider_id);
            } else {
                log::warn!("Unhandled tray menu event: {event_id}");
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- TrayTexts tests ---

    #[test]
    fn test_tray_texts_zh() {
        let texts = TrayTexts::from_language("zh");
        assert_eq!(texts.show_main, "打开主窗口");
        assert_eq!(texts.quit, "退出");
        assert_eq!(texts.claude_header, "Claude Code");
        assert_eq!(texts.codex_header, "Codex");
    }

    #[test]
    fn test_tray_texts_en() {
        let texts = TrayTexts::from_language("en");
        assert_eq!(texts.show_main, "Open Main Window");
        assert_eq!(texts.quit, "Quit");
        assert_eq!(texts.claude_header, "Claude Code");
        assert_eq!(texts.codex_header, "Codex");
    }

    #[test]
    fn test_tray_texts_default() {
        // Non-recognized language falls back to Chinese
        let texts = TrayTexts::from_language("fr");
        assert_eq!(texts.show_main, "打开主窗口");
        assert_eq!(texts.quit, "退出");
        assert_eq!(texts.claude_header, "Claude Code");
        assert_eq!(texts.codex_header, "Codex");
    }

    // --- parse_provider_event tests ---

    #[test]
    fn test_parse_provider_event_claude() {
        let result = parse_provider_event("claude_abc-123");
        assert_eq!(result, Some(("claude", "abc-123")));
    }

    #[test]
    fn test_parse_provider_event_codex() {
        let result = parse_provider_event("codex_def-456");
        assert_eq!(result, Some(("codex", "def-456")));
    }

    #[test]
    fn test_parse_provider_event_header() {
        let result = parse_provider_event("claude_header");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_provider_event_unknown() {
        let result = parse_provider_event("show_main");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_provider_event_empty_suffix() {
        let result = parse_provider_event("claude_");
        assert_eq!(result, None);
    }
}
