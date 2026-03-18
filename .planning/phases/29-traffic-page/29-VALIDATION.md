---
phase: 29
slug: traffic-page
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 29 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust cargo test (backend); 无前端测试框架 |
| **Config file** | src-tauri/Cargo.toml |
| **Quick run command** | `cd src-tauri && cargo test --lib traffic` |
| **Full suite command** | `cd src-tauri && cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test --lib traffic`（如有 Rust 改动）
- **After every plan wave:** Run `cd src-tauri && cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green + 手动验证 5 个 success criteria
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 29-01-01 | 01 | 1 | LOG-02 | manual | 手动检查 TrafficPage 渲染 | N/A | ⬜ pending |
| 29-01-02 | 01 | 1 | LOG-02 | unit (Rust) | `cd src-tauri && cargo test --lib traffic::log` | ✅ | ⬜ pending |
| 29-01-03 | 01 | 1 | LOG-03 | manual | 手动检查 Provider 筛选 | N/A | ⬜ pending |
| 29-01-04 | 01 | 1 | STAT-01 | manual | 手动检查统计卡片 | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.*

本阶段纯前端实现，后端 Rust 测试已覆盖 `get_recent_logs` command。前端无 vitest/jest 框架，验收依赖手动测试 success criteria。不需要在 Wave 0 建立前端测试基础设施（超出本阶段范围）。

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| TrafficPage 正确渲染日志表格 | LOG-02 | 无前端测试框架 | 启动应用，点击 Traffic 图标，确认表格展示日志（时间、Provider、模型、状态码、Token、耗时列） |
| 新请求自动追加到表格顶部 | LOG-02 | 需要实时事件验证 | 发送 API 请求，确认新日志条目无需刷新自动追加 |
| Provider 筛选正确过滤 | LOG-03 | 无前端测试框架 | 选择某 Provider，确认表格只显示该 Provider 的日志 |
| 统计摘要卡片数值正确 | STAT-01 | 无前端测试框架 | 确认请求数、Input Token、Output Token、成功率、缓存命中率卡片数值与表格数据一致 |
| 导航 toggle 行为正确 | LOG-02 | UI 交互验证 | 点击 Traffic 图标进入流量页，再点一次回到 main 视图 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
