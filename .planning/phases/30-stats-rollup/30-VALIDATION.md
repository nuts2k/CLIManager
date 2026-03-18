---
phase: 30
slug: stats-rollup
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 30 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust) + vitest (前端，如已有) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test -p cli-manager-lib rollup` |
| **Full suite command** | `cargo test -p cli-manager-lib` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p cli-manager-lib rollup`
- **After every plan wave:** Run `cargo test -p cli-manager-lib`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 30-01-01 | 01 | 0 | STORE-04 | unit | `cargo test -p cli-manager-lib rollup::tests::test_rollup_moves_old_logs` | ❌ W0 | ⬜ pending |
| 30-01-02 | 01 | 0 | STORE-04 | unit | `cargo test -p cli-manager-lib rollup::tests::test_prune_deletes_old_logs` | ❌ W0 | ⬜ pending |
| 30-01-03 | 01 | 0 | STORE-04 | unit | `cargo test -p cli-manager-lib rollup::tests::test_prune_deletes_old_rollups` | ❌ W0 | ⬜ pending |
| 30-01-04 | 01 | 0 | STORE-04 | unit | `cargo test -p cli-manager-lib rollup::tests::test_rollup_idempotent` | ❌ W0 | ⬜ pending |
| 30-01-05 | 01 | 0 | STAT-02 | unit | `cargo test -p cli-manager-lib rollup::tests::test_query_provider_stats_24h` | ❌ W0 | ⬜ pending |
| 30-01-06 | 01 | 0 | STAT-03 | unit | `cargo test -p cli-manager-lib rollup::tests::test_query_hourly_trend` | ❌ W0 | ⬜ pending |
| 30-02-01 | 02 | 1 | STAT-04 | manual | 目测 recharts 图表双轴正确渲染 | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/traffic/rollup.rs` — 新文件，包含 rollup_and_prune 方法和 `#[cfg(test)]` 测试模块
- [ ] 测试辅助函数 `make_test_db_with_logs()` — 在 rollup.rs tests 模块内定义（与 log.rs 的 make_test_db 模式一致）

*Existing cargo test infrastructure covers all Rust unit tests.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| recharts 双轴图表渲染 | STAT-04 | 图表视觉输出无法自动化断言 | 打开统计分析 Tab，确认柱状图(请求量)+折线图(Token)双轴正确展示 |
| Tab 切换视觉一致性 | STAT-02/03 | CSS 布局需目测 | 切换实时日志/统计分析 Tab，确认 line 下划线样式与 Settings 页一致 |
| 24h/7d 数据联动 | STAT-02/03/04 | 端到端交互流 | 切换 Segment 按钮，确认排行榜+图表数据同步更新 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
