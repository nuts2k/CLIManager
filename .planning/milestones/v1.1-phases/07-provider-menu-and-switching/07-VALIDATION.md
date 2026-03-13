---
phase: 7
slug: provider-menu-and-switching
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-13
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test (cargo test) |
| **Config file** | Cargo.toml (existing) |
| **Quick run command** | `cd src-tauri && cargo test --lib` |
| **Full suite command** | `cd src-tauri && cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test --lib`
- **After every plan wave:** Run `cd src-tauri && cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 07-01-01 | 01 | 1 | PROV-01 | unit | `cd src-tauri && cargo test tray::tests::test_provider_sorting` | ❌ W0 | ⬜ pending |
| 07-01-02 | 01 | 1 | PROV-01 | unit | `cd src-tauri && cargo test tray::tests::test_empty_cli_groups` | ❌ W0 | ⬜ pending |
| 07-01-03 | 01 | 1 | PROV-01 | unit | `cd src-tauri && cargo test tray::tests::test_menu_layout` | ❌ W0 | ⬜ pending |
| 07-01-04 | 01 | 1 | PROV-02 | unit | `cd src-tauri && cargo test tray::tests::test_provider_switch` | ❌ W0 | ⬜ pending |
| 07-01-05 | 01 | 1 | MENU-03 | unit | `cd src-tauri && cargo test tray::tests::test_tray_texts_i18n` | ❌ W0 | ⬜ pending |
| 07-02-01 | 02 | 1 | PROV-03 | manual | Manual: create provider in UI, verify tray updates | N/A | ⬜ pending |
| 07-02-02 | 02 | 1 | MENU-03 | manual | Manual: switch language in settings, verify tray labels | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/tray.rs` — add `#[cfg(test)] mod tests` section with unit tests for sorting, i18n, ID parsing, empty group hiding
- [ ] Test helper functions for creating mock provider lists and settings (reuse patterns from `commands/provider.rs` tests)

*Existing infrastructure covers framework setup — only test stubs needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Tray menu lists providers grouped by CLI with checkmarks | PROV-01 | Requires running Tauri app with system tray | Launch app, verify tray menu shows CLI groups with providers |
| Provider switch from tray updates CLI config | PROV-02 | Requires running Tauri app + filesystem verification | Click provider in tray, check CLI config file updated |
| Tray rebuilds after frontend CRUD / iCloud sync | PROV-03 | Requires running app + simulated file changes | Add/edit/delete provider in UI, verify tray reflects changes |
| Language change updates tray labels | MENU-03 | Requires running app + language switch | Change language in settings, verify tray labels update |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
