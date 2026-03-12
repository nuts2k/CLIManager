---
phase: 05-onboarding
plan: 01
subsystem: api
tags: [tauri-command, cli-detection, config-scanning, toml-edit, serde-json, onboarding]

# Dependency graph
requires:
  - phase: 01-storage
    provides: iCloud provider storage (save_provider_to)
  - phase: 04-icloud-sync
    provides: SelfWriteTracker for watcher coordination
provides:
  - scan_cli_configs Tauri command returning detected CLI configs from ~/.claude/ and ~/.codex/
  - import_provider Tauri command creating providers with relaxed validation (empty api_key/base_url allowed)
  - DetectedCliConfig struct for frontend consumption
affects: [05-onboarding]

# Tech tracking
tech-stack:
  added: []
  patterns: [scan_*_config_in testable internal variants, normalize_import_fields for relaxed validation]

key-files:
  created:
    - src-tauri/src/commands/onboarding.rs
  modified:
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/lib.rs

key-decisions:
  - "Separate import_provider command instead of relaxing create_provider global validation"
  - "import_provider_to internal variant for test isolation consistent with _in/_to project pattern"
  - "Model field set to empty string on import (only API key + base URL imported per CONTEXT.md)"

patterns-established:
  - "scan_*_config_in pattern: internal functions accept home_dir parameter for tempdir-based testing"
  - "import_provider_to pattern: import-specific internal variant bypasses strict provider validation"

requirements-completed: [ONBD-01]

# Metrics
duration: 4min
completed: 2026-03-12
---

# Phase 5 Plan 01: CLI Config Scan Backend Summary

**scan_cli_configs and import_provider Tauri commands for detecting Claude/Codex CLI configs and importing providers with relaxed validation**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-12T05:54:05Z
- **Completed:** 2026-03-12T05:58:22Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Implemented scan_cli_configs command that reads ~/.claude/settings.json and ~/.codex/auth.json + config.toml to detect existing CLI configurations
- Codex base_url extraction supports provider-scoped fallback (model_providers.<active>.base_url -> top-level base_url)
- Added import_provider command that creates providers without requiring non-empty api_key or base_url, enabling partial config imports
- 15 unit tests covering all edge cases: valid configs, missing files, corrupted data, partial configs, missing keys, provider-scoped URLs

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement scan_cli_configs command with TDD** - `c27745b` (feat)
2. **Task 2: Relax create_provider validation for import** - `24d0452` (feat)

## Files Created/Modified
- `src-tauri/src/commands/onboarding.rs` - DetectedCliConfig struct, scan_claude_config_in, scan_codex_config_in, scan_cli_configs, import_provider_to, import_provider commands + 15 tests
- `src-tauri/src/commands/mod.rs` - Added `pub mod onboarding` module registration
- `src-tauri/src/lib.rs` - Registered scan_cli_configs and import_provider in Tauri generate_handler

## Decisions Made
- Used separate import_provider command instead of relaxing create_provider's validate_provider -- keeps existing strict validation intact for normal provider creation while allowing partial imports during onboarding
- import_provider sets model to empty string since CONTEXT.md specifies only API key and base URL are imported
- Implemented import_provider_to internal variant for test isolation (consistent with project's _in/_to pattern from Phase 1-4)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing tauri::Manager import**
- **Found during:** Task 2 (import_provider implementation)
- **Issue:** `app_handle.state()` requires `tauri::Manager` trait in scope
- **Fix:** Added `use tauri::Manager;` to onboarding.rs imports
- **Files modified:** src-tauri/src/commands/onboarding.rs
- **Verification:** Compilation succeeded, all tests pass
- **Committed in:** 24d0452 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Trivial import fix necessary for compilation. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- scan_cli_configs and import_provider commands ready for frontend onboarding dialog (Plan 02)
- DetectedCliConfig struct available for TypeScript type generation

---
*Phase: 05-onboarding*
*Completed: 2026-03-12*
