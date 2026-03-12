---
phase: 3
slug: provider-management-ui
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-11
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (backend) + vitest (frontend) |
| **Config file** | `src-tauri/Cargo.toml` (existing) / `vitest.config.ts` (Wave 0 installs) |
| **Quick run command** | `cd src-tauri && cargo test` or `pnpm vitest run` |
| **Full suite command** | `cd src-tauri && cargo test && cd .. && pnpm vitest run` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test` (Rust) or `pnpm vitest run` (frontend)
- **After every plan wave:** Run `cd src-tauri && cargo test && cd .. && pnpm vitest run`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 0 | PROV-01 | unit (Rust) | `cd src-tauri && cargo test commands::provider::tests::test_create_provider_with_cli_id` | ❌ W0 | ⬜ pending |
| 03-01-02 | 01 | 0 | PROV-02 | unit (Rust) | `cd src-tauri && cargo test commands::provider::tests::test_list_by_cli_id` | ❌ W0 | ⬜ pending |
| 03-01-03 | 01 | 0 | PROV-04 | unit (Rust) | `cd src-tauri && cargo test commands::provider::tests::test_delete_active_auto_switch` | ❌ W0 | ⬜ pending |
| 03-01-04 | 01 | 0 | PROV-05 | unit (Rust) | `cd src-tauri && cargo test storage::local::tests::test_per_cli_active_providers` | ❌ W0 | ⬜ pending |
| 03-01-05 | 01 | 0 | PROV-06 | unit (Rust) | `cd src-tauri && cargo test commands::provider::tests::test_switch_triggers_patch` | ❌ W0 | ⬜ pending |
| 03-02-01 | 02 | 0 | I18N-02 | unit (frontend) | `pnpm vitest run src/i18n` | ❌ W0 | ⬜ pending |
| 03-02-02 | 02 | 0 | I18N-03 | unit (frontend) | `pnpm vitest run src/hooks/useSettings` | ❌ W0 | ⬜ pending |
| 03-03-01 | 03 | 2 | I18N-01 | manual | Visual inspection | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `vitest.config.ts` — frontend test config with jsdom environment
- [ ] `pnpm add -D vitest @testing-library/react @testing-library/jest-dom jsdom` — test dependencies
- [ ] Rust unit tests for cli_id filtering in list_providers
- [ ] Rust unit tests for per-CLI active_providers in LocalSettings
- [ ] Rust unit tests for delete-active-auto-switch logic
- [ ] Rust unit tests for set_active_provider triggering adapter patch
- [ ] Frontend unit tests for i18n default language and switching

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| All UI text externalized in both languages | I18N-01 | Visual inspection needed to verify no hardcoded text | Switch language in settings, verify every visible string changes |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
