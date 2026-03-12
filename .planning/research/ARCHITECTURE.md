# Architecture Research: System Tray Integration

**Domain:** System tray integration for existing Tauri 2 desktop app (CLIManager v1.1)
**Researched:** 2026-03-12
**Confidence:** HIGH (Tauri 2 tray API is stable and well-documented; cc-switch reference implementation provides proven patterns; existing codebase architecture is fully analyzed)

## System Overview (Before vs After)

### Before (v1.0) -- Window-only

```
+--------------------------------------------------+
|                  React Frontend                   |
|  ProviderList -- ProviderForm -- SettingsPanel    |
+--------------------------------------------------+
|              Tauri IPC (invoke + emit)            |
+--------------------------------------------------+
|               Rust Backend                        |
|  commands/provider -- commands/onboarding         |
|  storage/icloud ---- storage/local                |
|  adapter/claude ---- adapter/codex                |
|  watcher (FSEvents) ---- SelfWriteTracker         |
+--------------------------------------------------+
|  iCloud Drive (providers/*.json)                  |
|  ~/.cli-manager/local.json                        |
|  ~/.claude/settings.json                          |
|  ~/.codex/{auth.json, config.toml}                |
+--------------------------------------------------+
```

### After (v1.1) -- Window + Tray

```
+--------------------------------------------------+
|                  React Frontend                   |
|  ProviderList -- ProviderForm -- SettingsPanel    |
+--------------------------------------------------+
|              Tauri IPC (invoke + emit)            |
+----------+---------------------------------------+
|  System  |            Rust Backend                |
|  Tray    |  commands/provider -- commands/onboard |
|  Module  |  storage/icloud ---- storage/local     |
|  (NEW)   |  adapter/claude ---- adapter/codex     |
|          |  watcher (FSEvents) -- SelfWriteTracker|
+----------+---------------------------------------+
|  iCloud Drive (providers/*.json)                  |
|  ~/.cli-manager/local.json                        |
|  ~/.claude/settings.json                          |
|  ~/.codex/{auth.json, config.toml}                |
+--------------------------------------------------+
```

Key architectural insight: The tray module sits alongside the existing backend, consuming the same storage and adapter layers. It introduces NO new data paths, NO new state stores, and NO new persistence. It is purely a new UI surface that reads from and writes to the existing storage layer.

## New vs Modified Components

### New: `src-tauri/src/tray.rs`

**Responsibility:** Tray icon lifecycle, dynamic menu construction, menu event handling, i18n labels.

**Public API:**

| Function | Purpose | Called By |
|----------|---------|-----------|
| `create_tray_menu(app: &AppHandle) -> Result<Menu<Wry>>` | Build menu from current storage state | `lib.rs` setup, `update_tray_menu` |
| `update_tray_menu(app: &AppHandle)` | Rebuild and replace the menu | Watcher, frontend command, tray event handler |
| `handle_tray_menu_event(app: &AppHandle, event_id: &str)` | Dispatch menu item clicks | TrayIconBuilder callback |

**Does NOT contain:** Provider switching logic. It calls existing internal functions from `commands::provider` for that.

### Modified: `src-tauri/src/lib.rs`

| Change | What | Why |
|--------|------|-----|
| Add `mod tray;` | Declare new module | Standard Rust module registration |
| In `setup()` | Build `TrayIconBuilder`, register event handlers | Initialize tray at startup |
| Add `.on_window_event()` | Intercept `CloseRequested`, hide window instead of quit | App must stay alive in tray |
| Add `refresh_tray_menu` command | Register new Tauri command | Frontend needs to trigger tray rebuild after CRUD |

### Modified: `src-tauri/Cargo.toml`

Add `"tray-icon"` and `"image-png"` to tauri features:
```toml
tauri = { version = "2", features = ["tray-icon", "image-png"] }
```

### Modified: `src-tauri/src/watcher/mod.rs`

Add one line in `process_events()` after the existing `app.emit("providers-changed", ...)`:
```rust
crate::tray::update_tray_menu(app_handle);
```

This ensures the tray menu reflects provider changes from iCloud sync.

### Unchanged Components

| Component | Why Unchanged |
|-----------|---------------|
| `commands/provider.rs` | Tray calls internal functions directly, no new commands needed for switching |
| `storage/icloud.rs` | Tray reads providers via existing `list_providers()` |
| `storage/local.rs` | Tray reads active state via existing `read_local_settings()` |
| `adapter/claude.rs` | Switching reuses existing `_set_active_provider_in` which calls adapters |
| `adapter/codex.rs` | Same as above |
| `watcher/self_write.rs` | Unchanged tracking logic |
| `provider.rs` | Data model unchanged |
| `error.rs` | No new error variants needed (reuse `AppError::Validation` for menu errors) |

## Data Flow

### Flow 1: Tray Menu Build (startup + after changes)

```
list_providers() --------------------------+
                                           +---> create_tray_menu()
read_local_settings().active_providers ----+          |
                                            Menu with CheckMenuItems
                                            (checked = active provider per CLI)
                                                      |
                                            TrayIcon.set_menu(menu)
```

The menu is built synchronously from the same storage functions the frontend uses. Each provider becomes a `CheckMenuItem` with `id = "{cli_id}_{provider_id}"`. The active provider for each CLI gets `checked = true`.

### Flow 2: Tray Provider Switch

```
User clicks tray menu item
    |
    v
on_menu_event(event_id = "claude_abc-123")
    |
    +---> Parse: cli_id="claude", provider_id="abc-123"
    |
    +---> _set_active_provider_in(...)    <-- REUSE existing internal function
    |       +-- get_provider_in(...)
    |       +-- patch_provider_for_cli(...)
    |       +-- write_local_settings(...)
    |
    +---> update_tray_menu(app)           <-- Rebuild menu with new check state
    |
    +---> app.emit("provider-switched")   <-- Notify frontend if window is open
```

Critical design decision: The tray calls the same internal `_set_active_provider_in` function that the `set_active_provider` Tauri command uses. This avoids duplicating validation, patching, and persistence logic. The function signature:

```rust
fn _set_active_provider_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    cli_id: String,
    provider_id: Option<String>,
    adapter: Option<Box<dyn CliAdapter>>,
) -> Result<LocalSettings, AppError>
```

Currently this is `fn` (crate-private), which is exactly what we need since `tray.rs` is in the same crate.

### Flow 3: External Change Triggers Tray Update

```
iCloud sync changes provider file
    |
    v
FSEvents watcher fires
    |
    v
process_events() in watcher/mod.rs
    +-- filter, dedup, skip self-writes     (existing)
    +-- sync_changed_active_providers()     (existing re-patch)
    +-- app.emit("providers-changed")       (existing frontend notify)
    +-- tray::update_tray_menu(app)         (NEW: rebuild tray menu)
```

The watcher already has `AppHandle`. Adding one call to `tray::update_tray_menu()` is sufficient.

### Flow 4: Window Close -> Hide to Tray

```
User clicks window X button
    |
    v
on_window_event(CloseRequested { api, .. })
    |
    +-- api.prevent_close()
    +-- window.hide()
    |
    +-- [macOS only] app.set_activation_policy(Accessory)
        +-- Hides Dock icon, app lives in tray only
```

### Flow 5: Tray -> Show Window

```
User clicks "Open Main Window" in tray menu
    |
    v
handle_tray_menu_event("show_main")
    |
    +-- window.unminimize()
    +-- window.show()
    +-- window.set_focus()
    |
    +-- [macOS only] app.set_activation_policy(Regular)
        +-- Dock icon reappears
```

### Flow 6: Frontend CRUD -> Tray Refresh

```
User creates/edits/deletes provider in UI
    |
    v
Frontend calls invoke("create_provider", ...)   (existing)
    |
    v
Frontend calls invoke("refresh_tray_menu")       (NEW)
    |
    v
tray::update_tray_menu(app)
```

## Tray Menu Structure

```
+-----------------------------+
|  Open Main Window           |  <-- MenuItem, always enabled
+-----------------------------+
|  Claude Code                |  <-- MenuItem, disabled (section header)
|  * My Anthropic Direct      |  <-- CheckMenuItem, checked = active
|    OpenRouter                |  <-- CheckMenuItem, unchecked
|    Azure Proxy               |  <-- CheckMenuItem, unchecked
+-----------------------------+
|  Codex                      |  <-- MenuItem, disabled (section header)
|    My Codex Provider         |  <-- CheckMenuItem
+-----------------------------+
|  Quit                       |  <-- MenuItem
+-----------------------------+
```

Menu item ID scheme:
- `show_main` -- fixed: open main window
- `quit` -- fixed: exit app
- `{cli_id}_header` -- fixed per CLI: non-clickable section header
- `{cli_id}_empty` -- fixed per CLI: "no providers" hint
- `{cli_id}_{provider_id}` -- dynamic: provider switch target

Provider sorting: Use `created_at` ascending (same as `list_providers_in()` already returns).

## Implementation Skeleton

### `tray.rs`

```rust
use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuItem};
use tauri::{AppHandle, Manager, Wry};

use crate::error::AppError;
use crate::storage::icloud::list_providers;
use crate::storage::local::read_local_settings;

/// i18n labels for tray menu
struct TrayTexts {
    show_main: &'static str,
    quit: &'static str,
    no_providers: &'static str,
}

impl TrayTexts {
    fn from_language(lang: &str) -> Self {
        if lang.starts_with("en") {
            Self {
                show_main: "Open Main Window",
                quit: "Quit",
                no_providers: "  (No providers yet)",
            }
        } else {
            Self {
                show_main: "\u{6253}\u{5F00}\u{4E3B}\u{754C}\u{9762}",
                quit: "\u{9000}\u{51FA}",
                no_providers: "  (\u{6682}\u{65E0} Provider)",
            }
        }
    }
}

pub fn create_tray_menu(app: &AppHandle) -> Result<Menu<Wry>, AppError> {
    let settings = read_local_settings()?;
    let providers = list_providers()?;
    let lang = settings.language.as_deref().unwrap_or("zh-CN");
    let texts = TrayTexts::from_language(lang);

    let mut builder = MenuBuilder::new(app);

    // "Open Main Window"
    let show_item = MenuItem::with_id(app, "show_main", texts.show_main, true, None::<&str>)
        .map_err(menu_err)?;
    builder = builder.item(&show_item).separator();

    // Provider sections grouped by cli_id
    for (cli_id, header_label) in [("claude", "Claude Code"), ("codex", "Codex")] {
        let cli_providers: Vec<_> = providers.iter()
            .filter(|p| p.cli_id == cli_id)
            .collect();

        let header = MenuItem::with_id(app, format!("{cli_id}_header"), header_label, false, None::<&str>)
            .map_err(menu_err)?;
        builder = builder.item(&header);

        let active_id = settings.active_providers
            .get(cli_id)
            .and_then(|v| v.as_ref());

        if cli_providers.is_empty() {
            let empty = MenuItem::with_id(app, format!("{cli_id}_empty"), texts.no_providers, false, None::<&str>)
                .map_err(menu_err)?;
            builder = builder.item(&empty);
        } else {
            for provider in cli_providers {
                let is_active = active_id.map_or(false, |id| id == &provider.id);
                let item = CheckMenuItem::with_id(
                    app, format!("{}_{}", cli_id, provider.id),
                    &provider.name, true, is_active, None::<&str>,
                ).map_err(menu_err)?;
                builder = builder.item(&item);
            }
        }
        builder = builder.separator();
    }

    // Quit
    let quit = MenuItem::with_id(app, "quit", texts.quit, true, None::<&str>)
        .map_err(menu_err)?;
    builder = builder.item(&quit);

    builder.build().map_err(menu_err)
}

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

pub fn handle_tray_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "show_main" => show_main_window(app),
        "quit" => { log::info!("Quit from tray"); app.exit(0); }
        id => {
            // Parse "{cli_id}_{provider_id}" -- find first underscore
            // Skip header/empty items
            if id.ends_with("_header") || id.ends_with("_empty") { return; }
            if let Some((cli_id, provider_id)) = id.split_once('_') {
                handle_provider_click(app, cli_id, provider_id);
            }
        }
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        #[cfg(target_os = "macos")]
        {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        }
    }
}

fn handle_provider_click(app: &AppHandle, cli_id: &str, provider_id: &str) {
    // Reuse existing internal switching logic
    let providers_dir = match crate::storage::icloud::get_icloud_providers_dir() {
        Ok(d) => d,
        Err(e) => { log::error!("Tray switch failed: {e}"); return; }
    };
    let settings_path = crate::storage::local::get_local_settings_path();

    match crate::commands::provider::_set_active_provider_in(
        &providers_dir, &settings_path,
        cli_id.to_string(), Some(provider_id.to_string()), None,
    ) {
        Ok(_) => {
            log::info!("Tray: switched {cli_id} to {provider_id}");
            update_tray_menu(app);
            // Notify frontend
            let _ = app.emit("provider-switched", serde_json::json!({
                "cli_id": cli_id, "provider_id": provider_id
            }));
        }
        Err(e) => log::error!("Tray switch failed: {e}"),
    }
}

fn menu_err(e: impl std::fmt::Display) -> AppError {
    AppError::Validation(format!("menu error: {e}"))
}
```

**Note on `_set_active_provider_in` visibility:** This function is currently `fn` (private to `commands::provider`). It needs to be made `pub(crate)` so `tray.rs` can call it. This is a one-word change.

### `lib.rs` Changes

```rust
mod tray;  // ADD

// In setup():
let menu = tray::create_tray_menu(app.handle())?;

let _tray = tauri::tray::TrayIconBuilder::with_id("main")
    .icon(app.default_window_icon().unwrap().clone())
    .icon_as_template(true)  // macOS dark/light mode adaptation
    .menu(&menu)
    .show_menu_on_left_click(true)
    .on_menu_event(|app, event| {
        tray::handle_tray_menu_event(app, &event.id.0);
    })
    .on_tray_icon_event(|_tray, _event| {
        // Left click shows menu via show_menu_on_left_click(true)
    })
    .build(app)?;

// ADD to invoke_handler:
commands::provider::refresh_tray_menu,

// ADD before .setup():
.on_window_event(|window, event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
        #[cfg(target_os = "macos")]
        {
            let _ = window.app_handle()
                .set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    }
})
```

### New Tauri Command: `refresh_tray_menu`

```rust
// In commands/provider.rs (or commands/mod.rs)
#[tauri::command]
pub fn refresh_tray_menu(app_handle: tauri::AppHandle) -> Result<(), String> {
    crate::tray::update_tray_menu(&app_handle);
    Ok(())
}
```

The frontend calls this after any CRUD operation that changes provider data.

## Architectural Patterns

### Pattern 1: Menu-as-Snapshot (Full Rebuild)

**What:** The tray menu is rebuilt entirely from storage state each time it needs updating. No incremental menu patching.

**Why:** Tauri 2 menus are immutable after build. To update, you replace the entire menu via `tray.set_menu(Some(new_menu))`. This aligns with the app's file-based storage (re-read providers, rebuild menu).

**Trade-offs:** Rebuilds read all provider files from disk each time. With the expected scale (3-20 providers), this is negligible (<1ms). Avoids complex state synchronization between menu items and storage.

### Pattern 2: Shared Internal Functions (No Logic Duplication)

**What:** Tray event handlers call the same `_set_active_provider_in()` internal function that the Tauri command uses.

**Why:** The `#[tauri::command]` functions are designed for IPC. The tray handler is in the same Rust crate and can call the internal `_in` variant directly. This keeps switching logic in one place: one code path for both UI-initiated and tray-initiated switches.

**Trade-offs:** Requires `pub(crate)` visibility on the `_in` function (currently `fn`). This is a minor visibility change, not an API change.

### Pattern 3: Event-Driven Cross-Surface Sync

**What:** After a tray-initiated switch, the backend emits `"provider-switched"`. After a frontend-initiated CRUD, the frontend calls `refresh_tray_menu`. Both surfaces stay in sync via events and commands.

**Why:** The window may or may not be visible. Events are cheap and the frontend already has listener infrastructure from the file watcher.

**Bidirectional sync:**
- Tray -> Frontend: `app.emit("provider-switched", ...)`
- Frontend -> Tray: `invoke("refresh_tray_menu")`
- iCloud -> Both: watcher calls `update_tray_menu` + emits `providers-changed`

## Window Lifecycle Changes

| Scenario | Before (v1.0) | After (v1.1) |
|----------|---------------|--------------|
| User closes window | App exits | Window hides, app stays in tray |
| User clicks tray "Open" | N/A | Window shows, gets focus, Dock icon appears |
| User clicks tray "Quit" | N/A | App fully exits (`app.exit(0)`) |
| macOS Dock icon | Always visible while app runs | Hidden when window is hidden (ActivationPolicy::Accessory) |
| App startup | Window opens | Window opens + tray icon appears simultaneously |
| All windows closed | App exits (default Tauri behavior) | App continues (CloseRequested intercepted) |

### macOS ActivationPolicy Details

- `ActivationPolicy::Regular` -- App appears in Dock, Cmd+Tab, has menu bar. Use when window is visible.
- `ActivationPolicy::Accessory` -- App does NOT appear in Dock or Cmd+Tab. Only tray icon visible. Use when window is hidden.

This is controlled via `app.set_activation_policy()` (Tauri 2 API), not via Info.plist. It can be toggled at runtime.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Tray Holding Its Own Provider Cache

**What people do:** Store a copy of provider data in the tray module's state, sync it separately.
**Why it is wrong:** Creates a second source of truth. If the cache diverges from storage (missed update, iCloud sync), the tray shows stale data.
**Do this instead:** Always read from storage when building the menu. `list_providers()` + `read_local_settings()` are fast reads of small JSON files.

### Anti-Pattern 2: Duplicating Switch Logic in Tray Handler

**What people do:** Write provider-switching code (validation, adapter patching, settings update) directly in the tray event handler.
**Why it is wrong:** Diverges from the tested switching path. Bugs get fixed in one place but not the other.
**Do this instead:** Call `_set_active_provider_in()` -- the same function the Tauri command uses.

### Anti-Pattern 3: Keeping Default App Exit on Window Close

**What people do:** Keep `.run(tauri::generate_context!())` which exits when the last window closes.
**Why it is wrong:** Closing the window kills the app, defeating the purpose of a system tray.
**Do this instead:** Intercept `CloseRequested`, call `api.prevent_close()`, hide the window.

### Anti-Pattern 4: Provider ID Parsing Fragility

**What people do:** Use `split_once('_')` on menu IDs like `"claude_abc-123"` and assume the first part is always the CLI ID.
**Why it is wrong:** If a provider UUID happens to contain no hyphens, or if a CLI ID contains underscores, parsing breaks.
**Do this instead:** Use the known CLI ID prefixes (`"claude_"`, `"codex_"`) and `strip_prefix()` to extract the provider ID. The cc-switch reference code uses this pattern correctly.

## Integration Points

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `tray.rs` -> `commands::provider` | Direct `fn` call (`_set_active_provider_in`) | Same crate, needs `pub(crate)` |
| `tray.rs` -> `storage::icloud` | Direct `fn` call (`list_providers`) | Already `pub` |
| `tray.rs` -> `storage::local` | Direct `fn` call (`read_local_settings`) | Already `pub` |
| `watcher/mod.rs` -> `tray.rs` | Direct `fn` call (`update_tray_menu`) | New call in `process_events` |
| `tray.rs` -> Frontend | Event emission (`provider-switched`) | Frontend may be hidden |
| Frontend -> `tray.rs` | Tauri command (`refresh_tray_menu`) | After provider CRUD |

### Tray Icon Asset

macOS best practice: Use a template image (single-color, transparency-based) so the system can adapt it for light/dark mode. The icon should be:
- 22x22 points (44x44 pixels @2x, 66x66 pixels @3x)
- Single color (black/white, system inverts as needed)
- Named with `Template` suffix or flagged with `.icon_as_template(true)`

For v1.1 MVP, using the app's default window icon via `app.default_window_icon().unwrap().clone()` is acceptable. A proper template icon can be added as polish.

## Suggested Build Order

Build in this order to maintain a working app at each step:

| Step | What | Depends On | Deliverable |
|------|------|------------|-------------|
| 1 | `Cargo.toml` features + minimal `tray.rs` + `lib.rs` setup | Nothing | Tray icon appears with "Quit" menu item |
| 2 | Full `create_tray_menu` with provider sections | Step 1 | Providers visible in tray with active state |
| 3 | `handle_tray_menu_event` for provider switching | Step 2 | Switching from tray patches CLI configs |
| 4 | Bidirectional sync (emit events + `refresh_tray_menu` command) | Step 3 | Tray and frontend stay in sync |
| 5 | Window lifecycle (`on_window_event`, hide/show, ActivationPolicy) | Step 1 | Close-to-tray works |
| 6 | i18n labels + tray icon asset + edge cases | Steps 4+5 | Production polish |

Steps 4 and 5 can be developed in parallel after step 3.

## Sources

- [Tauri 2 System Tray documentation](https://v2.tauri.app/learn/system-tray/)
- [TrayIconBuilder API reference (Tauri 2)](https://docs.rs/tauri/2.0.0/tauri/tray/struct.TrayIconBuilder.html)
- [Tauri 2 crate feature flags](https://docs.rs/crate/tauri/latest/features)
- [Tauri 2 migration guide (tray changes)](https://v2.tauri.app/start/migrate/from-tauri-1/)
- cc-switch reference implementation: `cc-switch/src-tauri/src/tray.rs` (447 lines, full tray with multi-app sections)
- cc-switch tray setup: `cc-switch/src-tauri/src/lib.rs` (TrayIconBuilder at line 685, on_window_event at line 231)
- Existing CLIManager source: `src-tauri/src/lib.rs`, `src-tauri/src/commands/provider.rs`, `src-tauri/src/watcher/mod.rs`

---
*Architecture research for: CLIManager v1.1 System Tray Integration*
*Researched: 2026-03-12*
