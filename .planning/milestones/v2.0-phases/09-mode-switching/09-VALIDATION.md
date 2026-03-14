---
phase: 9
slug: mode-switching
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-13
updated: 2026-03-14
---

# Phase 9 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust) |
| **Config file** | src-tauri/Cargo.toml |
| **Quick run command** | `cd src-tauri && cargo test --lib` |
| **Full suite command** | `cd src-tauri && cargo test` |
| **Estimated runtime** | ~5 seconds |

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
| 09-01-01 | 01 | 1 | MODE-01, MODE-02 | unit | `cd src-tauri && cargo test storage::local::tests` | YES | green |
| 09-01-02 | 01 | 1 | MODE-02, MODE-03 | integration | `cd src-tauri && cargo test commands::proxy::tests::test_proxy_enable_patches_cli_and_starts_proxy` | YES | green |
| 09-01-03 | 01 | 1 | MODE-04 | integration | `cd src-tauri && cargo test commands::proxy::tests::test_proxy_disable_restores_real_provider` | YES | green |
| 09-01-04 | 01 | 1 | LIVE-04 | unit | `cd src-tauri && cargo test storage::local::tests` | YES | green |
| 09-01-05 | 01 | 1 | MODE-01 | manual | N/A (proxy_set_global uses real iCloud paths) | N/A | manual |
| 09-02-01 | 02 | 2 | MODE-05 | unit | `cd src-tauri && cargo test commands::proxy::tests::test_cleanup_on_exit_sync_restores_configs` | YES | green |
| 09-02-02 | 02 | 2 | MODE-06 | unit | `cd src-tauri && cargo test commands::proxy::tests::test_recover_on_startup_clears_takeover` | YES | green |
| 09-02-03 | 02 | 2 | UX-02 | integration | `cd src-tauri && cargo test commands::proxy::tests::test_restore_proxy_state` | YES | green |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [x] `src-tauri/src/commands/proxy.rs` -- test module for proxy mode commands
- [x] Existing test infrastructure (cargo test, tempfile, mock adapters) covers framework needs

*Existing infrastructure covers framework installation requirements.*

---

## Gap Audit (2026-03-14)

### Tests Added by Nyquist Auditor

| # | Gap | Test Name | Requirement | Status |
|---|-----|-----------|-------------|--------|
| 1 | _proxy_enable_in 完整流程 | `test_proxy_enable_patches_cli_and_starts_proxy` | MODE-02, MODE-03 | green |
| 2 | _proxy_disable_in 成功路径 | `test_proxy_disable_restores_real_provider` | MODE-04 | green |
| 3 | restore_proxy_state 自动恢复 | `test_restore_proxy_state_re_enables_proxy` | UX-02 | green |
| 4 | restore_proxy_state noop (disabled) | `test_restore_proxy_state_noop_when_disabled` | UX-02 | green |
| 5 | restore_proxy_state noop (no settings) | `test_restore_proxy_state_noop_when_no_proxy_settings` | UX-02 | green |

### Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Toast 通知显示 | UX-02 | UI 视觉反馈 | 开启/关闭代理，确认 toast 出现 |
| 总开关联动 | MODE-01 | proxy_set_global 使用真实 iCloud 路径 | 通过 UI 切换全局开关，验证联动 |
| Cmd+Q 退出还原 | MODE-05 | 进程退出行为 | 开启代理后 Cmd+Q，检查 CLI 配置已还原 |
| 崩溃后重启恢复 | MODE-06 | 进程崩溃模拟 | kill -9 进程，重启检查配置已还原 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated (2026-03-14 by Nyquist Auditor)
