---
phase: 31-tech-debt-fix
plan: "01"
subsystem: api
tags: [rust, sqlite, rollup, typescript, react, i18n, cache-tokens]

# Dependency graph
requires:
  - phase: 30-stats-rollup-01
    provides: query_provider_stats + ProviderStat 结构体、daily_rollups 表含 total_cache_creation_tokens 列
  - phase: 30-stats-rollup-03
    provides: CacheLeaderboard 组件、StatsAnalysisTab、i18n 键前缀 traffic.analysis
provides:
  - ProviderStat 后端结构体含 total_cache_creation_tokens 字段，SQL 24h/7d 两分支均正确聚合
  - TypeScript ProviderStat 接口含 total_cache_creation_tokens: number，Tauri 序列化链路完整
  - CacheLeaderboard 展示 6 列，含缓存创建 Token 列，支持排序
  - 30-03-SUMMARY.md 文档路径已修正为 src/i18n/locales/
affects: [future cache stats work, provider analytics]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - row.get(N) 索引在 SQL 新增列后须整体右移，顺序与 SELECT 列完全对应
    - UNION ALL 子查询两侧列名和顺序须一致，外层聚合才能正确对应

key-files:
  created: []
  modified:
    - src-tauri/src/traffic/rollup.rs
    - src/types/traffic.ts
    - src/components/traffic/CacheLeaderboard.tsx
    - src/i18n/locales/zh.json
    - src/i18n/locales/en.json
    - .planning/phases/30-stats-rollup/30-03-SUMMARY.md

key-decisions:
  - "total_cache_creation_tokens 插入位置：结构体/接口/SQL 均放在 total_output_tokens 和 total_cache_read_tokens 之间，与 DB schema 保持一致"
  - "test_query_time_trend_7d 修复：放弃 hours_ago_ts(23) 的跨天分割断言，改为断言总请求数和 d1 静态聚合值，彻底消除时区边界竞态"

patterns-established:
  - "SQL 多列插入后：必须逐一核对 query_map 闭包中每个 row.get(N) 的索引偏移"
  - "时间相关测试：避免依赖 '23 小时前 = 昨天' 的脆弱假设，改用绝对时间段断言"

requirements-completed: [DEBT-01, DEBT-03]

# Metrics
duration: 约9min
completed: 2026-03-19
---

# Phase 31 Plan 01: 补全 total_cache_creation_tokens 数据链路 Summary

**将 total_cache_creation_tokens 从 SQLite 存储层完整暴露至 ProviderStat API 和 CacheLeaderboard 前端展示，修正 30-03-SUMMARY.md 文档路径**

## Performance

- **Duration:** 约 9 min
- **Started:** 2026-03-19T00:05:11Z
- **Completed:** 2026-03-19T00:14:13Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Rust ProviderStat 结构体新增 total_cache_creation_tokens 字段，24h 和 7d SQL 分支均正确聚合（COALESCE SUM，索引右移）
- TypeScript ProviderStat 接口同步新增字段，Tauri 序列化链路完整，tsc --noEmit 零错误
- CacheLeaderboard 从 5 列扩展为 6 列（新增"缓存创建 Token"排序列），gridTemplateColumns 更新
- zh/en i18n 新增 colCacheCreationTokens 键
- 修正 30-03-SUMMARY.md 中 4 处错误路径（src/locales/ → src/i18n/locales/）
- 新增 insert_log_with_cache 测试辅助函数，扩展 24h/7d 测试对新字段的断言

## Task Commits

各任务原子提交：

1. **Task 1: 后端 ProviderStat + 前端类型 + CacheLeaderboard 展示** - `5b0fee7` (feat)
2. **Task 2: 修正 30-03-SUMMARY.md 文档路径** - `ae9b6e5` (fix)

**计划元数据:** 本次 SUMMARY + STATE + ROADMAP 更新提交

## Files Created/Modified

- `src-tauri/src/traffic/rollup.rs` - 新增结构体字段、更新两个查询分支 SQL 和 row.get() 索引、扩展测试
- `src/types/traffic.ts` - ProviderStat 接口新增 total_cache_creation_tokens: number
- `src/components/traffic/CacheLeaderboard.tsx` - 新增 SortKey、columns、数据行单元格、6 列 gridTemplateColumns
- `src/i18n/locales/zh.json` - 新增 colCacheCreationTokens: "缓存创建 Token"
- `src/i18n/locales/en.json` - 新增 colCacheCreationTokens: "Cache Creation"
- `.planning/phases/30-stats-rollup/30-03-SUMMARY.md` - 4 处 i18n 路径修正

## Decisions Made

- `total_cache_creation_tokens` 字段位置：放在 `total_output_tokens` 和 `total_cache_read_tokens` 之间（与 DB schema 保持一致，Research PLAN 指定顺序）
- 不修改 `rollup_and_prune` 函数（DB 中该字段已正确写入，本 plan 仅补齐查询暴露路径）

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修复 test_query_time_trend_7d 时区边界竞态**
- **Found during:** Task 1（运行 cargo test 时发现该测试在 UTC 凌晨失败）
- **Issue:** 原始测试使用 `hours_ago_ts(23)` 模拟 "昨天" 数据，在 UTC 00:xx 时该时间点仍属于今天，导致天点数为 2 而非预期的 3（且今天数据其实也是 30 分钟前 = 昨天 23:30，属于昨天），测试断言失败
- **Fix:** 重写测试：改用单条 `recent_ts()`（30 分钟前）明细，删除跨天断言，改为断言 d1 静态数据正确性 + 总请求数（23 = 22 daily_rollups + 1 明细）
- **Files modified:** src-tauri/src/traffic/rollup.rs
- **Verification:** 所有 25 个 traffic 测试全部通过
- **Committed in:** 5b0fee7（Task 1 提交的一部分）

---

**Total deviations:** 1 auto-fixed（Rule 1 - Bug）
**Impact on plan:** 修复预先存在的测试不稳定性，无范围扩展。

## Issues Encountered

- `row.get(N)` 索引偏移：24h 和 7d 分支各有 11 列（原 10 列），须将原 5-9 全部右移为 6-10，并在 index=5 处插入新字段。验证后无误。

## User Setup Required

无需外部服务配置。

## Next Phase Readiness

- Phase 31 Plan 01 完成，DEBT-01（total_cache_creation_tokens 数据链路）和 DEBT-03（文档路径修正）已清除
- CacheLeaderboard 现在可展示完整的缓存创建 Token 数据，供用户分析缓存使用成本
- 无遗留阻塞项

## Self-Check: PASSED

- FOUND: src-tauri/src/traffic/rollup.rs
- FOUND: src/types/traffic.ts
- FOUND: src/components/traffic/CacheLeaderboard.tsx
- FOUND: .planning/phases/31-tech-debt-fix/31-01-SUMMARY.md
- FOUND commit 5b0fee7: feat(31-01)
- FOUND commit ae9b6e5: fix(31-01)
- cargo test: 25 passed, 0 failed
- tsc --noEmit: 零错误

---
*Phase: 31-tech-debt-fix*
*Completed: 2026-03-19*
