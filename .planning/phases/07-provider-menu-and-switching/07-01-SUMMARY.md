---
phase: 07-provider-menu-and-switching
plan: 01
subsystem: tray
tags: [tauri, tray-icon, checkmenuitem, i18n, provider-switching]

# Dependency graph
requires:
  - phase: 06-tray-foundation
    provides: "Static tray menu with show_main_window, apply_tray_policy, TrayIconBuilder setup"
provides:
  - "Dynamic tray menu with provider listing grouped by CLI"
  - "CheckMenuItem with native checkmarks for active providers"
  - "One-click provider switching from tray via spawn_blocking"
  - "Tray menu i18n (zh/en) via TrayTexts struct"
  - "update_tray_menu for live menu rebuilds"
  - "refresh_tray_menu Tauri command for frontend-triggered rebuilds"
affects: [07-02-watcher-refresh]

# Tech tracking
tech-stack:
  added: []
  patterns: ["TrayTexts i18n struct with from_language", "parse_provider_event with strip_prefix", "Menu-as-Snapshot full rebuild via set_menu", "spawn_blocking for tray event handler I/O"]

key-files:
  created: []
  modified:
    - "src-tauri/src/tray.rs"
    - "src-tauri/src/commands/provider.rs"
    - "src-tauri/src/lib.rs"

key-decisions:
  - "Emit providers-changed (not provider-switched) from tray handler to reuse existing frontend listener"
  - "Make _set_active_provider_in pub(crate) for cross-module access from tray.rs"
  - "Added use tauri::Emitter import for app_handle.emit() in tray.rs"

patterns-established:
  - "TrayTexts::from_language(lang) for lightweight Rust-side menu i18n"
  - "parse_provider_event with strip_prefix for safe menu ID extraction"
  - "update_tray_menu(app) as single rebuild entry point called from multiple surfaces"
  - "handle_provider_click with spawn_blocking + menu rebuild on both success and failure"

requirements-completed: [PROV-01, PROV-02, MENU-03]

# Metrics
duration: 5min
completed: 2026-03-13
---

# Phase 7 Plan 1: Provider Menu and Switching Summary

**Dynamic tray menu with provider listing by CLI group, CheckMenuItem checkmarks, one-click switching via spawn_blocking, and zh/en i18n**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-13T05:19:33Z
- **Completed:** 2026-03-13T05:24:54Z
- **Tasks:** 2 (1 TDD auto + 1 auto)
- **Files modified:** 3

## Accomplishments
- Dynamic tray menu reads providers from iCloud storage and groups by CLI (Claude Code first, Codex second)
- Active provider per CLI shows native macOS checkmark via CheckMenuItem; active sorts first, remaining by name
- Clicking a provider in tray switches immediately via spawn_blocking without opening the main window
- Menu rebuilds on both switch success and failure to maintain correct visual state (pitfall 2)
- TrayTexts i18n returns correct labels for zh/en, brand names unchanged across languages
- refresh_tray_menu Tauri command registered for frontend-triggered rebuilds after CRUD and language changes
- Empty CLI groups hidden; when all CLIs have zero providers, menu shows only "Open Main Window" and "Quit"
- All 132 tests pass (124 existing + 8 new tray tests) with zero regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend tray.rs with dynamic menu, i18n, and provider click handling** - `c97c83e` (feat, TDD)
2. **Task 2: Add refresh_tray_menu command and register in invoke_handler** - `4fd92d2` (feat)

## Files Created/Modified
- `src-tauri/src/tray.rs` - Dynamic menu construction, TrayTexts i18n, parse_provider_event, update_tray_menu, handle_provider_click with spawn_blocking, 8 unit tests
- `src-tauri/src/commands/provider.rs` - Made _set_active_provider_in pub(crate), added refresh_tray_menu Tauri command
- `src-tauri/src/lib.rs` - Registered refresh_tray_menu in invoke_handler

## Decisions Made
- Emit "providers-changed" event from tray switch handler (not a new event name) to reuse existing frontend listener infrastructure per RESEARCH.md recommendation
- Used Emitter trait import for app_handle.emit() in tray.rs (Tauri 2 requires explicit trait import)
- Made _set_active_provider_in pub(crate) during Task 1 as a blocking dependency (planned for Task 2)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing `use tauri::Emitter` import**
- **Found during:** Task 1 (Dynamic menu with event emission)
- **Issue:** `app_handle.emit()` requires the `tauri::Emitter` trait in scope; cargo build failed with E0599
- **Fix:** Added `use tauri::Emitter;` to the import list in tray.rs
- **Files modified:** src-tauri/src/tray.rs
- **Verification:** cargo build succeeds, cargo test passes (132 tests)
- **Committed in:** c97c83e (Task 1 commit)

**2. [Rule 3 - Blocking] Made _set_active_provider_in pub(crate) early**
- **Found during:** Task 1 (tray.rs calls _set_active_provider_in)
- **Issue:** tray.rs cannot compile without pub(crate) visibility on _set_active_provider_in (E0603)
- **Fix:** Changed `fn _set_active_provider_in` to `pub(crate) fn _set_active_provider_in` in Task 1 instead of Task 2
- **Files modified:** src-tauri/src/commands/provider.rs
- **Verification:** cargo build succeeds
- **Committed in:** c97c83e (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both auto-fixes necessary for compilation. No scope creep. Task 2's pub(crate) change was pulled forward to Task 1 as a blocking dependency.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Dynamic tray menu complete, ready for Plan 2: Watcher integration and auto-refresh
- Plan 2 will add tray rebuild call in watcher process_events after providers-changed emit
- `update_tray_menu(app)` is the single entry point for all tray rebuild triggers
- Frontend can already call `invoke("refresh_tray_menu")` after CRUD operations

---
*Phase: 07-provider-menu-and-switching*
*Completed: 2026-03-13*

## Self-Check: PASSED

- FOUND: src-tauri/src/tray.rs
- FOUND: src-tauri/src/commands/provider.rs
- FOUND: src-tauri/src/lib.rs
- FOUND: 07-01-SUMMARY.md
- FOUND: commit c97c83e
- FOUND: commit 4fd92d2
