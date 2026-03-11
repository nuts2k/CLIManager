---
phase: 03-provider-management-ui
plan: 01
subsystem: api
tags: [rust, tauri, provider, cli-adapter, reqwest, serde]

# Dependency graph
requires:
  - phase: 01-storage-foundation
    provides: Provider struct, iCloud CRUD, LocalSettings, atomic_write
  - phase: 02-surgical-patch-engine
    provides: CliAdapter trait, ClaudeAdapter, CodexAdapter with surgical patching
provides:
  - Provider struct with cli_id field for per-CLI scoping
  - LocalSettings with active_providers HashMap for per-CLI active state
  - set_active_provider command that triggers adapter.patch() on switch
  - delete_provider with auto-switch to next available provider
  - test_provider async command for Anthropic and OpenAI-compatible protocols
  - update_local_settings command for language/test_config persistence
  - TestConfig struct for configurable test timeouts
affects: [03-02-provider-list-ui, 03-03-provider-form-ui, 03-04-settings-ui]

# Tech tracking
tech-stack:
  added: [reqwest]
  patterns: [_internal_in pattern for command test isolation with injectable adapters, per-CLI active_providers HashMap, skip_serializing for backward compat migration]

key-files:
  created: []
  modified:
    - src-tauri/src/provider.rs
    - src-tauri/src/storage/local.rs
    - src-tauri/src/commands/provider.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/error.rs
    - src-tauri/Cargo.toml

key-decisions:
  - "Used skip_serializing on old active_provider_id for backward-compat read-only migration to active_providers HashMap"
  - "Injectable adapter via Option<Box<dyn CliAdapter>> parameter for command test isolation"
  - "Auto-switch picks first provider sorted by created_at (circular from index 0)"
  - "test_provider uses reqwest with configurable timeout from LocalSettings.test_config"

patterns-established:
  - "_internal_in with injectable adapter: Commands have internal variants accepting adapter parameter for testability without filesystem mocking"
  - "Per-CLI state: active_providers HashMap<String, Option<String>> replaces single active_provider_id"

requirements-completed: [PROV-01, PROV-02, PROV-03, PROV-04, PROV-05, PROV-06]

# Metrics
duration: 5min
completed: 2026-03-11
---

# Phase 3 Plan 1: Backend Provider Commands Summary

**Per-CLI provider management with cli_id scoping, switch-triggers-patch, delete-auto-switch, and async test_provider using reqwest**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-11T06:31:55Z
- **Completed:** 2026-03-11T06:37:47Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Provider struct extended with cli_id field (backward-compatible default "claude")
- LocalSettings refactored to per-CLI active_providers HashMap with backward-compat migration
- set_active_provider triggers adapter.patch() automatically on switch
- delete_provider auto-switches to next available provider when active is deleted
- test_provider async command supports both Anthropic and OpenAI-compatible protocols
- All 76 tests pass including 10 new command tests

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend Provider with cli_id and LocalSettings with per-CLI active_providers** - `499255a` (feat)
2. **Task 2: Refactor Tauri commands for cli_id filtering, switch+patch, delete+auto-switch, and test_provider** - `c2e9cad` (feat)

_Note: TDD tasks - tests written first (RED), then implementation (GREEN) in same commit_

## Files Created/Modified
- `src-tauri/src/provider.rs` - Added cli_id field with serde default "claude"
- `src-tauri/src/storage/local.rs` - Added active_providers HashMap, TestConfig, language field
- `src-tauri/src/commands/provider.rs` - Full refactor: cli_id filtering, switch+patch, delete+auto-switch, test_provider, update_local_settings
- `src-tauri/src/lib.rs` - Registered test_provider and update_local_settings commands
- `src-tauri/src/error.rs` - Added Http error variant for reqwest
- `src-tauri/Cargo.toml` - Added reqwest dependency
- `src-tauri/src/adapter/claude.rs` - Updated test helper with cli_id
- `src-tauri/src/adapter/codex.rs` - Updated test helper with cli_id
- `src-tauri/src/adapter/mod.rs` - Updated integration test with cli_id
- `src-tauri/src/storage/icloud.rs` - Updated test helper with cli_id

## Decisions Made
- Used skip_serializing on old active_provider_id for backward-compat read-only migration to active_providers HashMap
- Injectable adapter via Option<Box<dyn CliAdapter>> parameter for command test isolation (avoids filesystem mocking)
- Auto-switch picks first available provider sorted by created_at when deleting the active one
- test_provider uses reqwest with configurable timeout from LocalSettings.test_config

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All backend commands ready for frontend wiring (Plan 02: Provider List UI)
- Commands support per-CLI filtering, one-click switching, auto-switch on delete, and connectivity testing
- TestResult struct ready for UI display

---
*Phase: 03-provider-management-ui*
*Completed: 2026-03-11*
