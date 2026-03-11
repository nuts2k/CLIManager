---
phase: 02-surgical-patch-engine
plan: 01
subsystem: adapter
tags: [serde_json, preserve_order, json-patch, backup, cli-adapter]

# Dependency graph
requires:
  - phase: 01-storage-and-data-model
    provides: "Provider struct, atomic_write utility, AppError enum, _in/_to test pattern"
provides:
  - "CliAdapter trait for unified CLI config patching interface"
  - "PatchResult struct for tracking files written and backups created"
  - "create_backup and rotate_backups shared utilities"
  - "ClaudeAdapter implementing surgical JSON patching of settings.json"
  - "AppError::Toml and AppError::Validation error variants"
affects: [02-02-codex-adapter, 03-ui, 04-icloud-sync]

# Tech tracking
tech-stack:
  added: [toml_edit 0.25, serde_json preserve_order feature]
  patterns: [CliAdapter trait, surgical Value-level JSON merge, timestamped backup with rotation]

key-files:
  created:
    - src-tauri/src/adapter/mod.rs
    - src-tauri/src/adapter/claude.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/error.rs
    - src-tauri/src/lib.rs

key-decisions:
  - "Used serde_json::Value merge for surgical JSON patching instead of struct deserialization"
  - "Backup uses fs::copy (not atomic_write) since it is a safety copy, not a critical write"
  - "ClaudeAdapter uses new_with_paths constructor for test isolation (consistent with Phase 1 _in/_to pattern)"
  - "Added #[derive(Debug)] to PatchResult for test ergonomics (unwrap_err)"

patterns-established:
  - "CliAdapter trait: cli_name() + patch() -- backup/validate are internal to each adapter's patch()"
  - "new_with_paths() constructor for adapter test isolation with tempdir"
  - "Backup naming: {filename}.{YYYY-MM-DDTHH-MM-SS.mmm}.bak"

requirements-completed: [PTCH-01, PTCH-02, PTCH-03, PTCH-04, ADPT-01]

# Metrics
duration: 4min
completed: 2026-03-11
---

# Phase 2 Plan 1: CliAdapter Trait and Claude Adapter Summary

**Surgical JSON patching of Claude Code settings.json via serde_json preserve_order, with timestamped backup rotation and CliAdapter trait foundation**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-11T03:51:58Z
- **Completed:** 2026-03-11T03:56:05Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Defined CliAdapter trait and PatchResult struct as the unified adapter interface
- Implemented create_backup (timestamped .bak) and rotate_backups (max N, oldest-first deletion) utilities
- Built ClaudeAdapter that surgically patches only env.ANTHROPIC_AUTH_TOKEN and env.ANTHROPIC_BASE_URL in settings.json
- 15 total tests: 6 for backup/rotate/error variants, 9 for Claude adapter patching behavior
- Full 42-test suite passes with zero regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Add dependencies, extend AppError, define CliAdapter trait and backup utilities** - `6b0597e` (feat)
2. **Task 2: Implement ClaudeAdapter with surgical JSON patching** - `29a8c46` (feat)

## Files Created/Modified
- `src-tauri/Cargo.toml` - Added toml_edit dep, enabled serde_json preserve_order
- `src-tauri/src/error.rs` - Extended AppError with Toml and Validation variants
- `src-tauri/src/lib.rs` - Added mod adapter
- `src-tauri/src/adapter/mod.rs` - CliAdapter trait, PatchResult, backup/rotate utilities with 6 tests
- `src-tauri/src/adapter/claude.rs` - ClaudeAdapter with surgical JSON patching and 9 tests

## Decisions Made
- Used serde_json::Value merge for surgical patching (preserves all unknown keys, ordering, nesting)
- Backup uses fs::copy not atomic_write (safety copy, not a critical write path)
- Added #[derive(Debug)] to PatchResult for test ergonomics (needed for unwrap_err in validation test)
- Kept backup/validate as internal details of each adapter's patch() method, not separate trait methods

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added #[derive(Debug)] to PatchResult**
- **Found during:** Task 2 (ClaudeAdapter tests)
- **Issue:** unwrap_err() requires Debug trait on PatchResult, compilation failed
- **Fix:** Added `#[derive(Debug)]` to PatchResult struct in mod.rs
- **Files modified:** src-tauri/src/adapter/mod.rs
- **Verification:** All tests compile and pass
- **Committed in:** 29a8c46 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Minimal -- standard derive addition for test support. No scope creep.

## Issues Encountered
None -- plan executed cleanly.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CliAdapter trait and shared utilities ready for CodexAdapter implementation (plan 02-02)
- toml_edit dependency already added for Codex TOML patching
- All Phase 1 tests still passing (no regressions)

---
*Phase: 02-surgical-patch-engine*
*Completed: 2026-03-11*

## Self-Check: PASSED
- All 6 created/modified files verified on disk
- Commits 6b0597e and 29a8c46 verified in git log
- 42/42 tests passing (15 new + 27 existing)
