---
phase: 06-tray-foundation
plan: 01
subsystem: tray
tags: [tauri, tray-icon, macos, activation-policy, close-to-tray]

# Dependency graph
requires:
  - phase: 05-onboarding
    provides: "Complete v1.0 app with provider management, file watching, UI"
provides:
  - "System tray icon in macOS menu bar with dark/light mode adaptation"
  - "Close-to-tray lifecycle with ActivationPolicy toggling"
  - "Tray menu with show-window and quit items"
  - "Cmd+Q vs close button distinction via .build()+.run() pattern"
affects: [07-provider-menu-switching]

# Tech tracking
tech-stack:
  added: ["tray-icon feature flag", "image-png feature flag"]
  patterns: ["TrayIconBuilder programmatic setup", "on_window_event close-to-tray", ".build()+.run() for RunEvent access", "ActivationPolicy toggle helper"]

key-files:
  created:
    - "src-tauri/src/tray.rs"
    - "src-tauri/icons/tray/tray-icon-template.png"
  modified:
    - "src-tauri/Cargo.toml"
    - "src-tauri/src/lib.rs"

key-decisions:
  - "Programmatic TrayIconBuilder only (no tauri.conf.json trayIcon) to avoid duplicate icon bugs"
  - "DoubleClick best-effort: conflicts with show_menu_on_left_click, menu item provides same function"
  - "Added use tauri::Manager import in lib.rs for app_handle() access on Window"

patterns-established:
  - "tray::apply_tray_policy(app, bool) for Dock/Cmd+Tab visibility toggling"
  - "on_window_event CloseRequested -> prevent_close + hide + Accessory mode"
  - "include_bytes! for embedding tray icon PNG at compile time"

requirements-completed: [TRAY-01, TRAY-02, TRAY-03, MENU-01, MENU-02]

# Metrics
duration: 8min
completed: 2026-03-13
---

# Phase 6 Plan 1: Tray Foundation Summary

**macOS system tray with template icon, close-to-tray lifecycle via ActivationPolicy toggling, and minimal menu ("打开主窗口" / "退出")**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-03-13T00:05:35Z
- **Completed:** 2026-03-13T00:13:15Z
- **Tasks:** 3 (2 auto + 1 human-verify checkpoint)
- **Files modified:** 4

## Accomplishments
- Tray icon visible in macOS menu bar, auto-adapts to dark/light mode via icon_as_template(true)
- Close button hides window to tray silently; Cmd+Q fully quits (via .build()+.run() refactor)
- Hidden state switches to Accessory mode (no Dock, no Cmd+Tab); restore switches back to Regular
- Tray menu with "打开主窗口" (show/focus window) and "退出" (full exit) working correctly
- All 124 existing tests pass with zero regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Create tray module, icon asset, and enable feature flags** - `f600f5a` (feat)
2. **Task 2: Wire tray into lib.rs with close-to-tray lifecycle** - `c3e7c9f` (feat)
3. **Task 3: Verify tray foundation end-to-end** - human-verify checkpoint (approved)

## Files Created/Modified
- `src-tauri/src/tray.rs` - Tray menu construction, event handling, show_main_window, apply_tray_policy
- `src-tauri/icons/tray/tray-icon-template.png` - 44x44 monochrome template icon for macOS menu bar
- `src-tauri/Cargo.toml` - Added tray-icon and image-png feature flags on tauri crate
- `src-tauri/src/lib.rs` - mod tray, on_window_event close-to-tray, tray setup in .setup(), .build()+.run() refactor

## Decisions Made
- Programmatic TrayIconBuilder only (no tauri.conf.json trayIcon) -- avoids duplicate icon bugs per GitHub Issue #10912
- DoubleClick on tray icon does not work due to show_menu_on_left_click(true) consuming the first click -- acceptable per plan, "打开主窗口" menu item provides identical functionality
- Added `use tauri::Manager` import to lib.rs -- required for `app_handle()` method on Window reference in on_window_event handler

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing `use tauri::Manager` import**
- **Found during:** Task 2 (Wire tray into lib.rs)
- **Issue:** `window.app_handle()` in on_window_event closure requires the Manager trait in scope; cargo build failed with E0599
- **Fix:** Added `use tauri::Manager;` at the top of lib.rs
- **Files modified:** src-tauri/src/lib.rs
- **Verification:** cargo build succeeds, cargo test passes (124 tests)
- **Committed in:** c3e7c9f (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Trivial import addition required for compilation. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviation above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Tray foundation complete, ready for Phase 7: Provider Menu and Switching
- Phase 7 will insert Provider list between "打开主窗口" and separator in the tray menu
- `tray::create_tray_menu` is the extension point for Phase 7
- Release build tray behavior still needs verification (deferred concern from STATE.md)

---
*Phase: 06-tray-foundation*
*Completed: 2026-03-13*

## Self-Check: PASSED

- FOUND: src-tauri/src/tray.rs
- FOUND: src-tauri/icons/tray/tray-icon-template.png
- FOUND: src-tauri/src/lib.rs
- FOUND: 06-01-SUMMARY.md
- FOUND: commit f600f5a
- FOUND: commit c3e7c9f
