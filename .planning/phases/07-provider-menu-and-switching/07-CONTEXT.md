# Phase 7: Provider Menu and Switching - Context

**Gathered:** 2026-03-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can view and switch Providers directly from the tray menu without opening the main window. Delivers: dynamic Provider menu grouped by CLI with active checkmarks, one-click switching via tray, auto-refresh on Provider changes/iCloud sync, and tray menu i18n. Provider CRUD stays in the main window.

</domain>

<decisions>
## Implementation Decisions

### Menu Layout and Grouping
- Use disabled MenuItem as section headers for CLI groups (e.g., disabled "Claude Code", disabled "Codex")
- Claude Code group appears first, Codex second (Claude Code is the primary use case)
- Within each group, active Provider sorts first, remaining sorted by name
- Standard layout: "Open Main Window" -> separator -> [Claude Code] -> Providers... -> [Codex] -> Providers... -> separator -> "Quit"
- Hide CLI groups that have no Providers (don't show empty groups)
- When all CLIs have zero Providers, hide the entire Provider section (menu shows only "Open Main Window" and "Quit")

### Menu Item Display
- Show Provider name only (the `name` field), no model/protocol suffix
- Use Tauri CheckMenuItem for active Provider marking — native system checkmark
- Clicking a CheckMenuItem triggers Provider switch (patch CLI config) and updates checkmark position

### Tray Menu i18n
- Sync immediately when user switches language in frontend settings
- Rust maintains its own simplified translation map (only the few menu strings: "Open Main Window", "Quit", CLI group titles)
- Frontend notifies Rust via Tauri command when language changes, Rust rebuilds menu
- Language preference persisted to local.json (device-local layer) so Rust can read it on startup
- This requires adding a `language` field to LocalSettings and a new Tauri command for language change notification

### Claude's Discretion
- Exact Rust translation map structure (HashMap, match, or enum)
- Menu rebuild implementation details (replace menu vs rebuild TrayIcon)
- Error handling for switch failures in tray context
- How to read current active providers from local.json for menu construction

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tray.rs`: `create_tray_menu()` — needs to be extended to accept Provider list and language; `handle_tray_menu_event()` needs new cases for Provider switching
- `commands/provider.rs`: `set_active_provider()` — contains full switch logic (patch CLI config + update local.json), can be called from tray menu handler
- `commands/provider.rs`: `list_providers()` / `get_local_settings()` — needed to build dynamic menu
- `storage/local.rs`: `LocalSettings` — needs `language` field added for Rust-side language access
- `watcher/mod.rs`: Already emits "providers-changed" event — can trigger tray menu rebuild

### Established Patterns
- Tauri managed state via `.manage()` for shared resources
- `SelfWriteTracker` pattern for managed state
- Event-driven refresh via `app_handle.emit()`
- `_in/_to` internal function variants for testability

### Integration Points
- `lib.rs` setup: Tray menu construction moves from static to dynamic (read providers + language on startup)
- `watcher/mod.rs`: Add tray menu rebuild call in `process_events()` after "providers-changed" emit
- `tray.rs`: New `rebuild_tray_menu()` function called from watcher and from language-change command
- New Tauri command: `set_language` or `update_tray_language` — frontend calls this on language switch
- `local.json`: Add `language` field (default "zh")

</code_context>

<specifics>
## Specific Ideas

- Menu should feel like a natural extension of Phase 6's minimal tray — same structure, just with Provider items inserted
- Switching should be instant — click Provider, checkmark moves, done. No confirmation dialog, no toast.
- The disabled section headers should look like macOS system preferences grouping style

</specifics>

<deferred>
## Deferred Ideas

- Launch at login (Login Items) — future phase or v1.2
- Tray icon tooltip showing active Provider name — TRAY-04 (v2)
- Tray icon state variants (active vs no-provider) — TRAY-05 (v2)
- Provider menu item showing model name for disambiguation — PROV-04 (v2)
- Global hotkey for Provider switching — KEY-01 (v2)

</deferred>

---

*Phase: 07-provider-menu-and-switching*
*Context gathered: 2026-03-13*
