---
phase: 5
slug: onboarding
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-12
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (built-in) |
| **Config file** | `src-tauri/Cargo.toml` |
| **Quick run command** | `cd src-tauri && cargo test --lib -q` |
| **Full suite command** | `cd src-tauri && cargo test --lib -q` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test --lib -q`
- **After every plan wave:** Run `cd src-tauri && cargo test --lib -q`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 01 | 1 | ONBD-01 | unit | `cd src-tauri && cargo test scan_claude -q` | ❌ W0 | ⬜ pending |
| 05-01-02 | 01 | 1 | ONBD-01 | unit | `cd src-tauri && cargo test scan_codex -q` | ❌ W0 | ⬜ pending |
| 05-01-03 | 01 | 1 | ONBD-01 | unit | `cd src-tauri && cargo test scan_missing -q` | ❌ W0 | ⬜ pending |
| 05-01-04 | 01 | 1 | ONBD-01 | unit | `cd src-tauri && cargo test scan_corrupted -q` | ❌ W0 | ⬜ pending |
| 05-01-05 | 01 | 1 | ONBD-01 | unit | `cd src-tauri && cargo test scan_no_key -q` | ❌ W0 | ⬜ pending |
| 05-02-01 | 02 | 1 | ONBD-02 | existing | `cd src-tauri && cargo test create_provider -q` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Backend scan tests — unit tests for `scan_cli_configs` logic (scan_claude_config, scan_codex_config, edge cases)
- [ ] No frontend test framework exists — frontend testing is manual-only for this project

*Frontend import dialog, trigger logic, and settings button are manual verification only since no frontend test infrastructure exists.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Import dialog appears on first launch when no providers exist | ONBD-01 | UI interaction flow, no frontend test framework | Launch app with empty provider store, verify dialog appears |
| Checkbox selection and masked API key preview display | ONBD-01 | Visual UI verification | Open import dialog, verify each detected config shows masked key |
| Skip button dismisses dialog and shows main UI | ONBD-01 | UI interaction | Click "跳过", verify dialog closes and main UI is shown |
| Import creates providers and shows success toast | ONBD-01 | End-to-end UI flow | Select items, click "导入已选项", verify providers appear in list |
| Settings page import button triggers import flow | ONBD-02 | UI interaction | Navigate to settings, click import button, verify dialog appears |
| Manual provider creation still works independently | ONBD-02 | Already covered by Phase 3 | Create provider via "+" button, verify it works |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
