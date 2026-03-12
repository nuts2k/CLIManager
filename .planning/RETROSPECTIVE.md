# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-03-12
**Phases:** 5 | **Plans:** 12 | **Commits:** 85

### What Was Built
- Two-layer storage architecture (iCloud + local) with per-provider JSON files
- Surgical patch engine — CliAdapter trait with JSON merge and TOML comment-preserving editing
- Full provider management UI with CLI-tabbed interface, CRUD dialogs, one-click switching
- FSEvents-based file watcher with self-write tracking for live iCloud sync
- First-launch onboarding with automatic CLI config detection and selective import
- Chinese/English i18n with runtime language switching

### What Worked
- Bottom-up phase ordering (storage -> engine -> UI -> sync -> onboarding) kept each phase cleanly buildable on the previous
- _in/_to internal function variant pattern provided consistent test isolation across all modules
- serde_json preserve_order + toml_edit::DocumentMut perfectly solved the surgical patch core value
- Per-file JSON in iCloud Drive eliminated sync conflict concerns entirely
- 12 plans averaging 6 minutes each — very efficient execution

### What Was Inefficient
- Some ROADMAP plan checkboxes were left unchecked despite plans being executed (Phase 2, 3 plan items showed `[ ]` instead of `[x]`)
- Research flagged potential issues (config schema verification, iCloud eviction detection) that weren't resolved during v1.0 — deferred as known issues

### Patterns Established
- `_in/_to` internal function variants for filesystem test isolation (used in storage, adapters, onboarding)
- Thin Tauri command wrappers delegating to pure-logic modules
- SelfWriteTracker pattern for preventing watcher infinite loops
- Dark-only theme with CSS variables on :root (zinc palette)
- Hook-per-domain pattern (useProviders, useSettings, useSyncListener)

### Key Lessons
1. Surgical patching via Value-level merge (not string manipulation) is the right approach for preserving config structure
2. iCloud sync is easy when you design for it from the start (per-file JSON, two-layer separation)
3. Self-write tracking with expiry windows is essential for FSEvents-based watchers on macOS
4. Separating import_provider from create_provider keeps validation strict for normal flows while allowing onboarding flexibility

### Cost Observations
- Model mix: quality profile (opus-based)
- Total execution: ~1.12 hours for 12 plans
- Notable: 3-day MVP from requirements to shipped — efficient phase decomposition and parallel-ready architecture

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 85 | 5 | Initial project — established patterns and conventions |

### Cumulative Quality

| Milestone | LOC | Files | Avg Plan Duration |
|-----------|-----|-------|-------------------|
| v1.0 | 7,986 | 143 | 6min |

### Top Lessons (Verified Across Milestones)

1. Design for iCloud from the start — per-file JSON + two-layer storage eliminates sync conflicts
2. Surgical patching via structured data merge preserves what whole-file writes destroy
