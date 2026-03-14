---
phase: 11
slug: proxy-awareness-fixes
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-14
---

# Phase 11 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust: cargo test (lib tests) |
| **Config file** | `src-tauri/Cargo.toml` |
| **Quick run command** | `cd src-tauri && cargo test --lib commands::provider -- --test-threads=1` |
| **Full suite command** | `cd src-tauri && cargo test --lib -- --test-threads=1` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test --lib commands::provider -- --test-threads=1`
- **After every plan wave:** Run `cd src-tauri && cargo test --lib -- --test-threads=1`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 11-01-01 | 01 | 1 | LIVE-01 | unit | `cd src-tauri && cargo test --lib tray -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 11-01-02 | 01 | 1 | LIVE-03 | unit | `cd src-tauri && cargo test --lib commands::provider -- --test-threads=1` | ✅ (extend) | ⬜ pending |
| 11-01-03 | 01 | 1 | UX-01 | manual | N/A (doc check) | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] tray 模块代理感知测试 — 提取 `handle_provider_click` 内部逻辑为可测试纯函数，测试代理模式下不调用 `patch_provider_for_cli`
- [ ] `_update_provider_in` 代理模式参数扩展测试 — 若修改签名，更新现有测试覆盖 `skip_patch=true` 分支

*若 tray 测试难以单元化（涉及 AppHandle mock），可接受提取 `is_proxy_mode_active` 逻辑为独立可测函数 + 手工集成验证。*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| 文档复选框和 SUMMARY 字段正确 | UX-01 | 文档内容验证，非运行时行为 | 1. 检查 REQUIREMENTS.md UX-01 为 `[x]` 2. 检查 10-02-SUMMARY.md frontmatter 包含 `requirements-completed: [UX-01]` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
