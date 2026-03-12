# Feature Research: System Tray (v1.1)

**Domain:** Desktop system tray for CLI config-switching app
**Researched:** 2026-03-12
**Confidence:** HIGH

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist once a tray icon is present. Missing any of these = the tray feels broken or pointless.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Tray icon with app identity | Any tray-resident app shows a recognizable icon. Without it the app appears not running. | LOW | Use macOS template image (monochrome, auto dark/light). Tauri 2 `TrayIconBuilder` supports `icon_as_template(true)`. cc-switch has `statusbar_template_3x.png` as reference. |
| Provider list in tray menu | The entire value of tray = quick switching without opening the window. Must list all providers grouped by CLI. | MEDIUM | Use `CheckMenuItem` with radio-style semantics (one checked at a time per CLI section). Group by CLI (Claude header, then providers; Codex header, then providers). Flat layout, no nested submenus. cc-switch does exactly this pattern. |
| Current active provider indicated | Users need to know which provider is active at a glance. Without this, the menu is a list with no context. | LOW | `CheckMenuItem` with `is_checked = true` for the active provider per CLI section. Proven pattern in cc-switch's `append_provider_section`. |
| One-click provider switching | Click a provider name in tray menu -> it becomes active immediately. No confirmation dialog, no opening the main window. | MEDIUM | Reuse existing `set_active_provider` command logic. The tray click handler calls the same surgical-patch pipeline. Must emit events so the main window (if open) stays in sync. |
| "Open main window" menu item | Users need a way to get back to the full UI for CRUD operations. Standard pattern in every tray app. | LOW | `MenuItem::with_id("show_main", ...)`. On click: `window.show()`, `window.unminimize()`, `window.set_focus()`. On macOS also `set_activation_policy(Regular)` + `set_dock_visibility(true)`. |
| "Quit" menu item | Without explicit quit, users cannot exit the app since closing the window no longer terminates it. Critical UX requirement. | LOW | `MenuItem::with_id("quit", ...)`. On click: `app.exit(0)`. |
| Close-to-tray behavior | The core reason for having a tray. Users expect the app to keep running after closing the window. Without this, the tray icon is just decoration. | MEDIUM | Intercept `on_window_event` for `CloseRequested`. Call `event.prevent_default()`, then `window.hide()`. On macOS: switch to `ActivationPolicy::Accessory` to hide from Dock and Cmd+Tab. |
| Menu updates after provider changes | If user adds/edits/deletes providers in the main window, the tray menu must reflect changes immediately. Stale menus destroy trust. | MEDIUM | After any provider mutation (create/update/delete/switch), rebuild and re-set the tray menu via `tray.set_menu(Some(new_menu))`. Triggered by Tauri events or direct function call. |
| i18n in tray menu | App already supports zh/en. Tray menu items in mixed languages = broken feel. | LOW | Read language from local settings at menu build time. Only a few static strings: "Open main window", "Quit". Provider names are user-defined, no translation needed. Use `TrayTexts` struct pattern from cc-switch. |

### Differentiators (Competitive Advantage)

Features that make CLIManager's tray experience notably better. Not required for launch, but low-effort and high-value.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Active provider name in tray tooltip | Hovering over the tray icon shows "CLIManager - Claude: MyProvider / Codex: AnotherProvider". Saves a click to check status. | LOW | `TrayIconBuilder::tooltip(...)`. Update tooltip whenever active provider changes. Very low effort, high information density. |
| Tray icon visual state indicator | Different tray icon variants (normal vs dimmed/hollow) to show whether a provider is active or none configured. Quick visual feedback without clicking the menu. | LOW | 2-3 icon variants, swap with `tray.set_icon()`. macOS template icons auto-adapt to dark/light mode. |
| Provider protocol/model in menu label | Show "My Provider (opus-4)" not just "My Provider" next to each provider. Helps distinguish providers with similar names. | LOW | Append model info to the `CheckMenuItem` label string. No extra API needed, just string formatting from provider data. |
| Global keyboard shortcut | Hotkey (e.g., Cmd+Shift+P) to open the tray menu or trigger a provider switch without touching the mouse. | HIGH | Requires `tauri-plugin-global-shortcut`. Registration, conflict handling, and cross-platform differences add significant complexity. Defer to v1.2+. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem useful for tray but create problems in practice.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Provider CRUD in tray menu | "Let me add/edit providers without opening the window" | Tray menus are constrained to simple items (text, checkboxes, separators). Forms, text inputs, validation, and error display cannot be done in native menus. Attempting workarounds (popup windows, webview menus) produces a worse experience than the main UI. | Keep "Open main window" prominent. Tray = view + switch only. This is explicitly stated in PROJECT.md's Out of Scope. |
| Nested submenus per CLI | "Group Claude providers in a submenu, Codex in another" | Submenus add an extra click and feel sluggish on macOS. With typical provider counts (2-5 per CLI), flat list with section headers is faster and clearer. cc-switch deliberately chose flat layout as "more simple and reliable". | Use disabled `MenuItem` as section header + `CheckMenuItem` list. Separators between CLI sections. |
| Auto-switch on network/location | "Switch provider when I change WiFi or VPN" | Requires background network monitoring, complex rule engine, and handling edge cases (VPN flapping, captive portals). Enormous scope for a niche use case. | Manual one-click switching is fast enough for the actual frequency of provider changes. |
| Notification on switch success | "Show a macOS notification when provider switches" | Notification fatigue. Users switch intentionally and can see the checkmark move in the menu. Notifications add noise for a synchronous action the user just performed. | Visual feedback in the tray menu (checkmark moves) is sufficient. Log errors to console only. |
| Tray-only mode (no main window) | "I set up providers once, then only use the tray" | Removing the main window means no way to edit providers, view details, or handle errors. Complicates onboarding and error recovery. | Close-to-tray already achieves this: window stays hidden until explicitly opened. |
| Dynamic tray icon showing provider initial | "Show 'A' for Anthropic, 'O' for OpenAI in the icon" | Generating icons dynamically at runtime requires Core Graphics/Core Text on macOS. Results look inconsistent with other tray icons. Template icons must be pre-rendered PNGs. | Use tooltip for provider names. Keep icon simple and recognizable. |

## Feature Dependencies

```
[Tray Icon Setup]
    |
    +--requires--> [Close-to-Tray Behavior]
    |                  +--requires--> [macOS ActivationPolicy management]
    |
    +--requires--> [Tray Menu with Provider List]
    |                  +--requires--> [Provider data access from tray context]
    |                  |                  +--uses--> [existing iCloud storage: list_providers_in()]
    |                  |
    |                  +--requires--> [Active provider indicator (CheckMenuItem)]
    |                                     +--uses--> [existing LocalSettings.active_providers]
    |
    +--enables---> [One-Click Switching from Tray]
                       +--reuses--> [existing set_active_provider / surgical patch pipeline]
                       |
                       +--requires--> [Menu rebuild after switch]
                                          +--requires--> [Frontend sync via Tauri events]

[Provider mutations in main window] --triggers--> [Tray menu rebuild]
[iCloud file watcher detects change] --triggers--> [Tray menu rebuild]
```

### Dependency Notes

- **Tray Icon Setup requires Close-to-Tray:** Without close-to-tray, the tray icon disappears when the user closes the window, defeating the purpose. These must ship together.
- **One-Click Switching reuses set_active_provider:** The existing `_set_active_provider_in` function in `commands/provider.rs` handles all surgical patch logic. The tray handler calls the same pipeline, not a duplicate. This keeps switching behavior consistent whether done from the UI or the tray.
- **Menu rebuild triggered by multiple sources:** Provider CRUD in the main window, provider switch from tray itself, and iCloud watcher detecting external changes all need to trigger a tray menu rebuild. Build a centralized `rebuild_tray_menu(app_handle)` function.
- **Frontend sync via events:** When switching from the tray, emit a `provider-switched` event so the main window (if open) updates its UI. cc-switch does this with `app.emit("provider-switched", ...)`. The main window already handles provider list refresh via the watcher; the event ensures immediate UI update without waiting for FSEvents debounce.

## MVP Definition

### Launch With (v1.1)

All table stakes must ship together. This is the minimum viable tray experience.

- [ ] Tray icon with template image (macOS) -- app identity in menu bar
- [ ] Tray menu: "Open main window" item -- escape hatch to full UI
- [ ] Tray menu: Provider list grouped by CLI with section headers -- core value
- [ ] Tray menu: Active provider indicated with checkmark -- status at a glance
- [ ] Tray menu: One-click switching via CheckMenuItem click -- primary use case
- [ ] Tray menu: "Quit" item -- explicit app termination
- [ ] Close-to-tray: window close hides instead of exits -- reason the tray exists
- [ ] macOS ActivationPolicy: Accessory when hidden, Regular when shown -- proper Dock/Cmd+Tab behavior
- [ ] Menu rebuild on provider mutations from main window -- keeps tray in sync
- [ ] Menu rebuild on iCloud watcher events -- keeps tray in sync across devices
- [ ] i18n support in tray menu strings -- consistent with existing zh/en support

### Add After Validation (v1.1.x)

Features to add once core tray is working and tested.

- [ ] Tooltip showing active provider names per CLI -- low effort, high info density
- [ ] Tray icon state variants (active vs. no-provider) -- visual feedback improvement
- [ ] Provider model name in menu item label -- disambiguation for similar provider names

### Future Consideration (v2+)

Features to defer until tray UX is proven.

- [ ] Global keyboard shortcut for tray menu -- requires plugin, conflict handling
- [ ] Additional CLI sections in tray (Gemini, OpenCode) -- when those adapters are built
- [ ] Visible apps filtering (hide certain CLI sections) -- only relevant with 3+ CLIs

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Tray icon + basic menu structure | HIGH | LOW | P1 |
| Provider list with active indicator | HIGH | MEDIUM | P1 |
| One-click switching from tray | HIGH | MEDIUM | P1 |
| Close-to-tray behavior | HIGH | MEDIUM | P1 |
| macOS ActivationPolicy management | HIGH | LOW | P1 |
| Menu rebuild on mutations/watcher | HIGH | MEDIUM | P1 |
| i18n in tray strings | MEDIUM | LOW | P1 |
| Tooltip with active provider names | MEDIUM | LOW | P2 |
| Tray icon state variants | LOW | LOW | P2 |
| Provider model in menu label | LOW | LOW | P3 |
| Global keyboard shortcut | MEDIUM | HIGH | P3 |

**Priority key:**
- P1: Must have for v1.1 launch (all table stakes)
- P2: Should have, add in v1.1 if time permits or v1.1.x patch
- P3: Nice to have, future consideration

## Existing Code Dependencies

The tray feature depends heavily on existing CLIManager infrastructure. Here is what exists and what needs to be built.

### Reusable As-Is

| Existing Code | Location | How Tray Uses It |
|--------------|----------|------------------|
| `set_active_provider` / `_set_active_provider_in` | `commands/provider.rs` | Tray click handler calls this to perform surgical patch switching |
| `list_providers_in` / provider storage | `storage/icloud.rs` | Tray menu builder reads provider list to populate menu items |
| `read_local_settings` / `LocalSettings` | `storage/local.rs` | Tray reads `active_providers` map for checkmark state and `language` for i18n |
| `SelfWriteTracker` | `watcher/self_write.rs` | Prevents tray-triggered writes from re-triggering the file watcher |
| iCloud file watcher | `watcher/mod.rs` | Already watches for changes; needs hook to also trigger tray menu rebuild |
| CLI adapters | `adapter/claude.rs`, `adapter/codex.rs` | Used by the switching pipeline, no changes needed for tray |

### Needs Modification

| Existing Code | What Changes | Why |
|--------------|-------------|-----|
| `lib.rs` (the `run()` function) | Add `TrayIconBuilder` setup in `.setup()`, add `on_window_event` for close-to-tray, register tray menu event handler | This is where Tauri app lifecycle is configured |
| `watcher/mod.rs` | After processing iCloud changes and emitting frontend events, also call `rebuild_tray_menu()` | Tray menu must reflect provider changes from other devices |
| `Cargo.toml` dependencies | Add `"tray-icon"` to Tauri features | Enables `tauri::tray` module |
| `tauri.conf.json` | Potentially update window close behavior or permissions | May need `"withGlobalTauri"` or tray-related config |

### Needs to Be Built (New Code)

| New Code | Purpose | Estimated Size |
|----------|---------|---------------|
| `src-tauri/src/tray.rs` | Tray module: `create_tray_menu()`, `handle_tray_menu_event()`, `rebuild_tray_menu()`, `TrayTexts` i18n struct, `apply_tray_policy()` | ~200-300 lines (cc-switch's is ~450 but includes failover/proxy logic we skip) |
| Tray icon assets | Template PNG for macOS status bar (monochrome, @1x/@2x/@3x) in `src-tauri/icons/tray/` | 3 PNG files |
| Tauri command: `update_tray_menu` | Exposed command so frontend can trigger tray rebuild after provider CRUD | ~10 lines (thin wrapper) |

## Competitor Feature Analysis

| Feature | cc-switch (reference) | CLIManager v1.1 (our approach) |
|---------|----------------------|-------------------------------|
| Tray menu structure | Flat list with CLI section headers, `CheckMenuItem` per provider | Same pattern -- proven to work, simple and reliable |
| Provider switching from tray | Calls `switch_provider`, rebuilds menu, emits events to frontend | Same pattern -- reuse `set_active_provider`, rebuild menu, emit events |
| Close-to-tray | `CloseRequested` -> `prevent_default()` + `window.hide()` + `ActivationPolicy::Accessory` | Same pattern -- this is the standard macOS approach |
| Tray i18n | `TrayTexts` struct with hardcoded zh/en/ja strings | Same pattern, zh/en only (matching existing i18n scope) |
| Auto/Failover in tray | Has "Auto (Failover)" mode toggle per CLI section | Not applicable -- CLIManager has no proxy/failover feature. Simplifies the menu. |
| Visible apps filtering | Settings to hide certain CLI sections from tray | Defer -- only 2 CLIs (Claude, Codex), not enough to warrant filtering |
| Tray icon | macOS template image with `icon_as_template(true)` | Same approach -- must create our own icon asset |
| Menu rebuild trigger | Frontend calls `update_tray_menu` command; also rebuilds internally after tray-initiated switch | Same approach -- centralized rebuild function called from multiple trigger points |

## Sources

- cc-switch tray implementation: `cc-switch/src-tauri/src/tray.rs` (447 lines, complete reference with menu creation, event handling, i18n, ActivationPolicy)
- cc-switch tray setup: `cc-switch/src-tauri/src/lib.rs` (TrayIconBuilder config, window close interception, dock visibility)
- Tauri 2 tray API: `tauri::tray::TrayIconBuilder`, `tauri::menu::{Menu, MenuBuilder, MenuItem, CheckMenuItem}` (used directly in cc-switch, proven with Tauri 2.10)
- CLIManager provider pipeline: `src-tauri/src/commands/provider.rs` (`set_active_provider`, `_set_active_provider_in`, surgical patch flow)
- CLIManager app setup: `src-tauri/src/lib.rs` (current `run()` function, watcher setup, command registration)
- PROJECT.md: Out of Scope section explicitly states "tray only does view + switch, CRUD stays in main window"

---
*Feature research for: System Tray (v1.1 milestone)*
*Researched: 2026-03-12*
