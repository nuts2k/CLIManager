---
phase: 10
slug: live-switching-ui
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-14
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
| **Estimated runtime** | ~15 seconds |

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
| 10-01-01 | 01 | 1 | LIVE-02 | unit | `cargo test --lib watcher -- --test-threads=1` | ✅ | ⬜ pending |
| 10-01-02 | 01 | 1 | LIVE-03 | unit | `cargo test --lib commands::provider -- --test-threads=1` | ✅ | ⬜ pending |
| 10-01-03 | 01 | 1 | LIVE-03 | unit | `cargo test --lib commands::proxy -- --test-threads=1` | ✅ | ⬜ pending |
| 10-02-01 | 02 | 2 | UX-01 | manual | N/A (UI) | N/A | ⬜ pending |
| 10-02-02 | 02 | 2 | UX-01 | manual | N/A (UI) | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| 设置页全局开关联动 | UX-01 | UI 交互，需验证 Switch 组件状态 | 开启/关闭全局开关，确认各 Tab 独立开关状态联动 |
| Tab 独立开关置灰逻辑 | UX-01 | UI 状态依赖多条件 | 全局关闭时独立开关置灰，无 Provider 时置灰 |
| 端口占用 toast 提示 | UX-01 | 需模拟端口占用场景 | 手动占用 15800 端口后尝试开启代理，确认 toast 内容 |
| Tab 绿色状态点 | UX-01 | CSS 视觉样式 | 开启代理后确认 Tab 标签出现绿色圆点 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
