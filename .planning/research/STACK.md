# Technology Stack

**Project:** CLIManager v1.1 System Tray
**Researched:** 2026-03-12

## Scope

This document covers ONLY the incremental stack additions needed for system tray support in v1.1. The existing v1.0 stack (Tauri 2.10, React 19, Vite 7, shadcn/ui, Tailwind CSS v4, i18next, Rust backend with serde/toml_edit/notify/etc.) is validated and shipped -- it is NOT re-evaluated here.

---

## Recommended Stack Additions

### Cargo Feature Flags (No New Crates)

| Feature | On Crate | Purpose | Why | Confidence |
|---------|----------|---------|-----|------------|
| `tray-icon` | `tauri` | Enables `TrayIconBuilder`, `TrayIconEvent`, tray menu APIs | Required for any system tray functionality in Tauri 2. This is the renamed `system-tray` feature from Tauri v1. Without it, `tauri::tray` module is not available. | HIGH |
| `image-png` | `tauri` | Enables `Image::from_bytes()` for PNG parsing | Required to load custom tray icons from embedded PNG bytes via `include_bytes!`. Without it, runtime panics on PNG decode. | HIGH |

**No new crate dependencies are needed.** Everything required for tray support is built into the `tauri` crate behind feature flags.

### Cargo.toml Change (Single Line)

```toml
# Before (v1.0)
tauri = { version = "2", features = [] }

# After (v1.1)
tauri = { version = "2", features = ["tray-icon", "image-png"] }
```

Confidence: **HIGH** -- verified against cc-switch's working `Cargo.toml` which uses `tauri = { version = "2.8.2", features = ["tray-icon", "protocol-asset", "image-png"] }`, and confirmed by the [official Tauri 2 system tray documentation](https://v2.tauri.app/learn/system-tray/).

### Frontend Changes: None

No new npm packages are needed. The existing `@tauri-apps/api` v2 package already includes `tray` and `menu` namespaces, but **all tray logic should be implemented in Rust** because:

1. The tray must work when the window is hidden (no webview running JS).
2. Provider switching triggers Rust-side file I/O (surgical patch via existing commands).
3. cc-switch does it entirely in Rust -- this is the correct and proven pattern.

The frontend only needs to call a single new Tauri command (`update_tray_menu`) after provider data changes, to keep the tray menu in sync.

### Icon Assets Needed

| Asset | Spec | Purpose |
|-------|------|---------|
| `src-tauri/icons/tray/macos/statusbar_template.png` | 22x22px, monochrome black on transparent | macOS status bar icon (1x) |
| `src-tauri/icons/tray/macos/statusbar_template@2x.png` | 44x44px, monochrome black on transparent | macOS status bar icon (2x Retina) |

**Why template icons:** On macOS, menu bar icons must be "template images" (monochrome with alpha channel). Tauri 2 exposes `.icon_as_template(true)` on `TrayIconBuilder`, which tells AppKit to auto-tint for light/dark mode. Using a colored or full-resolution app icon would look wrong and violate macOS HIG.

cc-switch stores these at `icons/tray/macos/statusbar_template_3x.png` and loads via `include_bytes!`.

---

## Key Tauri 2 APIs

All APIs are from the `tauri` crate. No external plugins involved.

### Tray Construction (`tauri::tray`)

| API | Purpose |
|-----|---------|
| `TrayIconBuilder::with_id("main")` | Create named tray icon (ID used for `tray_by_id()` lookup later) |
| `.icon(Image::from_bytes(include_bytes!(...))?)` | Set the tray icon from embedded PNG |
| `.icon_as_template(true)` | macOS: treat as template image for auto light/dark tinting |
| `.menu(&menu)` | Attach a `Menu` to the tray |
| `.show_menu_on_left_click(true)` | Show menu on left click (macOS convention for utility apps) |
| `.on_menu_event(\|app, event\| { ... })` | Handle menu item clicks by `event.id` |
| `.on_tray_icon_event(\|tray, event\| { ... })` | Handle tray icon clicks/hover (optional) |
| `.build(app)?` | Finalize and register the tray icon |

### Menu Construction (`tauri::menu`)

| API | Purpose |
|-----|---------|
| `MenuBuilder::new(app)` | Start building a menu |
| `MenuItem::with_id(app, id, label, enabled, accel)` | Non-checkable item ("Show Window", "Quit", section headers) |
| `CheckMenuItem::with_id(app, id, label, enabled, checked, accel)` | Checkable item (providers -- shows native checkmark for active provider) |
| `.separator()` | Visual separator between menu sections |
| `.item(&item)` | Add an item to the builder |
| `.build()?` | Finalize the menu |

### Dynamic Menu Updates

| API | Purpose |
|-----|---------|
| `app.tray_by_id("main")` | Get existing tray icon by ID |
| `tray.set_menu(Some(new_menu))` | Replace the entire menu |

**Critical pattern:** Tauri 2 menus are immutable after `.build()`. You cannot add/remove/modify individual items. The correct approach is to rebuild the entire `Menu` and call `set_menu()`. This is exactly what cc-switch does in `create_tray_menu()` + `set_menu()`.

### Window Lifecycle -- Close-to-Tray

| API | Purpose |
|-----|---------|
| `Builder::on_window_event(\|window, event\| { ... })` | Intercept window events globally |
| `WindowEvent::CloseRequested { api, .. }` | Fired when user clicks window close button |
| `api.prevent_close()` | Prevent the window from actually being destroyed |
| `window.hide()` | Hide the window (process stays alive, tray remains) |
| `window.show()` | Restore hidden window |
| `window.set_focus()` | Bring window to front |
| `window.unminimize()` | Restore from minimized state |

**The close-to-tray pattern:**

```rust
.on_window_event(|window, event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
        #[cfg(target_os = "macos")]
        {
            tray::apply_tray_policy(window.app_handle(), false);
        }
    }
})
```

Confidence: **HIGH** -- this is the standard pattern documented by Tauri and used by cc-switch. The `api.prevent_close()` + `window.hide()` combination is the recommended approach per [community discussion](https://github.com/tauri-apps/tauri/discussions/2684) and official docs.

### macOS Dock Visibility

| API | Purpose |
|-----|---------|
| `app.set_activation_policy(ActivationPolicy::Accessory)` | Hide app from Dock (tray-only mode) |
| `app.set_activation_policy(ActivationPolicy::Regular)` | Show app in Dock (normal mode) |
| `app.set_dock_visibility(bool)` | Show/hide Dock icon |

**Pattern:** When window hides, switch to `Accessory` to remove Dock icon. When window shows, switch to `Regular` to restore Dock presence. cc-switch wraps this in `tray::apply_tray_policy()` with error handling.

**Note:** `ActivationPolicy` is macOS-only. Wrap with `#[cfg(target_os = "macos")]`.

---

## tauri.conf.json Changes

**None required.** The `tray-icon` feature is a Cargo feature flag, not a Tauri capability/permission. The existing `tauri.conf.json` remains unchanged.

---

## What NOT to Do

| Anti-Pattern | Why |
|-------------|-----|
| Use Tauri v1 `SystemTray` / `SystemTrayMenu` / `CustomMenuItem` | Removed in Tauri 2. Use `TrayIconBuilder` / `Menu` / `MenuItem`. |
| Implement tray logic in JavaScript | Tray must work when webview is hidden. All tray logic belongs in Rust. |
| Add any `tauri-plugin-*` for tray | No plugin needed. Tray is built into `tauri` core behind `tray-icon` feature. |
| Mutate individual menu items after build | Tauri 2 menus are immutable. Rebuild entire menu + `set_menu()`. |
| Use `Submenu` for provider grouping | CLIManager only has 2 CLI types (Claude Code, Codex). Flat menu with disabled `MenuItem` headers and `CheckMenuItem` providers is simpler and more accessible. |
| Add `protocol-asset` feature | Only needed for `asset://` protocol. Not needed for tray. cc-switch uses it for other features. |
| Skip `icon_as_template(true)` on macOS | Without template mode, icon won't adapt to light/dark menu bar. |
| Use `app.default_window_icon()` as tray icon | App icons are colored and too large for menu bar. Use a dedicated 22x22 template icon. |

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Tray implementation layer | Rust-only (`tauri::tray`) | JS-side (`@tauri-apps/api/tray`) | Tray must function when window is hidden; Rust is the correct layer |
| Menu updates | Full rebuild + `set_menu()` | Individual item mutation | Tauri 2 menus are immutable post-build; rebuild is the only option |
| Active provider display | `CheckMenuItem` with native checkmark | `MenuItem` with emoji/text prefix | `CheckMenuItem` is semantic, native, handles check state automatically |
| Dock icon behavior | Toggle Accessory/Regular on hide/show | Always hide dock icon | Users expect dock icon when window is visible; toggling matches macOS convention |
| Tray icon asset | Monochrome template PNG | Colored app icon | macOS HIG mandates monochrome template icons in menu bar |
| Menu structure | Flat with section headers | Nested submenus per CLI | Flat is faster to navigate; only 2 CLI types don't warrant nesting |

---

## Integration Points with Existing v1.0 Code

The tray feature integrates with existing code at these points:

| Existing Code | Integration |
|---------------|-------------|
| `lib.rs` `run()` function | Add `.on_window_event()` for close-to-tray; add tray builder in `.setup()` |
| `commands::provider::set_active_provider` | After switching, call tray menu rebuild |
| `watcher::start_file_watcher` | When iCloud sync triggers provider refresh, also rebuild tray menu |
| `storage` module | Tray reads provider list + active provider via existing storage APIs |
| i18next translations | Add tray-specific strings: "Show Window", "Quit", "No providers" |

No changes needed to: `adapter`, `provider` (model), `error`, `commands::onboarding`.

---

## Installation Summary

```toml
# src-tauri/Cargo.toml -- single dependency line change
tauri = { version = "2", features = ["tray-icon", "image-png"] }
```

No `npm install`. No new Rust crates. No config changes. Two icon assets to create.

---

## Sources

- [Tauri 2 System Tray Guide](https://v2.tauri.app/learn/system-tray/) -- official documentation (HIGH confidence)
- [Tauri 2 Tray JS API Reference](https://v2.tauri.app/reference/javascript/api/namespacetray/) -- official API reference
- [Tauri Feature Flags](https://lib.rs/crates/tauri/features) -- Cargo feature reference
- [Tauri GitHub Discussion #2684 -- Close to Tray](https://github.com/tauri-apps/tauri/discussions/2684) -- community pattern for close-to-tray
- [Tauri GitHub Discussion #6038 -- macOS Dock Hide](https://github.com/tauri-apps/tauri/discussions/6038) -- ActivationPolicy pattern
- [Tauri GitHub Discussion #10774 -- Dock Toggle](https://github.com/tauri-apps/tauri/discussions/10774) -- dynamic dock visibility toggle
- cc-switch `src-tauri/src/tray.rs` -- working reference: CheckMenuItem, dynamic menu rebuild, ActivationPolicy, i18n tray texts
- cc-switch `src-tauri/src/lib.rs` -- working reference: TrayIconBuilder setup, on_window_event close-to-tray, macOS template icon loading
- cc-switch `src-tauri/Cargo.toml` -- confirmed features: `tray-icon`, `image-png`
