---
phase: 1
slug: storage-and-data-model
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-10
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | vitest (frontend) + cargo test (Rust backend) |
| **Config file** | vitest.config.ts + Cargo.toml |
| **Quick run command** | `pnpm test -- --run` |
| **Full suite command** | `pnpm test -- --run && cd src-tauri && cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `pnpm test -- --run`
- **After every plan wave:** Run `pnpm test -- --run && cd src-tauri && cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | ADPT-03 | unit | `cd src-tauri && cargo test provider` | ❌ W0 | ⬜ pending |
| 01-01-02 | 01 | 1 | SYNC-01 | unit | `cd src-tauri && cargo test storage` | ❌ W0 | ⬜ pending |
| 01-02-01 | 02 | 1 | SYNC-02 | unit | `cd src-tauri && cargo test local` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/tests/` — test stubs for provider model, iCloud storage, local settings
- [ ] vitest + cargo test setup in project scaffold
- [ ] Test fixtures for sample provider JSON files

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| iCloud Drive directory creation | SYNC-01 | Requires actual iCloud Drive access | 1. Launch app 2. Verify directory created in ~/Library/Mobile Documents/ |
| Tauri 2 build success | N/A (SC4) | Build process, not runtime behavior | 1. Run `pnpm tauri build` 2. Verify .app produced |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
