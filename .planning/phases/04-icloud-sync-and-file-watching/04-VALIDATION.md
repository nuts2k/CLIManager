---
phase: 4
slug: icloud-sync-and-file-watching
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-11
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test (`#[test]`) + cargo test |
| **Config file** | src-tauri/Cargo.toml (dev-dependencies: tempfile) |
| **Quick run command** | `cd src-tauri && cargo test` |
| **Full suite command** | `cd src-tauri && cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test`
- **After every plan wave:** Run `cd src-tauri && cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 04-01-01 | 01 | 1 | SYNC-03 | unit | `cd src-tauri && cargo test watcher` | ❌ W0 | ⬜ pending |
| 04-01-02 | 01 | 1 | SYNC-03 | unit | `cd src-tauri && cargo test self_write` | ❌ W0 | ⬜ pending |
| 04-01-03 | 01 | 1 | SYNC-03 | unit | `cd src-tauri && cargo test watcher::filter` | ❌ W0 | ⬜ pending |
| 04-02-01 | 02 | 2 | SYNC-04 | manual | Manual - requires Tauri runtime | N/A | ⬜ pending |
| 04-02-02 | 02 | 2 | SYNC-05 | unit | `cd src-tauri && cargo test watcher::repatch` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/watcher/self_write.rs` — unit tests for SelfWriteTracker (record, expiry, cleanup)
- [ ] `src-tauri/src/watcher/mod.rs` — unit tests for event filtering logic (json-only, dedup)

*Note: Full integration testing (watcher -> event -> frontend) requires running Tauri app; covered by manual verification*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| providers-changed event emitted with correct payload | SYNC-04 | Requires Tauri runtime and frontend event listener | 1. Start app 2. Modify a provider JSON in iCloud dir 3. Verify toast appears with correct provider name |
| UI refreshes on sync change | SYNC-04 | End-to-end requires running app | 1. Start app 2. Add/modify provider file externally 3. Verify provider list updates without manual refresh |
| CLI config re-patched on active provider sync | SYNC-05 | Requires full app + CLI config files | 1. Set a provider as active 2. Modify its JSON externally 3. Verify CLI config file reflects new values |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
