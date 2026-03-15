---
phase: 18-homepage-layout
plan: 02
subsystem: ui
tags: [react, tailwind, i18n, brand-accent, status-success, animate-pulse]

# Dependency graph
requires:
  - phase: 17-design-foundation
    provides: CSS 变量 brand-accent、status-success，Tailwind 工具类
provides:
  - EmptyState 品牌橙色圆形装饰（bg-brand-accent/10 + text-brand-accent）
  - 空状态文案优化（中英双语）
  - Tab 标签旁代理绿点加大（size-2.5）+ 脉冲动画（animate-pulse）
  - 代理开关旁始终可见的状态圆点（启用绿/停用灰）
affects: [19-provider-card, 21-micro-animations]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "brand-accent 用于空状态圆形装饰：bg-brand-accent/10 背景 + text-brand-accent 图标色"
    - "代理活跃状态圆点：启用 bg-status-success，停用 bg-muted-foreground/40，始终渲染"
    - "animate-pulse 表达「服务运行中」的动态感知"

key-files:
  created: []
  modified:
    - src/components/provider/EmptyState.tsx
    - src/components/provider/ProviderTabs.tsx
    - src/i18n/locales/zh.json
    - src/i18n/locales/en.json

key-decisions:
  - "状态圆点提取到 Switch 条件分支外，确保 disabled 和正常两种状态下均可见"
  - "脉冲动画（animate-pulse）仅加在 Tab 标签绿点上，传达「活跃」语义；开关旁圆点不加脉冲以区分职责"

patterns-established:
  - "品牌图标装饰：size-20 rounded-full + bg-brand-accent/10 圆形容器，内放 size-10 图标"

requirements-completed: [HOME-03, HOME-04]

# Metrics
duration: 10min
completed: 2026-03-15
---

# Phase 18 Plan 02：首页空状态与代理指示视觉优化 Summary

**EmptyState 新增品牌橙色圆形图标装饰，空状态文案更新，Tab 代理绿点加大并加脉冲，开关旁添加始终可见的启用/停用状态圆点**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-15T07:44:00Z
- **Completed:** 2026-03-15T07:53:58Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- EmptyState 图标外层包裹 `size-20 rounded-full bg-brand-accent/10` 圆形装饰，图标色改为 `text-brand-accent`，空洞感消除
- 中英文空状态文案更新为更友好的引导语（「还没有 Provider」/「添加你的第一个 API Provider 开始使用」）
- Tab 标签代理绿点从 `size-2` 加大到 `size-2.5`，并添加 `animate-pulse` 呼吸动画
- 代理开关行右侧追加状态圆点：启用时 `bg-status-success`，停用时 `bg-muted-foreground/40`，始终可见

## Task Commits

每个任务原子提交：

1. **Task 1: 空状态页面精致化** - `5dc34c6` (feat)
2. **Task 2: 代理状态指示优化** - `04b8988` (feat)

## Files Created/Modified

- `src/components/provider/EmptyState.tsx` — 添加品牌橙色圆形装饰，更新图标颜色
- `src/components/provider/ProviderTabs.tsx` — 代理绿点加大 + 脉冲，开关旁添加状态圆点
- `src/i18n/locales/zh.json` — 更新 empty.title / empty.description
- `src/i18n/locales/en.json` — 更新 empty.title / empty.description

## Decisions Made

- 状态圆点放在 Switch 条件分支（disabled TooltipProvider / 正常 Switch）之外，确保两种状态都能渲染，避免被条件遮蔽
- Tab 绿点加 animate-pulse，开关旁圆点不加脉冲：前者表达「服务活跃」动态感，后者仅做静态指示

## Deviations from Plan

None — 计划按原样执行完毕。

## Issues Encountered

None。

## User Setup Required

None — 无需外部服务配置。

## Next Phase Readiness

- 空状态与代理状态视觉已优化，Phase 19（Provider 卡片样式）可继续叠加
- brand-accent 和 status-success CSS 变量已在本 Phase 中被实际引用，验证 Phase 17 变量体系可用性

---
*Phase: 18-homepage-layout*
*Completed: 2026-03-15*
