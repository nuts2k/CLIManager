# Phase 7: Provider Menu and Switching - Research

**Researched:** 2026-03-13
**Domain:** Tauri 2 dynamic tray menu with CheckMenuItem, provider switching, i18n, event-driven menu refresh
**Confidence:** HIGH

## Summary

Phase 7 extends the Phase 6 tray foundation (static "Open Main Window" + "Quit" menu) into a dynamic provider menu. The existing codebase provides all building blocks: `list_providers()` for provider data, `read_local_settings()` for active provider state, `_set_active_provider_in()` for switching logic, and the file watcher for iCloud sync events. The tray module (`tray.rs`) needs to be extended -- not rewritten -- with dynamic menu construction, provider click handling, menu rebuild triggers, and a lightweight Rust i18n map.

The core technical challenge is keeping the tray menu synchronized with three sources of state change: (1) user clicks in the tray itself, (2) provider CRUD operations from the main window frontend, and (3) iCloud file sync detected by the watcher. Tauri 2 menus are immutable after `.build()` -- there is no API to modify individual menu items in place. The correct approach is full menu rebuild via `tray.set_menu(Some(new_menu))`, triggered from all three change sources.

The implementation requires zero new dependencies. All needed APIs (`CheckMenuItem`, `MenuItem`, `MenuBuilder`, `tray_by_id`, `set_menu`) are already available in the `tauri` crate with the `tray-icon` feature that Phase 6 enabled. The `language` field already exists in `LocalSettings`. The primary code changes are in `tray.rs` (dynamic menu construction + event handling), `watcher/mod.rs` (add tray rebuild call), `lib.rs` (pass dynamic menu at startup + register new command), and `commands/provider.rs` (make `_set_active_provider_in` `pub(crate)` + add `refresh_tray_menu` command).

**Primary recommendation:** Extend `tray.rs` with a `create_tray_menu(app) -> Menu` that reads providers + settings from storage, groups by CLI, and uses `CheckMenuItem` for active state; add an `update_tray_menu(app)` helper that rebuilds and sets the menu via `tray_by_id("main").set_menu()`. Call this from three places: tray click handler, watcher `process_events`, and a new `refresh_tray_menu` Tauri command that the frontend invokes after CRUD.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Use disabled MenuItem as section headers for CLI groups (e.g., disabled "Claude Code", disabled "Codex")
- Claude Code group appears first, Codex second (Claude Code is the primary use case)
- Within each group, active Provider sorts first, remaining sorted by name
- Standard layout: "Open Main Window" -> separator -> [Claude Code] -> Providers... -> [Codex] -> Providers... -> separator -> "Quit"
- Hide CLI groups that have no Providers (don't show empty groups)
- When all CLIs have zero Providers, hide the entire Provider section (menu shows only "Open Main Window" and "Quit")
- Show Provider name only (the `name` field), no model/protocol suffix
- Use Tauri CheckMenuItem for active Provider marking -- native system checkmark
- Clicking a CheckMenuItem triggers Provider switch (patch CLI config) and updates checkmark position
- Sync language immediately when user switches language in frontend settings
- Rust maintains its own simplified translation map (only the few menu strings: "Open Main Window", "Quit", CLI group titles)
- Frontend notifies Rust via Tauri command when language changes, Rust rebuilds menu
- Language preference persisted to local.json (device-local layer) so Rust can read it on startup
- This requires adding a `language` field to LocalSettings and a new Tauri command for language change notification

### Claude's Discretion
- Exact Rust translation map structure (HashMap, match, or enum)
- Menu rebuild implementation details (replace menu vs rebuild TrayIcon)
- Error handling for switch failures in tray context
- How to read current active providers from local.json for menu construction

### Deferred Ideas (OUT OF SCOPE)
- Launch at login (Login Items) -- future phase or v1.2
- Tray icon tooltip showing active Provider name -- TRAY-04 (v2)
- Tray icon state variants (active vs no-provider) -- TRAY-05 (v2)
- Provider menu item showing model name for disambiguation -- PROV-04 (v2)
- Global hotkey for Provider switching -- KEY-01 (v2)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PROV-01 | Tray menu lists all Providers grouped by CLI, active Provider shows checkmark | Dynamic menu construction with `list_providers()` + `read_local_settings()`, `CheckMenuItem::with_id()` for check state, disabled `MenuItem` for group headers |
| PROV-02 | Clicking a Provider in tray switches immediately without opening main window | Tray `on_menu_event` handler parses menu item ID, calls `_set_active_provider_in()` via `spawn_blocking`, rebuilds menu |
| PROV-03 | Provider add/edit/delete in main window or iCloud sync triggers tray menu auto-refresh | Three refresh triggers: watcher `process_events` calls `update_tray_menu()`, frontend calls `refresh_tray_menu` command after CRUD, tray handler rebuilds after switch |
| MENU-03 | Tray menu text follows app language setting (Chinese or English) | `TrayTexts::from_language()` reads `language` from `LocalSettings`, new `set_language` command notifies Rust to rebuild menu |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri | 2.x (features: tray-icon, image-png) | Tray icon, Menu, CheckMenuItem, event system | Already in Cargo.toml; provides all needed menu APIs |

### Key Tauri 2 APIs Used
| API | Purpose |
|-----|---------|
| `CheckMenuItem::with_id(app, id, label, enabled, checked, accel)` | Create checkable menu item with initial check state |
| `MenuItem::with_id(app, id, label, enabled, accel)` | Create regular menu item (for headers, fixed items) |
| `MenuBuilder::new(app).item(&item).separator().build()` | Build immutable menu from items |
| `app.tray_by_id("main")` | Retrieve existing tray icon by ID |
| `tray.set_menu(Some(new_menu))` | Replace the tray's menu at runtime |
| `tauri::async_runtime::spawn_blocking(move \|\| { ... })` | Run blocking I/O off the main thread in tray event handlers |
| `app.emit("event-name", payload)` | Emit events to frontend (works even if window is hidden) |

### No New Dependencies
Zero additions to Cargo.toml. All APIs come from the existing `tauri` crate with features already enabled in Phase 6.

## Architecture Patterns

### Recommended Changes to Existing Structure

```
src-tauri/src/
├── tray.rs              # EXTEND: dynamic menu + event handler + i18n
├── lib.rs               # MODIFY: dynamic menu at startup, register new commands
├── commands/
│   └── provider.rs      # MODIFY: pub(crate) on _set_active_provider_in, add refresh_tray_menu + set_language commands
├── storage/
│   └── local.rs         # ALREADY has language field in LocalSettings
├── watcher/
│   └── mod.rs           # MODIFY: add tray rebuild call after providers-changed emit
└── (everything else)    # UNCHANGED
```

### Pattern 1: Menu-as-Snapshot (Full Rebuild)

**What:** The tray menu is built entirely from storage state each time it needs updating. No incremental menu patching.

**When to use:** Always -- Tauri 2 menus are immutable after `.build()`. The only way to update is to rebuild and call `set_menu()`.

**Example:**
```rust
// Source: Tauri 2 docs + cc-switch reference tray.rs
pub fn create_tray_menu(app: &AppHandle) -> Result<Menu<Wry>, AppError> {
    let settings = read_local_settings()?;
    let providers = list_providers()?;
    let lang = settings.language.as_deref().unwrap_or("zh");
    let texts = TrayTexts::from_language(lang);

    let mut builder = MenuBuilder::new(app);
    // ... build items ...
    builder.build().map_err(|e| AppError::Validation(format!("menu: {e}")))
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
```

**Why this works:** Rebuilding reads all provider files + local.json from disk. With expected scale (2-10 providers), this is <1ms. Avoids complex in-memory caching and state synchronization.

### Pattern 2: Shared Internal Functions (No Logic Duplication)

**What:** Tray event handlers call the same `_set_active_provider_in()` internal function that the Tauri `set_active_provider` command uses.

**When to use:** For provider switching from tray clicks.

**Required change:** Make `_set_active_provider_in` visibility `pub(crate)` (currently `fn` private). One-word change in `commands/provider.rs` line 143.

**Example:**
```rust
// Source: existing commands/provider.rs architecture + cc-switch tray.rs pattern
fn handle_provider_click(app: &AppHandle, cli_id: &str, provider_id: &str) {
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
            let _ = app.emit("provider-switched", serde_json::json!({
                "cli_id": cli_id, "provider_id": provider_id
            }));
        }
        Err(e) => {
            log::error!("Tray switch failed: {e}");
            // Rebuild menu to reset checkmark to correct state
            update_tray_menu(app);
        }
    }
}
```

### Pattern 3: Event-Driven Cross-Surface Sync

**What:** Three triggers keep tray and frontend synchronized.

**Trigger map:**

| Source of Change | Tray Update Mechanism | Frontend Update Mechanism |
|------------------|-----------------------|---------------------------|
| Tray provider click | Handler rebuilds menu directly | `app.emit("provider-switched")` |
| Frontend CRUD | Frontend calls `invoke("refresh_tray_menu")` | Already updated (it initiated the change) |
| iCloud sync | Watcher calls `update_tray_menu()` in `process_events` | Watcher emits `"providers-changed"` (existing) |
| Language change | Frontend calls `invoke("set_language")` which rebuilds menu | Already updated (it initiated the change) |

### Pattern 4: Menu Item ID Scheme with Prefix Parsing

**What:** Each menu item gets a structured ID. Provider items use `{cli_id}_{provider_id}` format. Fixed items use known strings.

**Menu item IDs:**
| ID Pattern | Type | Handler |
|------------|------|---------|
| `show_main` | MenuItem | Show/focus main window |
| `quit` | MenuItem | Exit app |
| `claude_header` | MenuItem (disabled) | Non-interactive section header |
| `codex_header` | MenuItem (disabled) | Non-interactive section header |
| `claude_{uuid}` | CheckMenuItem | Switch Claude Code provider |
| `codex_{uuid}` | CheckMenuItem | Switch Codex provider |

**Parsing approach:** Use `strip_prefix("claude_")` and `strip_prefix("codex_")` (NOT `split_once('_')`) to extract provider ID. This avoids fragility if UUIDs or CLI IDs contain underscores.

```rust
// Source: cc-switch tray.rs handle_provider_tray_event pattern
fn parse_provider_event(event_id: &str) -> Option<(&str, &str)> {
    for (prefix, cli_id) in [("claude_", "claude"), ("codex_", "codex")] {
        if let Some(provider_id) = event_id.strip_prefix(prefix) {
            // Skip header/empty items
            if provider_id == "header" || provider_id == "empty" {
                return None;
            }
            return Some((cli_id, provider_id));
        }
    }
    None
}
```

### Pattern 5: Async Tray Event Handling via spawn_blocking

**What:** Tray menu event handlers use `tauri::async_runtime::spawn_blocking` to run provider switching off the main thread.

**Why:** Provider switching involves multiple file reads/writes (provider JSON, local.json, CLI config files). Running this on the main thread would block the UI.

```rust
// Source: cc-switch tray.rs lines 181-188
fn handle_tray_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "show_main" => show_main_window(app),
        "quit" => { log::info!("Quit from tray"); app.exit(0); }
        id => {
            if let Some((cli_id, provider_id)) = parse_provider_event(id) {
                let app_handle = app.clone();
                let cli_id = cli_id.to_string();
                let provider_id = provider_id.to_string();
                tauri::async_runtime::spawn_blocking(move || {
                    handle_provider_click(&app_handle, &cli_id, &provider_id);
                });
            }
        }
    }
}
```

### Pattern 6: Lightweight Rust i18n Map

**What:** A simple struct with `&'static str` fields for the ~5 tray menu strings. Read language from `LocalSettings.language` on each menu build.

**Recommendation:** Use a `match` on language string. Simpler than HashMap, zero allocation, compile-time checked.

```rust
// Source: cc-switch tray.rs TrayTexts pattern (adapted for CLIManager)
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
```

Note: CLI group headers ("Claude Code", "Codex") are brand names and stay the same in all languages.

### Anti-Patterns to Avoid

- **Caching providers in tray module state:** Creates second source of truth. Always read from storage on rebuild. Disk reads are <1ms at this scale.
- **Duplicating switch logic in tray handler:** Call `_set_active_provider_in()`, never rewrite the validation/patch/persist pipeline.
- **Using `split_once('_')` for menu ID parsing:** Breaks if provider UUID contains no hyphens. Use `strip_prefix("claude_")` / `strip_prefix("codex_")`.
- **Not rebuilding menu after switch failure:** If switching fails, the CheckMenuItem may have auto-toggled its visual state. Always rebuild to restore correct check state.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Provider switching logic | Custom tray-specific switching | `_set_active_provider_in()` from `commands/provider.rs` | Tested, handles validation, adapter patching, settings persistence |
| Active provider detection | New state tracking | `read_local_settings().active_providers` | Single source of truth, already used by frontend |
| Provider listing | New data access | `list_providers()` from `storage/icloud` | Already handles sorting, validation, malformed file skipping |
| Menu item check state | Manual toggle tracking | Rebuild entire menu from storage after each action | Tauri CheckMenuItem auto-toggles on click; rebuild ensures correctness |
| i18n framework for tray | Full i18n library in Rust | Simple `match` on language string | Only ~5 strings; full i18n library is over-engineering |

**Key insight:** Phase 7 is primarily a "wiring" phase. All the business logic (switching, listing, settings) already exists. The work is building a new UI surface (tray menu) that consumes existing functions and adding sync triggers.

## Common Pitfalls

### Pitfall 1: Stale Tray Menu After State Changes

**What goes wrong:** Menu shows wrong checkmark or outdated provider list after switching via main window, after iCloud sync, or after CRUD operations.

**Why it happens:** Three independent sources of state change exist, and any one of them forgetting to trigger a menu rebuild results in stale UI. Tauri 2 has no automatic menu refresh mechanism.

**How to avoid:** Create a single `update_tray_menu()` function and call it from exactly three places: (1) tray click handler after switch, (2) watcher `process_events` after re-patch, (3) `refresh_tray_menu` Tauri command called by frontend after CRUD.

**Warning signs:** Switching via main window doesn't update tray checkmark. Adding a provider doesn't add it to tray.

### Pitfall 2: CheckMenuItem Auto-Toggle on Click

**What goes wrong:** User clicks a provider in the tray, the OS auto-toggles the CheckMenuItem's visual check state before the event handler runs. If the handler fails (switch error) but doesn't rebuild the menu, the tray shows the wrong provider as active.

**Why it happens:** Tauri 2 `CheckMenuItem` (via the `muda` crate) auto-toggles its checked state on native click, independent of the application's event handling.

**How to avoid:** Always rebuild the menu after handling a click, whether the switch succeeded or failed. On success, the rebuild reflects the new correct state. On failure, the rebuild resets the check to the previous correct state.

**Warning signs:** After a failed switch, the tray shows a checkmark on the provider that was clicked (not the one that's actually active).

### Pitfall 3: Blocking Main Thread in Tray Event Handler

**What goes wrong:** Tray becomes unresponsive, spinning cursor appears while switching providers.

**Why it happens:** `_set_active_provider_in` involves multiple file reads/writes (provider JSON, local.json, CLI config files like `~/.claude/settings.json`). Running this synchronously in `on_menu_event` blocks the main thread.

**How to avoid:** Use `tauri::async_runtime::spawn_blocking` to run the switch + menu rebuild off the main thread. Clone `AppHandle` into the closure.

**Warning signs:** Brief UI freeze when clicking tray providers. More noticeable on slower disks.

### Pitfall 4: Menu ID Parsing Fragility

**What goes wrong:** Parsing `"claude_abc-123"` with `split_once('_')` works for UUID-style IDs, but breaks if provider IDs ever contain underscores.

**Why it happens:** Using generic string splitting instead of known-prefix matching.

**How to avoid:** Use `strip_prefix("claude_")` and `strip_prefix("codex_")` to extract provider ID. Skip IDs ending in `_header` or `_empty`.

### Pitfall 5: Provider Sorting Not Matching User Decision

**What goes wrong:** User decision says "active Provider sorts first, remaining sorted by name." But the existing `list_providers()` sorts by `created_at`. If the tray code uses `list_providers()` directly, the sort order will be wrong.

**Why it happens:** The user's decided sort order (active first, then by name) differs from the storage sort order (by created_at).

**How to avoid:** After getting providers from `list_providers()` and identifying the active provider, re-sort: active first, then remaining by `provider.name` alphabetically.

### Pitfall 6: Language Change Doesn't Rebuild Menu

**What goes wrong:** User switches language in settings, but tray menu still shows old language until next app restart.

**Why it happens:** `handleLanguageChange` in `SettingsPage.tsx` calls `updateSettings({ language: lang })` which writes to `local.json`, but nothing tells the Rust tray module to rebuild.

**How to avoid:** Add a new Tauri command (`set_language` or `notify_language_change`) that the frontend calls after language change. This command reads the new language from `local.json` and calls `update_tray_menu()`. Alternatively, the frontend can call the existing `refresh_tray_menu` command after the settings update, since `create_tray_menu` reads language from settings on every build.

## Code Examples

### Dynamic Menu Construction (Core of PROV-01)

```rust
// Source: Architecture from existing codebase + Tauri 2 API + cc-switch reference
pub fn create_tray_menu(app: &AppHandle) -> Result<Menu<Wry>, AppError> {
    let settings = read_local_settings()?;
    let all_providers = crate::storage::icloud::list_providers()?;
    let lang = settings.language.as_deref().unwrap_or("zh");
    let texts = TrayTexts::from_language(lang);

    let mut builder = MenuBuilder::new(app);

    // "Open Main Window"
    let show_item = MenuItem::with_id(app, "show_main", texts.show_main, true, None::<&str>)
        .map_err(menu_err)?;
    builder = builder.item(&show_item).separator();

    // CLI sections: Claude Code first, then Codex (user decision)
    let mut has_any_providers = false;
    for (cli_id, header_label) in [("claude", texts.claude_header), ("codex", texts.codex_header)] {
        let mut cli_providers: Vec<_> = all_providers.iter()
            .filter(|p| p.cli_id == cli_id)
            .collect();

        // Hide CLI groups with no providers (user decision)
        if cli_providers.is_empty() {
            continue;
        }

        has_any_providers = true;

        let active_id = settings.active_providers
            .get(cli_id)
            .and_then(|v| v.as_ref())
            .map(|s| s.as_str());

        // Sort: active first, then by name (user decision)
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
        let header = MenuItem::with_id(app, format!("{cli_id}_header"), header_label, false, None::<&str>)
            .map_err(menu_err)?;
        builder = builder.item(&header);

        // Provider items (CheckMenuItem)
        for provider in cli_providers {
            let is_active = active_id == Some(provider.id.as_str());
            let item = CheckMenuItem::with_id(
                app,
                format!("{cli_id}_{}", provider.id),
                &provider.name,  // name only, no model suffix (user decision)
                true,
                is_active,
                None::<&str>,
            ).map_err(menu_err)?;
            builder = builder.item(&item);
        }
    }

    // Only add separator before Quit if there were provider sections
    if has_any_providers {
        builder = builder.separator();
    }

    // "Quit"
    let quit = MenuItem::with_id(app, "quit", texts.quit, true, None::<&str>)
        .map_err(menu_err)?;
    builder = builder.item(&quit);

    builder.build().map_err(menu_err)
}
```

### Watcher Integration (Core of PROV-03)

```rust
// Source: existing watcher/mod.rs process_events function
// Add after existing app_handle.emit("providers-changed", &payload) call:
fn process_events(events: Vec<DebouncedEvent>, app_handle: &AppHandle) {
    // ... existing filter/dedup/self-write logic ...
    // ... existing sync_changed_active_providers call ...
    // ... existing app.emit("providers-changed", &payload) ...

    // NEW: Rebuild tray menu to reflect provider changes
    crate::tray::update_tray_menu(app_handle);
}
```

### Frontend Language Change + Tray Refresh

```typescript
// Source: existing SettingsPage.tsx handleLanguageChange
const handleLanguageChange = async (lang: string) => {
    await i18n.changeLanguage(lang);
    await updateSettings({ language: lang });
    // NEW: Notify Rust to rebuild tray menu with new language
    await invoke("refresh_tray_menu");
};
```

### Visibility Change: `_set_active_provider_in`

```rust
// In commands/provider.rs, line 143:
// Change from:
fn _set_active_provider_in(
// To:
pub(crate) fn _set_active_provider_in(
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Static tray menu (Phase 6) | Dynamic menu rebuilt from storage | Phase 7 | Menu reflects live provider state |
| Language field unused by Rust | Rust reads language for tray i18n | Phase 7 | Tray labels match app language |
| Frontend-only provider switching | Tray + Frontend both can switch | Phase 7 | Switch without opening window |

**Already current / no migration needed:**
- `LocalSettings.language` field already exists (added during v1.0)
- `tray-icon` and `image-png` features already enabled (Phase 6)
- `SelfWriteTracker` pattern already in place
- Watcher already emits `providers-changed` events

## Open Questions

1. **Frontend listener for `provider-switched` event**
   - What we know: The frontend currently listens for `providers-changed` (from watcher) but has no listener for `provider-switched` (from tray). When the tray switches a provider, the frontend's `useSettings` and `useProviders` hooks won't know about it.
   - What's unclear: Whether to add a new event listener in `useSyncListener` for `provider-switched`, or simply have the tray switch also emit `providers-changed`.
   - Recommendation: Emit `providers-changed` from the tray switch handler (same payload format). This reuses the existing frontend listener infrastructure without changes. Alternatively, add a `provider-switched` listener that calls `refreshSettings()`.

2. **Menu rebuild timing from `spawn_blocking`**
   - What we know: `spawn_blocking` moves the switch logic off the main thread. But `update_tray_menu` calls `tray.set_menu()` which may need to run on the main thread.
   - What's unclear: Whether Tauri 2's `set_menu` is thread-safe.
   - Recommendation: The cc-switch reference calls `create_tray_menu` + `tray.set_menu()` from within `spawn_blocking` without issues (lines 260, 308). Follow this proven pattern. If issues arise, use `app.run_on_main_thread()` for the `set_menu` call.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (cargo test) |
| Config file | Cargo.toml (existing) |
| Quick run command | `cd src-tauri && cargo test --lib` |
| Full suite command | `cd src-tauri && cargo test` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PROV-01-a | Provider sorting (active first, then by name) | unit | `cd src-tauri && cargo test tray::tests::test_provider_sorting -x` | Wave 0 |
| PROV-01-b | CLI groups hidden when empty | unit | `cd src-tauri && cargo test tray::tests::test_empty_cli_groups -x` | Wave 0 |
| PROV-01-c | Menu structure matches layout spec | unit | `cd src-tauri && cargo test tray::tests::test_menu_layout -x` | Wave 0 |
| PROV-02 | Provider switch calls _set_active_provider_in correctly | unit | `cd src-tauri && cargo test tray::tests::test_provider_switch -x` | Wave 0 |
| PROV-03 | Tray rebuild triggered after provider changes | manual-only | Manual: create provider in UI, verify tray updates | N/A -- requires running app |
| MENU-03-a | TrayTexts returns correct labels per language | unit | `cd src-tauri && cargo test tray::tests::test_tray_texts_i18n -x` | Wave 0 |
| MENU-03-b | Language change triggers menu rebuild | manual-only | Manual: switch language in settings, verify tray labels | N/A -- requires running app |

### Sampling Rate
- **Per task commit:** `cd src-tauri && cargo test --lib`
- **Per wave merge:** `cd src-tauri && cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src-tauri/src/tray.rs` needs `#[cfg(test)] mod tests` section -- currently has no tests
- [ ] Test helper functions for creating mock provider lists and settings (can reuse patterns from `commands/provider.rs` tests)

Note: Many Phase 7 behaviors (menu appearance, tray click interaction, watcher triggering menu rebuild) are inherently integration/manual tests requiring a running Tauri application. Unit tests cover the pure logic: sorting, i18n text selection, ID parsing, menu item construction decisions.

## Sources

### Primary (HIGH confidence)
- CLIManager existing source code: `tray.rs`, `commands/provider.rs`, `storage/local.rs`, `storage/icloud.rs`, `watcher/mod.rs`, `lib.rs`, `provider.rs`, `error.rs` -- direct code inspection
- cc-switch reference: `cc-switch/src-tauri/src/tray.rs` -- battle-tested tray menu with multi-CLI sections, provider switching, i18n (direct code inspection)
- [Tauri 2 System Tray documentation](https://v2.tauri.app/learn/system-tray/) -- API reference for TrayIconBuilder, menu event handling
- [TrayIcon API reference](https://docs.rs/tauri/latest/tauri/tray/struct.TrayIcon.html) -- `set_menu()`, `tray_by_id()`
- [Tauri 2 Window Menu documentation](https://v2.tauri.app/learn/window-menu/) -- CheckMenuItem behavior, menu building patterns

### Secondary (MEDIUM confidence)
- [Issue #9280: Easier updates for system tray menu](https://github.com/tauri-apps/tauri/issues/9280) -- confirms menus are immutable, must rebuild
- [Discussion #11489: Tray-only app patterns](https://github.com/tauri-apps/tauri/discussions/11489) -- CloseRequested vs ExitRequested patterns
- Previous v1.1 research: `.planning/research/ARCHITECTURE.md`, `.planning/research/PITFALLS.md` -- detailed analysis done 2026-03-12

### Tertiary (LOW confidence)
- CheckMenuItem auto-toggle behavior -- verified via web search + cc-switch pattern of always rebuilding after click. Official Tauri docs don't explicitly document this.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- zero new dependencies, all APIs already verified in Phase 6 and cc-switch reference
- Architecture: HIGH -- follows proven patterns from cc-switch, extends existing well-understood codebase
- Pitfalls: HIGH -- comprehensive pitfall analysis from v1.1 research + cc-switch patterns + verified Tauri issues

**Research date:** 2026-03-13
**Valid until:** 2026-04-13 (stable domain, Tauri 2 menu APIs are not changing rapidly)
