---
phase: 04-icloud-sync-and-file-watching
plan: 01
subsystem: sync
tags: [notify, fsevent, file-watcher, debounce, tauri-events]

# Dependency graph
requires:
  - phase: 01-storage-foundation
    provides: iCloud providers directory, atomic_write, provider CRUD
  - phase: 03-provider-management-ui
    provides: Provider commands (create/update/delete/sync_active_providers)
provides:
  - FSEvents-based file watcher monitoring iCloud providers directory
  - SelfWriteTracker preventing infinite loops from app's own writes
  - ProvidersChangedPayload Tauri event with repatched status
  - Automatic CLI config re-patching on sync events
affects: [04-02, frontend-sync-listener]

# Tech tracking
tech-stack:
  added: [notify 8 (macos_fsevent), notify-debouncer-mini 0.7]
  patterns: [self-write tracking with expiry window, pure filter function for testability]

key-files:
  created:
    - src-tauri/src/watcher/mod.rs
    - src-tauri/src/watcher/self_write.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs
    - src-tauri/src/commands/provider.rs

key-decisions:
  - "Used notify-debouncer-mini with 500ms debounce for FSEvents batching"
  - "SelfWriteTracker uses 1-second expiry window with automatic cleanup on check"
  - "Extracted filter_and_dedup_events as pure function with closure parameter for testability"
  - "std::mem::forget(debouncer) to keep watcher alive for app lifetime"

patterns-established:
  - "Self-write tracking: record_write after file ops, check in watcher callback"
  - "Pure filter functions with injected dependencies for unit testing watcher logic"

requirements-completed: [SYNC-03, SYNC-05]

# Metrics
duration: 5min
completed: 2026-03-11
---

# Phase 4 Plan 1: File Watcher Backend Summary

**FSEvents file watcher with self-write tracking, 500ms debounce, .json filtering, and automatic CLI re-patching on sync**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-11T11:57:09Z
- **Completed:** 2026-03-11T12:02:03Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- File watcher monitors iCloud providers directory using FSEvents via notify crate
- SelfWriteTracker prevents infinite feedback loops with 1-second expiry window
- Events debounced at 500ms, filtered to .json only, deduplicated by file stem
- Automatic CLI config re-patching via sync_active_providers on external changes
- ProvidersChangedPayload includes repatched boolean for frontend toast display
- 11 new unit tests, all 93 project tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Create watcher module with self-write tracker and event processing** - `f4e1997` (feat)
2. **Task 2: Wire watcher into Tauri app lifecycle and add self-write recording** - `b610c46` (feat)

## Files Created/Modified
- `src-tauri/src/watcher/mod.rs` - File watcher initialization, event filtering/dedup, ProvidersChangedPayload, Tauri event emission
- `src-tauri/src/watcher/self_write.rs` - SelfWriteTracker with Mutex<HashMap<PathBuf, Instant>>, 1-second expiry, automatic cleanup
- `src-tauri/Cargo.toml` - Added notify and notify-debouncer-mini dependencies
- `src-tauri/src/lib.rs` - Added mod watcher, managed SelfWriteTracker state, setup() hook for watcher startup
- `src-tauri/src/commands/provider.rs` - Added AppHandle parameter and self-write recording to create/update/delete commands

## Decisions Made
- Used notify-debouncer-mini with 500ms debounce (matches FSEvents batching behavior)
- SelfWriteTracker uses 1-second expiry -- enough to cover atomic_write's rename operation
- Extracted filter_and_dedup_events as a pure function with closure parameter for testability without needing Tauri AppHandle in tests
- Used std::mem::forget(debouncer) to keep watcher alive for app lifetime (standard pattern for background watchers)
- delete_provider records self-write before deletion (vs after for create/update) since the file won't exist after

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed DebouncedEvent construction in tests**
- **Found during:** Task 1 (test compilation)
- **Issue:** DebouncedEvent.kind field is DebouncedEventKind enum, not Option (API difference from plan assumption)
- **Fix:** Changed test helper to use DebouncedEventKind::Any instead of None
- **Files modified:** src-tauri/src/watcher/mod.rs
- **Verification:** All tests compile and pass
- **Committed in:** f4e1997 (Task 1 commit)

**2. [Rule 3 - Blocking] Added mut to debouncer binding**
- **Found during:** Task 1 (compilation)
- **Issue:** new_debouncer returns debouncer that needs mutable reference for watcher() call
- **Fix:** Added `let mut debouncer` binding
- **Files modified:** src-tauri/src/watcher/mod.rs
- **Committed in:** f4e1997 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both were minor API corrections. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- File watcher backend complete, ready for frontend event listener (Plan 02)
- ProvidersChangedPayload with repatched boolean ready for frontend toast display
- Watcher starts automatically on app launch via setup() hook

---
*Phase: 04-icloud-sync-and-file-watching*
*Completed: 2026-03-11*
