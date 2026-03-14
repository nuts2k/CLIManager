---
phase: 10
slug: live-switching-ui
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-14
updated: 2026-03-14
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust: cargo test / Frontend: vitest |
| **Config file** | `src-tauri/Cargo.toml` / `vitest.config.ts` |
| **Quick run command** | `cd src-tauri && cargo test --lib commands::proxy -- --test-threads=1` |
| **Full suite command** | `cd src-tauri && cargo test --lib -- --test-threads=1` |
| **Estimated runtime** | ~13 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test --lib commands::proxy -- --test-threads=1`
- **After every plan wave:** Run `cd src-tauri && cargo test --lib -- --test-threads=1`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 10-01-01 | 01 | 1 | LIVE-02 | unit | `cargo test --lib watcher -- --test-threads=1` | yes | green |
| 10-01-02 | 01 | 1 | LIVE-03 | unit | `cargo test --lib commands::provider -- --test-threads=1` | yes | green |
| 10-01-03 | 01 | 1 | LIVE-03 | unit | `cargo test --lib commands::proxy -- --test-threads=1` | yes | green |
| 10-02-01 | 02 | 2 | UX-01 | manual | N/A (UI) | N/A | manual-only |
| 10-02-02 | 02 | 2 | UX-01 | manual | N/A (UI) | N/A | manual-only |

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Settings page global switch | UX-01 | UI interaction, Switch component state | Toggle global switch, confirm Tab switches react |
| Tab individual switch disabled logic | UX-01 | UI state depends on multiple conditions | Disabled when global off or no Provider |
| Port-in-use toast | UX-01 | Requires port occupation scenario | Occupy port 15800, try enabling proxy, confirm toast |
| Tab green status dot | UX-01 | CSS visual style | Enable proxy, confirm green dot appears on Tab label |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated

---

## Validation Audit 2026-03-14

| Metric | Count |
|--------|-------|
| Gaps found | 3 |
| Resolved | 3 |
| Escalated | 0 |

### Tests Added

**watcher/mod.rs** (5 new tests for `find_proxy_upstream_candidates`):
- `test_proxy_upstream_candidates_global_disabled_returns_empty`
- `test_proxy_upstream_candidates_no_takeover_returns_empty`
- `test_proxy_upstream_candidates_matching_changed_file`
- `test_proxy_upstream_candidates_non_matching_changed_file`
- `test_proxy_upstream_candidates_multiple_clis`

**commands/provider.rs** (6 new tests for `find_proxy_cli_ids_for_provider` + `check_proxy_blocks_delete`):
- `test_find_proxy_cli_ids_for_provider_returns_matching_cli`
- `test_find_proxy_cli_ids_for_provider_global_disabled_returns_empty`
- `test_find_proxy_cli_ids_for_provider_non_active_returns_empty`
- `test_check_proxy_blocks_delete_active_provider_in_proxy_mode`
- `test_check_proxy_blocks_delete_non_active_provider_ok`
- `test_check_proxy_blocks_delete_no_takeover_ok`

### Approach

Extracted pure testable functions from Tauri command wrappers:
- `find_proxy_upstream_candidates` (watcher) — matching logic for iCloud sync proxy linkage
- `find_proxy_cli_ids_for_provider` (provider) — matching logic for update_provider proxy linkage
- `check_proxy_blocks_delete` (provider) — validation logic for delete_provider proxy blocking

Original Tauri commands refactored to call these helpers, preserving behavior.

Total test count: 204 -> 215 (11 new tests, 0 regressions).
