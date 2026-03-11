---
phase: 2
slug: surgical-patch-engine
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-11
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in Rust test framework) |
| **Config file** | `src-tauri/Cargo.toml` (test deps: `tempfile = "3"`) |
| **Quick run command** | `cargo test --manifest-path src-tauri/Cargo.toml` |
| **Full suite command** | `cargo test --manifest-path src-tauri/Cargo.toml` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --manifest-path src-tauri/Cargo.toml`
- **After every plan wave:** Run `cargo test --manifest-path src-tauri/Cargo.toml`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 0 | PTCH-01 | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter` | ❌ W0 | ⬜ pending |
| 02-01-02 | 01 | 0 | PTCH-02 | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter` | ❌ W0 | ⬜ pending |
| 02-01-03 | 01 | 0 | PTCH-03 | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter` | ❌ W0 | ⬜ pending |
| 02-01-04 | 01 | 0 | PTCH-04 | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter` | ❌ W0 | ⬜ pending |
| 02-02-01 | 02 | 1 | ADPT-01 | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter::claude` | ❌ W0 | ⬜ pending |
| 02-03-01 | 03 | 1 | ADPT-02 | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter::codex` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/adapter/mod.rs` — trait definition + backup/rotate utilities + tests
- [ ] `src-tauri/src/adapter/claude.rs` — Claude adapter stubs + tests for PTCH-01..04, ADPT-01
- [ ] `src-tauri/src/adapter/codex.rs` — Codex adapter stubs + tests for ADPT-02
- [ ] Add `toml_edit = "0.25"` to Cargo.toml dependencies
- [ ] Enable `preserve_order` feature on `serde_json` in Cargo.toml

*Existing infrastructure: `tempfile` in dev-deps, `atomic_write` in storage module.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
