---
phase: 29-traffic-page
plan: 01
subsystem: ui
tags: [react, typescript, tauri, traffic, hooks, i18n]

# Dependency graph
requires:
  - phase: 28-stream-tokens
    provides: get_recent_logs Tauri command + traffic-log 事件，SQLite traffic DB 就绪
provides:
  - TrafficLog TypeScript 接口（20 字段与后端 TrafficLogPayload 完全对应）
  - useTrafficLogs hook（双轨：command 初始拉取 + event 增量追加，500 条上限）
  - getRecentLogs Tauri command 封装
  - AppShell 三视图切换框架（main | traffic | settings）
  - Header Traffic 按钮（Activity 图标，toggle 行为）
  - traffic.* i18n 翻译 key（中英文）
affects: [29-02]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "双轨数据 hook：Tauri command 初始拉取 + event 增量追加"
    - "type=update 事件替换同 id 条目，找不到则静默忽略（避免竞态）"
    - "always-render + opacity/pointer-events 150ms 过渡（不卸载 DOM）"
    - "Header 按钮 toggle 逻辑：currentView === target ? main : target"

key-files:
  created:
    - src/types/traffic.ts
    - src/hooks/useTrafficLogs.ts
    - src/components/traffic/TrafficPage.tsx
  modified:
    - src/lib/tauri.ts
    - src/components/layout/AppShell.tsx
    - src/components/layout/Header.tsx
    - src/i18n/locales/zh.json
    - src/i18n/locales/en.json

key-decisions:
  - "useTrafficLogs type=update 事件：找不到同 id 条目时静默忽略而非新增（避免竞态，Research Pitfall 2）"
  - "占位 TrafficPage 组件直接 import（非 lazy），Plan 02 替换为完整实现"

patterns-established:
  - "Traffic 事件监听模式：listen<TrafficLog>('traffic-log') + setLogs 函数式更新"
  - "AppShell 视图过渡：showXxxView 计算 + absolute inset-0 + opacity 切换"

requirements-completed: [LOG-02, LOG-03]

# Metrics
duration: 3min
completed: 2026-03-18
---

# Phase 29 Plan 01: TrafficPage 基础设施 Summary

**TrafficLog 接口 + useTrafficLogs 双轨 hook + AppShell 三视图切换框架 + Header Traffic 按钮 + 中英文 i18n key**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-18T11:35:35Z
- **Completed:** 2026-03-18T11:38:29Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- 完整的 TrafficLog TypeScript 接口（20 字段，含 type 字段，全部 Option<T> 映射为 T | null）
- useTrafficLogs hook：启动拉取 100 条历史 + traffic-log 事件增量追加，500 条内存上限，update 替换同 id 条目
- AppShell 扩展为三视图（main / traffic / settings），traffic 视图 always-render + opacity 150ms 过渡
- Header 新增 Activity 图标 Traffic 按钮，Settings/Traffic 均为 toggle 逻辑，currentView prop 支持
- zh.json / en.json 新增 traffic.* 完整翻译 key（标题、统计、过滤、表格列、空状态）

## Task Commits

每个任务独立提交：

1. **Task 1: 类型定义 + Tauri 封装 + 数据 Hook** - `dac0f50` (feat)
2. **Task 2: 导航层扩展 + 国际化 key** - `66622bf` (feat)

## Files Created/Modified

- `src/types/traffic.ts` - TrafficLog 接口，20 字段与后端完全对应
- `src/hooks/useTrafficLogs.ts` - 双轨数据 hook，500 条上限，type=update 替换逻辑
- `src/lib/tauri.ts` - 新增 getRecentLogs() Tauri command 封装
- `src/components/layout/AppShell.tsx` - AppView 三视图、traffic 渲染块、currentView prop 传递
- `src/components/layout/Header.tsx` - Activity Traffic 按钮、currentView prop、toggle 逻辑
- `src/components/traffic/TrafficPage.tsx` - 占位组件（Plan 02 替换）
- `src/i18n/locales/zh.json` - traffic.* 中文翻译 key
- `src/i18n/locales/en.json` - traffic.* 英文翻译 key

## Decisions Made

- `useTrafficLogs` 中 `type === "update"` 事件：找不到同 id 条目时静默忽略而非新增，避免竞态问题（Research Pitfall 2 指出的 race condition 场景）
- 占位 `TrafficPage` 直接 import（非 lazy），保持代码简洁，Plan 02 会替换为完整实现

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 02 可直接开始：TrafficLog 类型、useTrafficLogs hook、AppShell 三视图框架全部就绪
- TrafficPage.tsx 占位组件已创建，Plan 02 直接替换内容即可
- i18n key 已完整预定义，UI 组件可直接使用 `t('traffic.*')`

## Self-Check: PASSED

All created files verified present. All task commits verified in git log.

---
*Phase: 29-traffic-page*
*Completed: 2026-03-18*
