---
phase: 30-stats-rollup
plan: "02"
subsystem: ui
tags: [react, typescript, tauri-invoke, i18n, tabs, leaderboard, traffic-stats]

requires:
  - phase: 30-stats-rollup/30-01
    provides: get_provider_stats / get_time_trend Tauri commands，ProviderStat/TimeStat 后端数据结构
  - phase: 29-traffic-ui
    provides: TrafficTable div-based grid 布局模式，formatters.ts，TrafficEmptyState 风格

provides:
  - TrafficPage 重构为 Tab 容器（实时日志 + 统计分析，默认实时日志）
  - StatsAnalysisTab：24h/7d Segment 切换 + 排行榜 + 趋势图占位
  - ProviderLeaderboard：供应商排行榜（可排序 6 列）
  - CacheLeaderboard：缓存命中率排行榜（可排序 5 列）
  - useTrafficStats hook：管理 timeRange 状态，自动拉取聚合数据
  - ProviderStat / TimeStat / TimeRange 类型定义
  - getProviderStats / getTimeTrend Tauri invoke 封装
  - i18n 中英文统计分析相关 key（tabLogs/tabStats/analysis.*）

affects: [30-stats-rollup/30-03（趋势图），前端统计分析 Tab 后续迭代]

tech-stack:
  added: []
  patterns:
    - "div-based grid 排行榜表格（与 TrafficTable 保持一致，避免 tr 内嵌套 div 问题）"
    - "useEffect + cancelled flag 模式防止 race condition（timeRange 切换时取消上一次请求）"
    - "group-hover 跨列 hover 高亮（div contents 内子元素通过 group/group-hover 联动）"

key-files:
  created:
    - src/hooks/useTrafficStats.ts
    - src/components/traffic/StatsAnalysisTab.tsx
    - src/components/traffic/ProviderLeaderboard.tsx
    - src/components/traffic/CacheLeaderboard.tsx
  modified:
    - src/components/traffic/TrafficPage.tsx
    - src/types/traffic.ts
    - src/lib/tauri.ts
    - src/i18n/locales/zh.json
    - src/i18n/locales/en.json

key-decisions:
  - "默认 Tab 为 logs（实时日志），用户决策：不打断现有工作流"
  - "5 张统计摘要卡片仅在实时日志 Tab 显示（用户决策：聚合统计在统计分析 Tab 展示）"
  - "useEffect + cancelled flag 防止 timeRange 切换时旧请求覆盖新结果（race condition 防护）"
  - "平均 TPS 公式：SUM(output_tokens) / (SUM(duration_ms) - SUM(ttfb_ms)) * 1000，与 CONTEXT 公式一致"
  - "命中率分母为 0 时显示 '--' 而非 0%（避免误导）"

patterns-established:
  - "div-based grid 排行榜表格：grid + contents + group-hover 实现跨列行 hover"
  - "SortKey union type + handleSort 函数模式：同列切换升降序，不同列默认降序"

requirements-completed: [STAT-02, STAT-03]

duration: 4min
completed: "2026-03-18"
---

# Phase 30 Plan 02: 前端 Tab 重构 + 排行榜表格 Summary

**TrafficPage 重构为双 Tab 容器，统计分析 Tab 展示 24h/7d 联动的供应商排行榜和缓存命中率排行榜，两个排行榜均支持点击表头多列排序**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-18T13:55:19Z
- **Completed:** 2026-03-18T13:59:23Z
- **Tasks:** 3（Task 1 + Task 2a + Task 2b）
- **Files modified:** 9

## Accomplishments

- TrafficPage 重构为 Tabs 容器（line 变体，与 SettingsPage 视觉一致），默认显示实时日志 Tab
- 创建 useTrafficStats hook：管理 timeRange 状态，useEffect + cancelled flag 防止 race condition
- ProviderLeaderboard 6 列可排序表格（供应商/请求数/Token/成功率/平均TTFB/平均TPS）
- CacheLeaderboard 5 列可排序表格（供应商/缓存触发数/命中率/缓存读取Token/总Token）
- 所有 UI 文字通过 i18n（中英文，traffic.analysis.* key 体系）
- TypeScript 编译和前端构建均通过

## Task Commits

每个任务均原子提交：

1. **Task 1: 类型定义 + invoke 封装 + 聚合数据 hook + i18n key** - `c546d14` (feat)
2. **Task 2a: TrafficPage Tab 重构 + StatsAnalysisTab 面板** - `5195d1e` (feat)
3. **Task 2b: ProviderLeaderboard + CacheLeaderboard 排行榜组件** - `c18c947` (feat)

## Files Created/Modified

- `src/hooks/useTrafficStats.ts` - 聚合数据 hook，管理 timeRange 状态，自动拉取 ProviderStat + TimeStat
- `src/components/traffic/StatsAnalysisTab.tsx` - 统计分析 Tab 主面板（Segment + 排行榜 + 趋势图占位）
- `src/components/traffic/ProviderLeaderboard.tsx` - 供应商排行榜表格（6 列可排序）
- `src/components/traffic/CacheLeaderboard.tsx` - 缓存命中率排行榜表格（5 列可排序）
- `src/components/traffic/TrafficPage.tsx` - 重构为 Tab 容器（实时日志 Tab + 统计分析 Tab）
- `src/types/traffic.ts` - 新增 ProviderStat / TimeStat 接口和 TimeRange 类型
- `src/lib/tauri.ts` - 新增 getProviderStats / getTimeTrend invoke 封装
- `src/i18n/locales/zh.json` - 新增 tabLogs/tabStats/analysis.* key
- `src/i18n/locales/en.json` - 新增 tabLogs/tabStats/analysis.* key（英文翻译）

## Decisions Made

- Tab 默认值为 "logs"（实时日志优先），不打断现有用户工作流
- 统计摘要卡片（TrafficStatsBar）仅在实时日志 Tab 显示，与聚合统计分离
- useEffect cancelled flag 模式防止 timeRange 切换时的 race condition
- 平均 TPS 使用精确公式：output_tokens / ((duration_ms - ttfb_ms) / 1000)
- 命中率分母为 0 时显示 "--"，避免 0% 误导用户

## Deviations from Plan

无 — 计划完全按规范执行。

## Issues Encountered

无。

## User Setup Required

无 — 不需要任何外部服务配置。

## Next Phase Readiness

- 前端统计分析 Tab 框架已就绪，排行榜组件可正常展示聚合数据
- 趋势图占位已预留（"趋势图将在 Plan 03 中实现"），Plan 03 可直接接入 timeTrend 数据
- useTrafficStats hook 已同时拉取 timeTrend 数据，Plan 03 只需接入即可

## Self-Check: PASSED
