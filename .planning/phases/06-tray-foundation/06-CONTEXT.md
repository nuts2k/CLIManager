# Phase 6: Tray Foundation - Context

**Gathered:** 2026-03-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Application persists in macOS menu bar after window close, with basic tray controls. Delivers: tray icon, close-to-tray lifecycle with ActivationPolicy toggle, "Open Main Window" and "Quit" menu items. Provider listing and switching belong to Phase 7.

</domain>

<decisions>
## Implementation Decisions

### Cmd+Q vs Close Button
- Cmd+Q should fully quit the application (macOS standard behavior)
- Close button (red X) hides the window to tray instead of quitting
- Fallback: if technically unable to distinguish Cmd+Q from close button in Tauri 2's CloseRequested event, fall back to hiding to tray for both
- Priority: attempt to distinguish first, accept hide-to-tray as fallback

### Window Lifecycle
- App startup: always open the main window (current behavior preserved)
- Close button: hide window silently, no notification or dialog
- Hidden state: app switches to Accessory mode (no Dock icon, no Cmd+Tab entry) per TRAY-03
- Window restore: switch back to Regular mode (Dock + Cmd+Tab reappear)

### Tray Icon Interaction
- Single click on tray icon: opens the tray menu (macOS standard)
- Double click on tray icon: opens/shows the main window
- No tooltip in Phase 6 (deferred to Phase 7+)

### Menu Structure
- Layout top-to-bottom: "打开主窗口" → separator → "退出"
- Phase 7 will insert Provider list between "打开主窗口" and the separator
- Language: Chinese hardcoded in Phase 6, i18n conversion in Phase 7

### Claude's Discretion
- Tray icon PNG asset design (22x22 monochrome template)
- Exact implementation of Cmd+Q vs close button distinction attempt
- Error handling for edge cases (e.g., window already visible when "打开主窗口" clicked)
- Release build verification approach

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `lib.rs`: `tauri::Builder` setup in `run()` — tray setup goes in `.setup()` closure, `on_window_event` handler added to Builder chain
- `watcher/mod.rs`: `start_file_watcher()` pattern — Phase 7 will add `tray::update_tray_menu()` call here
- `SelfWriteTracker`: managed state pattern — tray can follow same `app.manage()` pattern if needed

### Established Patterns
- Tauri managed state via `.manage()` for shared resources
- Event emission via `app_handle.emit()` for cross-surface communication
- `_in/_to` internal function variants for testability

### Integration Points
- `lib.rs` `run()`: Add `tray-icon` + `image-png` features to Cargo.toml, add `TrayIconBuilder` in `.setup()`, add `on_window_event` for close-to-tray
- New file `tray.rs`: Menu construction, event handling, ActivationPolicy helper (~200 lines)
- `tauri.conf.json`: No tray config here (programmatic only, per research)

</code_context>

<specifics>
## Specific Ideas

- Menu should feel minimal — just two items with a separator, ready for Phase 7 to add Provider list in between
- Silent close-to-tray: no first-time dialog, no toast, no notification. User sees the tray icon and understands.

</specifics>

<deferred>
## Deferred Ideas

- Launch at login (Login Items) — new capability, future phase or v1.2
- Switching visual feedback (icon flash/animation) — Phase 7 scope (needs Provider switching first)
- Tray icon tooltip showing active Provider — Phase 7+
- Tray icon state variants (active vs no-provider) — Phase 7+

</deferred>

---

*Phase: 06-tray-foundation*
*Context gathered: 2026-03-12*
