---
phase: 30-stats-rollup
plan: "01"
subsystem: database
tags: [rusqlite, sqlite, rollup, aggregation, tauri-command, tokio]

requires:
  - phase: 26-sqlite
    provides: TrafficDb 结构体、request_logs 和 daily_rollups 表 schema、init_traffic_db
  - phase: 27-log-writer
    provides: log_worker、insert_request_log 等写入方法
  - phase: 28-stream-log
    provides: update_streaming_log（流式记录补全）

provides:
  - rollup_and_prune：单次 SQLite 事务聚合超 24h 明细 + 删除超 24h logs + 删除超 7d rollups
  - query_provider_stats("24h"/"7d")：按 Provider 聚合 ProviderStat Vec
  - query_time_trend("24h"/"7d")：按小时/天聚合 TimeStat Vec
  - get_provider_stats Tauri command（供前端统计分析 Tab 调用）
  - get_time_trend Tauri command（供前端趋势图调用）
  - lib.rs rollup 定时任务（启动后立即执行一次，之后每小时重复）

affects: [30-stats-rollup/30-02, 前端统计分析 Tab（Phase 30 后续 Plan）]

tech-stack:
  added: []
  patterns:
    - "ON CONFLICT(provider_name, rollup_date) DO UPDATE SET col = col + excluded.col（增量 upsert，不丢历史）"
    - "loop + tokio::time::sleep 后台定时任务模式（首次立即执行）"
    - "tauri::async_runtime::spawn 而非 tokio::spawn（Tauri 2 兼容）"

key-files:
  created:
    - src-tauri/src/traffic/rollup.rs
  modified:
    - src-tauri/src/traffic/mod.rs
    - src-tauri/src/commands/traffic.rs
    - src-tauri/src/lib.rs

key-decisions:
  - "rollup_and_prune 使用 ON CONFLICT DO UPDATE SET 增量 upsert（而非 INSERT OR REPLACE），防止多次 rollup 丢失累积数据（RESEARCH.md Pitfall 1）"
  - "created_at 是 epoch 毫秒，SQL 阈值比较统一乘以 1000 转换（RESEARCH.md Pitfall 2）"
  - "lib.rs 定时任务使用 loop + tokio::time::sleep（更清晰，避免 tokio::interval 首次 tick 双重执行问题）"
  - "rollup.rs tests 模块使用真实超 24h 前时间戳（chrono::Utc::now().timestamp() - 86400 - 3600）而非固定值"

patterns-established:
  - "TrafficDb impl 方法分文件（log.rs、rollup.rs），通过 impl super::TrafficDb 在子模块中扩展"
  - "单元测试辅助函数 insert_log() 就近定义于 tests 模块，与 log.rs 的 make_full_entry 模式一致"

requirements-completed: [STORE-04, STAT-02, STAT-03]

duration: 6min
completed: "2026-03-18"
---

# Phase 30 Plan 01: 统计聚合后端 Summary

**rollup_and_prune 定时任务 + ProviderStat/TimeStat 聚合查询接口，通过 ON CONFLICT upsert 保证增量安全，8 个单元测试全部通过**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-18T13:44:55Z
- **Completed:** 2026-03-18T13:51:05Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- 新建 `rollup.rs`：rollup_and_prune 单次 SQLite 事务原子完成聚合+清理，ON CONFLICT upsert 保证幂等安全
- query_provider_stats / query_time_trend 覆盖 24h（request_logs）和 7d（daily_rollups）两个数据源
- 8 个单元测试全部通过（rollup 聚合、prune 删除超期行、7d rollup 清理、幂等性、24h/7d 查询）
- lib.rs 注册两个新 Tauri command + 启动 rollup 定时任务（首次立即执行，每小时重复）
- 446 个全量测试通过，cargo build 无错误

## Task Commits

每个任务均原子提交：

1. **Task 1: rollup.rs rollup_and_prune + 聚合查询 + 单元测试** - `961ced2` (feat)
2. **Task 2: Tauri commands + lib.rs 定时任务注册** - `32184f9` (feat)

**Plan 元数据：** 待创建（docs commit）

## Files Created/Modified

- `src-tauri/src/traffic/rollup.rs` - ProviderStat/TimeStat 数据结构 + rollup_and_prune + query_provider_stats + query_time_trend + 8 个单元测试
- `src-tauri/src/traffic/mod.rs` - 新增 `pub mod rollup;` 声明（按字母排序）
- `src-tauri/src/commands/traffic.rs` - 新增 get_provider_stats 和 get_time_trend 两个 Tauri command
- `src-tauri/src/lib.rs` - invoke_handler 注册新 command + setup 闭包新增 rollup 定时任务

## Decisions Made

- ON CONFLICT DO UPDATE SET 增量 upsert 而非 INSERT OR REPLACE：防止同一天同 provider 多次 rollup 时覆盖历史累积数据
- loop + tokio::time::sleep 模式：首次立即执行后 sleep(3600s)，比 tokio::interval 更直观（interval 第一个 tick 立即触发，易导致双重执行）
- 失败只 log::warn 不重试：等下一轮（1h 后）自动触发，避免堆积错误

## Deviations from Plan

无 — 计划完全按规范执行。

## Issues Encountered

无。

## User Setup Required

无 — 不需要任何外部服务配置。

## Next Phase Readiness

- 后端聚合接口已就绪：get_provider_stats 和 get_time_trend 可被前端直接调用
- rollup_and_prune 定时任务已注册，应用启动后自动运行
- 下一步（Phase 30 Plan 02 或后续）：前端统计分析 Tab 接入后端接口，使用 recharts 渲染趋势图

## Self-Check: PASSED

- rollup.rs: FOUND
- commands/traffic.rs: FOUND
- 30-01-SUMMARY.md: FOUND
- Commit 961ced2 (Task 1): FOUND
- Commit 32184f9 (Task 2): FOUND

---
*Phase: 30-stats-rollup*
*Completed: 2026-03-18*
