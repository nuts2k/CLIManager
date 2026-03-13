---
phase: 9
slug: mode-switching
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-13
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
| 09-01-01 | 01 | 1 | MODE-01, MODE-02 | unit | `cd src-tauri && cargo test storage::local::tests` | ✅ | ⬜ pending |
| 09-01-02 | 01 | 1 | MODE-03, MODE-04 | unit | `cd src-tauri && cargo test commands::proxy::tests` | ❌ W0 | ⬜ pending |
| 09-01-03 | 01 | 1 | LIVE-04 | unit | `cd src-tauri && cargo test storage::local::tests` | ✅ | ⬜ pending |
| 09-01-04 | 01 | 1 | UX-02 | unit | `cd src-tauri && cargo test commands::proxy::tests` | ❌ W0 | ⬜ pending |
| 09-02-01 | 02 | 2 | MODE-05 | unit | `cd src-tauri && cargo test` | ❌ W0 | ⬜ pending |
| 09-02-02 | 02 | 2 | MODE-06 | unit | `cd src-tauri && cargo test` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/commands/proxy.rs` — test module for proxy mode commands
- [ ] Existing test infrastructure (cargo test, tempfile, mock adapters) covers framework needs

*Existing infrastructure covers framework installation requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Toast 通知显示 | UX-02 | UI 视觉反馈 | 开启/关闭代理，确认 toast 出现 |
| 总开关置灰联动 | MODE-01 | UI 状态反馈 | 关闭总开关，确认 CLI 开关置灰 |
| Cmd+Q 退出还原 | MODE-05 | 进程退出行为 | 开启代理后 Cmd+Q，检查 CLI 配置已还原 |
| 崩溃后重启恢复 | MODE-06 | 进程崩溃模拟 | kill -9 进程，重启检查配置已还原 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
