# Phase 6: Tray Foundation - Research

**Researched:** 2026-03-13
**Domain:** macOS system tray integration (Tauri 2 tray-icon feature)
**Confidence:** HIGH

## Summary

Phase 6 adds system tray persistence to CLIManager: a menu bar icon, close-to-tray lifecycle, ActivationPolicy toggling, and a minimal tray menu with "Open Main Window" and "Quit". No provider listing in the menu -- that belongs to Phase 7.

The implementation requires zero new crate dependencies. Everything is behind Tauri's `tray-icon` and `image-png` feature flags on the existing `tauri` crate. The cc-switch reference codebase provides a battle-tested implementation of every pattern needed. The prior v1.1 research (`.planning/research/`) already covers the full API surface, architecture patterns, and pitfalls in detail.

The one technically uncertain area is distinguishing Cmd+Q (should quit) from the close button (should hide to tray). The CONTEXT.md locks in "attempt to distinguish first, accept hide-to-tray as fallback." Research shows this IS achievable by splitting handling between `on_window_event(CloseRequested)` (intercept close button -> hide) and `RunEvent::ExitRequested` (Cmd+Q -> allow exit). This requires refactoring `lib.rs` from `.run()` to `.build()` + `.run()`.

**Primary recommendation:** Implement tray foundation in a single new `tray.rs` module (~100 lines for Phase 6 scope), modify `lib.rs` to use `.build()+.run()` pattern for Cmd+Q handling, and create a monochrome template PNG for the tray icon.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Cmd+Q should fully quit the application (macOS standard behavior)
- Close button (red X) hides the window to tray instead of quitting
- Fallback: if technically unable to distinguish Cmd+Q from close button in Tauri 2's CloseRequested event, fall back to hiding to tray for both
- Priority: attempt to distinguish first, accept hide-to-tray as fallback
- App startup: always open the main window (current behavior preserved)
- Close button: hide window silently, no notification or dialog
- Hidden state: app switches to Accessory mode (no Dock icon, no Cmd+Tab entry) per TRAY-03
- Window restore: switch back to Regular mode (Dock + Cmd+Tab reappear)
- Single click on tray icon: opens the tray menu (macOS standard)
- Double click on tray icon: opens/shows the main window
- No tooltip in Phase 6 (deferred to Phase 7+)
- Menu layout top-to-bottom: "打开主窗口" -> separator -> "退出"
- Phase 7 will insert Provider list between "打开主窗口" and the separator
- Language: Chinese hardcoded in Phase 6, i18n conversion in Phase 7

### Claude's Discretion
- Tray icon PNG asset design (22x22 monochrome template)
- Exact implementation of Cmd+Q vs close button distinction attempt
- Error handling for edge cases (e.g., window already visible when "打开主窗口" clicked)
- Release build verification approach

### Deferred Ideas (OUT OF SCOPE)
- Launch at login (Login Items) -- new capability, future phase or v1.2
- Switching visual feedback (icon flash/animation) -- Phase 7 scope
- Tray icon tooltip showing active Provider -- Phase 7+
- Tray icon state variants (active vs no-provider) -- Phase 7+
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TRAY-01 | Tray icon in macOS menu bar, template icon adapting to dark/light mode | `TrayIconBuilder` + `.icon_as_template(true)` + monochrome PNG via `include_bytes!`. Verified in cc-switch reference and Tauri 2 docs. |
| TRAY-02 | Close window hides instead of quit, app persists in tray | `on_window_event(CloseRequested)` + `api.prevent_close()` + `window.hide()`. Cmd+Q handled via `RunEvent::ExitRequested` at app level. |
| TRAY-03 | Accessory mode when hidden (no Dock, no Cmd+Tab); Regular when shown | `app.set_activation_policy(Accessory/Regular)` + `app.set_dock_visibility(bool)`. macOS-only via `#[cfg(target_os = "macos")]`. |
| MENU-01 | "打开主窗口" menu item shows and focuses main window | `MenuItem::with_id` + `window.show()` + `window.set_focus()` + `window.unminimize()` + restore Regular policy. |
| MENU-02 | "退出" menu item fully exits the application | `MenuItem::with_id("quit", ...)` + `app.exit(0)`. |
</phase_requirements>

## Standard Stack

### Core (Feature Flags Only -- No New Crates)

| Feature | On Crate | Version | Purpose | Why Standard |
|---------|----------|---------|---------|--------------|
| `tray-icon` | `tauri` | 2.x | Enables `TrayIconBuilder`, `TrayIconEvent`, tray menu APIs | Required for any tray functionality in Tauri 2 |
| `image-png` | `tauri` | 2.x | Enables `Image::from_bytes()` for PNG parsing | Required to load embedded tray icon PNG at runtime |

### Cargo.toml Change (Single Line)

```toml
# Before (v1.0)
tauri = { version = "2", features = [] }

# After (v1.1 Phase 6)
tauri = { version = "2", features = ["tray-icon", "image-png"] }
```

### Frontend Changes: None

No npm packages needed. All tray logic is Rust-only because the tray must work when the webview/window is hidden.

### New Assets Needed

| Asset | Spec | Purpose |
|-------|------|---------|
| `src-tauri/icons/tray/tray-icon-template.png` | 44x44px, monochrome black on transparent, PNG | macOS menu bar icon (@2x Retina). Loaded via `include_bytes!`, flagged with `.icon_as_template(true)` for auto dark/light adaptation. |

Recommendation: Create a simple monochrome icon (e.g., a small CLI/terminal glyph or the app's silhouette). 44x44px single file is sufficient -- macOS downscales to 22x22 logical points. The `_template` naming convention signals to developers this is a template image.

### Alternatives Considered

| Instead of | Could Use | Why Not |
|------------|-----------|---------|
| Custom tray icon PNG | `app.default_window_icon()` | App icons are colored/large, look wrong in macOS menu bar |
| JS-side tray logic | Rust-only tray | Tray must work when window is hidden (no JS running) |
| `trayIcon` in tauri.conf.json | Programmatic `TrayIconBuilder` | Config-based + programmatic causes duplicate/missing icon bugs (GitHub Issue #10912) |

## Architecture Patterns

### New Files

```
src-tauri/
├── src/
│   ├── tray.rs              # NEW: ~100 lines, menu build + event handlers + ActivationPolicy
│   └── lib.rs               # MODIFIED: add mod tray, on_window_event, tray setup in .setup(), .build()+.run()
├── icons/
│   └── tray/
│       └── tray-icon-template.png  # NEW: 44x44 monochrome template icon
└── Cargo.toml                # MODIFIED: add tray-icon, image-png features
```

### Pattern 1: Builder-then-Run for Cmd+Q Distinction

**What:** Refactor `lib.rs` from `.run(context).expect(...)` to `.build(context).expect(...).run(callback)` to access `RunEvent::ExitRequested`.

**When to use:** When the app needs to intercept both window close (CloseRequested) and application quit (Cmd+Q / ExitRequested) separately.

**How it works:**
- `on_window_event` intercepts `CloseRequested` -> `api.prevent_close()` + `window.hide()` + Accessory mode. This handles the close button.
- `app.run()` callback receives `RunEvent::ExitRequested` -> this fires when Cmd+Q is pressed or when the app tries to exit because all windows closed. Since we prevent close (not actually closing windows), `ExitRequested` from Cmd+Q should still fire. We allow it to proceed (do NOT call `api.prevent_exit()`), which quits the app.
- The key insight: `CloseRequested` fires for the close button. Cmd+Q triggers `ExitRequested` at the app level without going through `CloseRequested` first.

**Example:**
```rust
// Source: Tauri docs (https://docs.rs/tauri/latest/tauri/struct.App.html)
let app = tauri::Builder::default()
    .on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            // Close button: hide to tray
            api.prevent_close();
            let _ = window.hide();
            #[cfg(target_os = "macos")]
            apply_tray_policy(window.app_handle(), false);
        }
    })
    .setup(|app| {
        // ... tray setup ...
        Ok(())
    })
    .build(tauri::generate_context!())
    .expect("error while building tauri application");

app.run(|_app_handle, event| {
    if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
        // code == None means user-initiated (Cmd+Q)
        // code == Some(_) means programmatic (app.exit())
        // In both cases, allow exit -- do NOT call api.prevent_exit()
        // This is the default behavior, so we can just let it pass through
    }
});
```

**Fallback:** If Cmd+Q still triggers `CloseRequested` on macOS (which would hide instead of quit), the fallback per CONTEXT.md is acceptable: both close button and Cmd+Q hide to tray, and "Quit" in tray menu is the only way to exit. The cc-switch reference uses this simpler approach.

### Pattern 2: Tray Icon with DoubleClick Handler

**What:** `TrayIconBuilder` with `show_menu_on_left_click(true)` + `on_tray_icon_event` for `DoubleClick`.

**How:**
```rust
// Source: Tauri docs (https://docs.rs/tauri/latest/tauri/tray/enum.TrayIconEvent.html)
TrayIconBuilder::with_id("main")
    .icon(icon)
    .icon_as_template(true)
    .menu(&menu)
    .show_menu_on_left_click(true)
    .on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } = event {
            // Show main window on double click
            let app = tray.app_handle();
            show_main_window(app);
        }
    })
    .on_menu_event(|app, event| {
        handle_tray_menu_event(app, &event.id.0);
    })
    .build(app)?;
```

**Note on DoubleClick reliability:** The `DoubleClick` event works on macOS when using programmatic tray setup (not config-based). The reported bug (Issue #11413) was user error from dual config. Since CLIManager has NO `trayIcon` in `tauri.conf.json`, this is not a concern.

### Pattern 3: ActivationPolicy Toggle via Helper Function

**What:** Wrap `set_activation_policy` + `set_dock_visibility` in a helper for consistent toggling.

**Example:**
```rust
// Source: cc-switch/src-tauri/src/tray.rs (apply_tray_policy)
#[cfg(target_os = "macos")]
pub fn apply_tray_policy(app: &tauri::AppHandle, dock_visible: bool) {
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
```

### Anti-Patterns to Avoid

- **Setting tray in both tauri.conf.json AND TrayIconBuilder:** Causes duplicate or invisible tray icons. Use ONLY programmatic setup.
- **Using `RunEvent::ExitRequested` with `api.prevent_exit()` to keep app alive:** Creates infinite loop on macOS. Use `on_window_event` + `prevent_close()` instead.
- **Calling `app.exit()` from `on_window_event`:** This bypasses the hide-to-tray logic. Only call `app.exit(0)` from the tray "Quit" handler.
- **Not wrapping ActivationPolicy in `#[cfg(target_os = "macos")]`:** ActivationPolicy is macOS-only. Will fail to compile on other platforms.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Tray icon rendering | Custom icon generation at runtime | Static template PNG + `icon_as_template(true)` | macOS auto-tints template images for dark/light mode |
| Menu construction | Manual NSMenu via FFI | `tauri::menu::MenuBuilder` + `MenuItem` | Tauri 2 wraps native menu APIs, handles memory management |
| Dock visibility toggling | Direct `NSApp.setActivationPolicy` FFI | `app.set_activation_policy()` + `app.set_dock_visibility()` | Tauri 2 exposes these as safe Rust APIs |
| Close-to-tray | Custom window delegate | `on_window_event(CloseRequested)` + `api.prevent_close()` | Standard Tauri 2 pattern, handles edge cases |

## Common Pitfalls

### Pitfall 1: Tray Icon Not Appearing

**What goes wrong:** Tray icon silently fails to show in menu bar.
**Why it happens:** (1) Missing `tray-icon` feature flag, (2) wrong icon format/size, (3) dual config (tauri.conf.json + programmatic).
**How to avoid:** Use ONLY `TrayIconBuilder` (no tauri.conf.json `trayIcon`), add both `tray-icon` and `image-png` features, use `include_bytes!` for the icon, verify with `icon_as_template(true)`.
**Warning signs:** Icon works in dev but not release build; icon visible on external monitor but not built-in.

### Pitfall 2: App Exits Instead of Hiding on Close

**What goes wrong:** Clicking close button terminates the entire app.
**Why it happens:** Missing `on_window_event` handler, or calling `prevent_close()` incorrectly.
**How to avoid:** Add `on_window_event` BEFORE `.setup()` in the builder chain. Always call `api.prevent_close()` first, then `window.hide()`.
**Warning signs:** Tray icon disappears when window closes.

### Pitfall 3: Dock Icon Persists After Window Hide

**What goes wrong:** After hiding window to tray, the dock icon remains and the app still appears in Cmd+Tab.
**Why it happens:** Forgetting to call `set_activation_policy(Accessory)` after hiding.
**How to avoid:** Always pair `window.hide()` with `apply_tray_policy(app, false)` and `window.show()` with `apply_tray_policy(app, true)`.

### Pitfall 4: DoubleClick Not Firing

**What goes wrong:** Double-clicking tray icon does nothing (no window opens).
**Why it happens:** If `show_menu_on_left_click(true)` is set, the first click opens the menu. The double-click may be consumed by the menu opening.
**How to avoid:** Test this interaction. If DoubleClick conflicts with `show_menu_on_left_click`, consider: (a) removing `show_menu_on_left_click` and handling single click manually, or (b) accepting that double-click only works when menu is dismissed first. The CONTEXT.md marks this as Claude's discretion for implementation details.
**Fallback:** If DoubleClick is unreliable with `show_menu_on_left_click`, the "打开主窗口" menu item provides the same functionality.

### Pitfall 5: lib.rs Refactor Breaking Existing Behavior

**What goes wrong:** Switching from `.run()` to `.build()+.run()` introduces subtle behavior changes.
**Why it happens:** The `.run()` convenience method has default `RunEvent` handling. With `.build()+.run()`, you must handle events yourself or let them fall through.
**How to avoid:** In the `app.run()` callback, only match the events you care about and use a `_ => {}` fallback. Do NOT call `api.prevent_exit()` unless you have a specific reason.

## Code Examples

### Complete tray.rs for Phase 6 (Minimal)

```rust
// Source: Synthesized from cc-switch reference + Tauri 2 docs
use tauri::menu::{Menu, MenuBuilder, MenuItem};
use tauri::{AppHandle, Manager, Wry};
use tauri::tray::TrayIconEvent;

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
```

### lib.rs Changes (Refactored to .build()+.run())

```rust
// Source: Tauri 2 App::run docs + cc-switch reference
mod tray; // NEW

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(watcher::SelfWriteTracker::new())
        .invoke_handler(tauri::generate_handler![
            // ... existing commands ...
        ])
        // Intercept close button -> hide to tray
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                #[cfg(target_os = "macos")]
                tray::apply_tray_policy(window.app_handle(), false);
            }
        })
        .setup(|app| {
            // Existing watcher setup
            let handle = app.handle().clone();
            watcher::start_file_watcher(handle)?;

            // Tray setup
            let menu = tray::create_tray_menu(app.handle())
                .map_err(|e| e.to_string())?;

            let icon_bytes: &[u8] = include_bytes!("../icons/tray/tray-icon-template.png");
            let icon = tauri::image::Image::from_bytes(icon_bytes)
                .map_err(|e| e.to_string())?;

            use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};

            let _tray = TrayIconBuilder::with_id("main")
                .icon(icon)
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_tray_icon_event(|tray_icon, event| {
                    if let TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } = event {
                        tray::show_main_window(tray_icon.app_handle());
                    }
                })
                .on_menu_event(|app, event| {
                    tray::handle_tray_menu_event(app, &event.id.0);
                })
                .build(app)?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Cmd+Q and programmatic exit pass through here
    // Default behavior: allow exit (no api.prevent_exit() call)
    app.run(|_app_handle, _event| {});
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tauri v1 `SystemTray` / `SystemTrayMenu` / `CustomMenuItem` | Tauri v2 `TrayIconBuilder` / `Menu` / `MenuItem` | Tauri 2.0 release | Completely different API surface. v1 code does not compile. |
| `trayIcon` in `tauri.conf.json` | Programmatic `TrayIconBuilder` only | Tauri 2.x best practice | Config-based causes duplicate icon bugs |
| `.run()` convenience | `.build()` + `.run()` for RunEvent access | Always available in Tauri 2 | Required to distinguish Cmd+Q from close button |

## Open Questions

1. **DoubleClick + show_menu_on_left_click Interaction**
   - What we know: Both are supported APIs. DoubleClick works on macOS (Issue #11413 was user error).
   - What's unclear: Whether `show_menu_on_left_click(true)` prevents DoubleClick events from firing (since the first click opens menu).
   - Recommendation: Test during implementation. If conflicting, fall back to menu-only interaction (single click opens menu with "打开主窗口"). The menu item achieves the same goal.

2. **Cmd+Q Behavior with .build()+.run() Pattern**
   - What we know: `CloseRequested` fires for window close button. `RunEvent::ExitRequested` fires for Cmd+Q.
   - What's unclear: Whether `prevent_close()` in `on_window_event` prevents `ExitRequested` from firing (since the window is never actually destroyed).
   - Recommendation: Test both paths during implementation. The CONTEXT.md already accepts the fallback (hide-to-tray for both).

3. **Tray Icon Design**
   - What we know: Must be monochrome, transparent background, 44x44px for @2x.
   - What's unclear: Exact glyph design.
   - Recommendation: Use a simple terminal/CLI-inspired glyph or the app's silhouette. Can iterate on design later without code changes (just replace the PNG file).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[cfg(test)]` + `cargo test` |
| Config file | None (standard Cargo test runner) |
| Quick run command | `cd src-tauri && cargo test` |
| Full suite command | `cd src-tauri && cargo test` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TRAY-01 | Tray icon appears in menu bar | manual-only | N/A (requires macOS GUI) | N/A |
| TRAY-02 | Close hides window, app persists | manual-only | N/A (requires window events) | N/A |
| TRAY-03 | Accessory/Regular mode toggle | manual-only | N/A (requires macOS Dock) | N/A |
| MENU-01 | "打开主窗口" shows/focuses window | manual-only | N/A (requires window + tray) | N/A |
| MENU-02 | "退出" fully exits app | manual-only | N/A (requires running app) | N/A |

**Justification for manual-only:** All Phase 6 requirements involve macOS GUI interactions (tray icon rendering, window show/hide, Dock visibility, ActivationPolicy). These cannot be unit-tested. The tray module functions (`create_tray_menu`, `handle_tray_menu_event`, `apply_tray_policy`) all require an `AppHandle` which is only available in a running Tauri application. The cc-switch reference also has no unit tests for tray logic.

### Sampling Rate

- **Per task commit:** `cd src-tauri && cargo test` (verify no regressions to existing tests)
- **Per task commit:** `cd src-tauri && cargo build` (verify tray code compiles)
- **Per wave merge:** Manual testing checklist (see below)
- **Phase gate:** All manual test items pass before `/gsd:verify-work`

### Manual Testing Checklist (Phase Gate)

1. `cargo tauri dev` -- tray icon appears in menu bar
2. Switch macOS to dark mode -- tray icon adapts color
3. Click tray icon -- menu opens with "打开主窗口" and "退出"
4. Click "退出" -- app fully exits
5. Relaunch, click window close button (red X) -- window hides, tray icon remains
6. After hide: dock icon gone, app not in Cmd+Tab
7. Click "打开主窗口" -- window appears, focused, dock icon returns
8. Double-click tray icon -- window appears (if supported)
9. Cmd+Q -- app quits (if distinction works) OR hides to tray (fallback)
10. `cargo tauri build` -- verify tray works in release build

### Wave 0 Gaps

- [ ] `src-tauri/icons/tray/tray-icon-template.png` -- 44x44 monochrome template icon asset
- [ ] Verify `cargo build` succeeds after adding `tray-icon` and `image-png` features

## Sources

### Primary (HIGH confidence)
- [Tauri 2 System Tray Guide](https://v2.tauri.app/learn/system-tray/) -- official setup, API overview
- [TrayIconEvent Rust docs](https://docs.rs/tauri/latest/tauri/tray/enum.TrayIconEvent.html) -- DoubleClick variant, event fields
- [App::run Rust docs](https://docs.rs/tauri/latest/tauri/struct.App.html) -- `.build()+.run()` pattern for RunEvent access
- [RunEvent Rust docs](https://docs.rs/tauri/latest/tauri/enum.RunEvent.html) -- ExitRequested variant
- cc-switch `src-tauri/src/tray.rs` -- `apply_tray_policy()`, `handle_tray_menu_event()`, `create_tray_menu()`, `TrayTexts` (direct code inspection)
- cc-switch `src-tauri/src/lib.rs` lines 231-249 -- `on_window_event(CloseRequested)` pattern, `TrayIconBuilder` setup at lines 685-719 (direct code inspection)
- cc-switch `src-tauri/Cargo.toml` -- confirmed features: `tray-icon`, `image-png` on `tauri = "2.8.2"`
- `.planning/research/STACK.md` -- comprehensive API table for tray, menu, window lifecycle, dock visibility
- `.planning/research/ARCHITECTURE.md` -- data flow diagrams, integration points, implementation skeleton
- `.planning/research/PITFALLS.md` -- 5 critical pitfalls with avoidance strategies

### Secondary (MEDIUM confidence)
- [GitHub Discussion #8341](https://github.com/tauri-apps/tauri/discussions/8341) -- Cmd+Q capture on macOS, ExitRequested vs CloseRequested distinction
- [GitHub Issue #11413](https://github.com/tauri-apps/tauri/issues/11413) -- TrayIconEvent on macOS (closed: user error, not a real bug)
- [GitHub Discussion #2684](https://github.com/tauri-apps/tauri/discussions/2684) -- close-to-tray community patterns
- [GitHub Issue #10912](https://github.com/tauri-apps/tauri/issues/10912) -- duplicate tray icon from dual config

### Tertiary (LOW confidence)
- DoubleClick + show_menu_on_left_click interaction -- no direct documentation found, needs runtime testing

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - verified via cc-switch Cargo.toml, Tauri docs, and prior v1.1 research
- Architecture: HIGH - cc-switch provides proven reference; prior research has full data flow diagrams
- Pitfalls: HIGH - 5 pitfalls documented with avoidance strategies from multiple sources
- Cmd+Q distinction: MEDIUM - API exists (RunEvent::ExitRequested) but macOS-specific behavior needs runtime validation
- DoubleClick interaction: LOW - needs runtime testing

**Research date:** 2026-03-13
**Valid until:** 2026-04-13 (Tauri 2 tray API is stable)

---
*Phase: 06-tray-foundation*
*Research completed: 2026-03-13*
