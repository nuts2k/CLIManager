---
phase: 26
slug: sqlite
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 26 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[cfg(test)]` + `cargo test` |
| **Config file** | Cargo.toml `[dev-dependencies]` |
| **Quick run command** | `cargo test -p cli-manager-lib traffic -- --nocapture` |
| **Full suite command** | `cargo test -p cli-manager-lib` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p cli-manager-lib traffic -- --nocapture`
- **After every plan wave:** Run `cargo test -p cli-manager-lib`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 26-01-01 | 01 | 1 | STORE-01 | unit | `cargo test -p cli-manager-lib traffic::db::tests::test_db_path_not_icloud` | ❌ W0 | ⬜ pending |
| 26-01-02 | 01 | 1 | STORE-01 | unit | `cargo test -p cli-manager-lib traffic::db::tests::test_wal_mode` | ❌ W0 | ⬜ pending |
| 26-01-03 | 01 | 1 | STORE-01 | unit | `cargo test -p cli-manager-lib traffic::db::tests::test_pragma_config` | ❌ W0 | ⬜ pending |
| 26-01-04 | 01 | 1 | STORE-02 | unit | `cargo test -p cli-manager-lib traffic::schema::tests::migrations_are_valid` | ❌ W0 | ⬜ pending |
| 26-01-05 | 01 | 1 | STORE-02 | unit | `cargo test -p cli-manager-lib traffic::schema::tests::migrations_are_idempotent` | ❌ W0 | ⬜ pending |
| 26-01-06 | 01 | 1 | STORE-02 | unit | `cargo test -p cli-manager-lib traffic::schema::tests::migrations_create_expected_tables` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/traffic/mod.rs` — TrafficDb struct, covers STORE-01
- [ ] `src-tauri/src/traffic/schema.rs` — MIGRATIONS constant, covers STORE-02
- [ ] `src-tauri/src/traffic/db.rs` — path resolution and open function, covers STORE-01
- [ ] Cargo.toml dependencies: `rusqlite = { version = "0.38", features = ["bundled"] }` and `rusqlite_migration = "2.4"`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| traffic.db appears in ~/Library/Application Support/ after app launch | STORE-01 | Requires actual app launch with Tauri runtime | 1. `cargo tauri dev` 2. Check `ls ~/Library/Application\ Support/com.climanager.app/traffic.db` |
| Repeated app restarts don't recreate tables | STORE-02 | Requires multiple app lifecycle cycles | 1. Launch app 2. Quit 3. Re-launch 4. Verify no migration errors in logs |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
