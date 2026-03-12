# Project Research Summary

**Project:** CLIManager v1.1 System Tray
**Domain:** System tray integration for existing Tauri 2 desktop app (macOS)
**Researched:** 2026-03-12
**Confidence:** HIGH

## Executive Summary

CLIManager v1.1 adds a system tray to the existing Tauri 2 desktop app, enabling users to view and switch CLI providers directly from the macOS menu bar without opening the main window. This is a well-understood pattern in the Tauri ecosystem: the entire tray feature is built into the `tauri` crate behind two feature flags (`tray-icon`, `image-png`), requiring zero new dependencies. A proven reference implementation exists in cc-switch, and all four research tracks converge on the same architecture: a single new Rust module (`tray.rs`) that reads from existing storage and reuses existing switching logic, with no new data paths or state stores.

The recommended approach is a two-phase build: first establish the tray foundation (icon, close-to-tray lifecycle, basic menu with Quit), then layer on the provider menu with one-click switching and bidirectional sync. This order is dictated by hard dependencies -- close-to-tray must work before provider features matter, and the tray icon must appear before anything else can be validated. The architecture is deliberately conservative: the tray module is a thin UI surface over existing infrastructure, calling `_set_active_provider_in()` for switching and `list_providers()`/`read_local_settings()` for menu construction.

The primary risks are (1) stale tray menus from missed rebuild triggers across three change sources (UI, tray, iCloud), (2) incorrect window close-vs-hide lifecycle breaking the "close to tray" contract, and (3) tray icon silently not appearing due to dual configuration or missing template mode. All three have well-documented prevention patterns from Tauri GitHub issues and the cc-switch reference. The mitigation strategy is centralized: one `rebuild_tray_menu()` function called from all mutation paths, programmatic-only tray setup via `TrayIconBuilder`, and explicit `ActivationPolicy` toggling on every show/hide transition.

## Key Findings

### Recommended Stack

No new dependencies. The entire tray feature ships via two Cargo feature flags on the existing `tauri` crate:

- **`tray-icon` feature on `tauri`**: Enables `TrayIconBuilder`, `TrayIconEvent`, and tray menu APIs -- required for any tray functionality in Tauri 2
- **`image-png` feature on `tauri`**: Enables `Image::from_bytes()` for PNG parsing -- required to load custom tray icons via `include_bytes!`
- **Tray icon assets**: 22x22 and 44x44 monochrome template PNGs for macOS menu bar -- must call `icon_as_template(true)` for dark/light mode adaptation

No npm packages needed. All tray logic belongs in Rust because the tray must function when the webview is hidden.

See [STACK.md](STACK.md) for full API reference and anti-patterns.

### Expected Features

**Must have (table stakes -- all ship together in v1.1):**
- Tray icon with macOS template image (app identity in menu bar)
- Provider list grouped by CLI with section headers and CheckMenuItem active indicator
- One-click provider switching from tray menu (reuses existing `_set_active_provider_in`)
- Close-to-tray: window close hides instead of exits, with ActivationPolicy toggle
- "Open Main Window" and "Quit" menu items
- Menu rebuilds on provider mutations from UI, tray, and iCloud sync
- i18n support in tray menu strings (zh/en)

**Should have (add in v1.1.x if not in initial release):**
- Tooltip showing active provider names per CLI
- Tray icon state variants (active vs no-provider)
- Provider model name in menu item label

**Defer (v2+):**
- Global keyboard shortcut (requires plugin, conflict handling)
- Provider CRUD from tray (anti-feature -- tray menus cannot do forms)
- Nested submenus per CLI (unnecessary with only 2 CLI types)

See [FEATURES.md](FEATURES.md) for full prioritization matrix and anti-features analysis.

### Architecture Approach

The tray module is a new UI surface that sits alongside the existing Rust backend, consuming the same storage and adapter layers. It introduces no new data paths, no new state stores, and no new persistence. One new file (`tray.rs`, ~200-300 lines) with three public functions (`create_tray_menu`, `update_tray_menu`, `handle_tray_menu_event`), plus modifications to `lib.rs` (TrayIconBuilder setup, on_window_event handler) and one line added to `watcher/mod.rs`.

**Major components:**
1. **`tray.rs`** -- Menu construction from storage state, menu event dispatch, i18n labels, ActivationPolicy helper
2. **`lib.rs` modifications** -- TrayIconBuilder setup in `.setup()`, `on_window_event` for close-to-tray, `refresh_tray_menu` command registration
3. **`watcher/mod.rs` modification** -- Single line: call `tray::update_tray_menu()` after processing iCloud sync events

**Key patterns:**
- Menu-as-Snapshot: Rebuild entire menu from disk on every change (Tauri 2 menus are immutable after build)
- Shared Internal Functions: Tray calls `_set_active_provider_in()` directly -- no logic duplication
- Event-Driven Cross-Surface Sync: Tray emits `provider-switched` to frontend; frontend calls `refresh_tray_menu` after CRUD

See [ARCHITECTURE.md](ARCHITECTURE.md) for data flow diagrams, implementation skeleton, and build order.

### Critical Pitfalls

1. **Stale tray menu after provider changes** -- Three change sources (UI, tray, iCloud) must all trigger menu rebuild. Build a centralized `rebuild_tray_menu()` function and call it from every mutation path. This is the most likely bug.
2. **Window close vs hide lifecycle** -- Must intercept `CloseRequested`, call `api.prevent_close()` + `window.hide()`, and toggle `ActivationPolicy`. Getting this wrong means the app either dies on close or leaves a ghost dock icon.
3. **Tray icon not appearing** -- Configure tray ONLY via `TrayIconBuilder` in Rust (never in `tauri.conf.json`). Use `include_bytes!` for the icon and `icon_as_template(true)` for dark mode. Silent failure is common.
4. **FSEvents watcher race condition** -- Tray rebuild must happen AFTER `sync_changed_active_providers` completes, not before or in parallel. Place within the existing debounced handler.
5. **Tray switch bypassing command pipeline** -- Call `_set_active_provider_in()` (same function the UI command uses), emit events to frontend, and record writes in `SelfWriteTracker`.

See [PITFALLS.md](PITFALLS.md) for full prevention strategies, warning signs, and "looks done but isn't" checklist.

## Implications for Roadmap

Based on research, the feature decomposes into two phases with a clear dependency boundary.

### Phase 1: Tray Foundation

**Rationale:** The tray icon and close-to-tray lifecycle are hard prerequisites for everything else. If the icon does not appear or the app dies on window close, no menu features matter. This phase also contains the highest-risk pitfalls (icon not appearing, close-vs-hide lifecycle) that should be validated early.

**Delivers:** Tray icon visible in macOS menu bar. Closing the window hides to tray instead of quitting. "Open Main Window" restores the window. "Quit" exits the app. Dock icon toggles correctly with ActivationPolicy.

**Addresses features:** Tray icon with app identity, close-to-tray behavior, macOS ActivationPolicy management, "Open Main Window" item, "Quit" item.

**Avoids pitfalls:** Tray icon not appearing (Pitfall 3), window close vs hide (Pitfall 2), Cmd+Q vs close button differentiation.

**Stack changes:** Add `tray-icon` and `image-png` features to Cargo.toml. Create tray icon PNG assets. Create minimal `tray.rs` with menu skeleton. Add `on_window_event` to `lib.rs`.

### Phase 2: Provider Menu and Switching

**Rationale:** With the tray foundation proven, this phase adds the core value: viewing providers and switching with one click. It depends on Phase 1 for the tray to exist and the app to survive window close. This phase contains the most integration complexity (three sync sources, shared switching logic, watcher hookup).

**Delivers:** Full provider list in tray menu grouped by CLI. CheckMenuItem with active indicator. One-click switching that reuses existing pipeline. Bidirectional sync between tray, frontend, and iCloud watcher. i18n in tray labels.

**Addresses features:** Provider list with section headers, active provider indicator, one-click switching, menu rebuild on mutations/watcher, i18n in tray strings.

**Avoids pitfalls:** Stale tray menu (Pitfall 1), FSEvents race condition (Pitfall 4), tray switch bypassing pipeline (Pitfall 5).

**Key integration:** Make `_set_active_provider_in` pub(crate). Add `tray::update_tray_menu()` call to watcher. Add `refresh_tray_menu` Tauri command. Emit `provider-switched` events from tray handler.

### Phase Ordering Rationale

- **Phase 1 before Phase 2** is a hard dependency: the tray must exist and close-to-tray must work before provider menu features are meaningful. Architecture research confirms this with a 6-step build order where steps 1-2 (icon + lifecycle) precede steps 3-6 (providers + sync).
- **Two phases, not three or four:** The feature is small enough that further splitting would create artificial boundaries. All table-stakes features ship within these two phases. The total new code is ~200-300 lines of Rust plus icon assets.
- **No separate "polish" phase:** i18n and edge cases (Cmd+Q differentiation, release build testing) belong in Phase 2 rather than a separate phase, because they are tightly coupled to the provider menu implementation.

### Research Flags

Phases with standard patterns (skip `/gsd:research-phase`):
- **Phase 1 (Tray Foundation):** Well-documented Tauri 2 APIs, proven cc-switch reference, exact code patterns available in ARCHITECTURE.md skeleton. No additional research needed.
- **Phase 2 (Provider Menu):** Architecture research provides a complete implementation skeleton including `create_tray_menu`, `handle_tray_menu_event`, and all integration points. Pitfalls research covers every known gotcha. No additional research needed.

Both phases have HIGH confidence research with working reference code. The implementation can proceed directly from the research documents.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Zero new dependencies. Feature flags verified against cc-switch Cargo.toml and official Tauri 2 docs. Single-line Cargo.toml change. |
| Features | HIGH | Table stakes clearly defined with cc-switch as direct comparison. Anti-features well-justified. Feature dependency graph is complete. |
| Architecture | HIGH | Full implementation skeleton provided. Data flows mapped for all 6 scenarios. Integration points with existing code identified at function-level specificity. |
| Pitfalls | HIGH | 5 critical pitfalls sourced from Tauri GitHub issues (with issue numbers), official docs, and cc-switch battle-testing. Prevention patterns are concrete, not theoretical. |

**Overall confidence:** HIGH

### Gaps to Address

- **Tray icon asset creation:** Research specifies requirements (22x22 monochrome template PNG) but the actual icon files need to be designed and created. This is a design task, not a research gap.
- **Cmd+Q vs close button differentiation:** Pitfalls research flags this as important UX but the exact Tauri 2 API for distinguishing Cmd+Q from the red close button needs validation during Phase 1 implementation. The `CloseRequested` event may not distinguish the source.
- **`_set_active_provider_in` signature compatibility:** Architecture assumes this function can be called from tray context with `None` for the adapter parameter (letting it resolve internally). Verify this works or adjust the call site during Phase 2.
- **Release build tray behavior:** Multiple Tauri GitHub issues report tray differences between dev and release builds. Phase 1 must include a release build verification step.

## Sources

### Primary (HIGH confidence)
- [Tauri 2 System Tray Guide](https://v2.tauri.app/learn/system-tray/) -- official documentation
- [Tauri 2 TrayIconBuilder API](https://docs.rs/tauri/2.0.0/tauri/tray/struct.TrayIconBuilder.html) -- API reference
- cc-switch `src-tauri/src/tray.rs` -- 447-line working tray implementation
- cc-switch `src-tauri/src/lib.rs` -- TrayIconBuilder setup, on_window_event, ActivationPolicy
- CLIManager codebase -- direct inspection of `lib.rs`, `commands/provider.rs`, `watcher/mod.rs`

### Secondary (HIGH confidence)
- [Issue #9280](https://github.com/tauri-apps/tauri/issues/9280) -- no in-place menu update API
- [Issue #10912](https://github.com/tauri-apps/tauri/issues/10912) -- duplicate tray icon from dual config
- [Issue #11931](https://github.com/tauri-apps/tauri/issues/11931) -- config vs programmatic conflict
- [Discussion #11489](https://github.com/tauri-apps/tauri/discussions/11489) -- ExitRequested infinite loop
- [Discussion #6038](https://github.com/tauri-apps/tauri/discussions/6038) -- ActivationPolicy toggle pattern
- [Discussion #2684](https://github.com/tauri-apps/tauri/discussions/2684) -- close-to-tray community pattern

### Tertiary (MEDIUM confidence)
- [Discussion #9365](https://github.com/orgs/tauri-apps/discussions/9365) -- tray icon on external display only
- [Issue #12060](https://github.com/tauri-apps/tauri/issues/12060) -- tray instability on macOS Sequoia
- [Issue #13770](https://github.com/tauri-apps/tauri/issues/13770) -- icon regression in Tauri 2.6.x

---
*Research completed: 2026-03-12*
*Ready for roadmap: yes*
