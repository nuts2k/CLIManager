# Milestones

## v1.0 MVP (Shipped: 2026-03-12)

**Phases completed:** 5 phases, 12 plans, 0 tasks

**Key accomplishments:**
- Two-layer storage architecture (iCloud + local) with per-provider JSON files for conflict-free sync
- Surgical patch engine with CliAdapter trait — JSON and TOML patching that preserves comments and unknown keys
- Full provider management UI with CLI-tabbed interface, CRUD dialogs, and one-click switching
- FSEvents-based file watcher with self-write tracking for live iCloud sync and auto re-patching
- First-launch onboarding with automatic CLI config detection and selective import
- Chinese/English i18n from day one with runtime language switching

**Stats:**
- 85 commits, 143 files, 7,986 LOC (Rust + TypeScript/React)
- Timeline: 3 days (2026-03-10 to 2026-03-12)
- Execution time: ~1.12 hours total (avg 6min/plan)
- Git range: feat(01-01) to feat(05-02)

---

