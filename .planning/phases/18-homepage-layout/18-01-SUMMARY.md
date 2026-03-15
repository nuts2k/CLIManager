---
phase: 18-homepage-layout
plan: "01"
subsystem: ui
tags: [react, tailwind, lucide-react, tooltip, radix-ui, dropdown-menu]

# Dependency graph
requires:
  - phase: 17-design-foundation
    provides: CSS 变量（status-active、border、accent）和 rounded-lg 圆角规范
provides:
  - ProviderCard 操作图标始终外露（编辑/复制/测试/删除/切换）
  - 卡片 hover 升起动效（shadow-md + translateY(-2px) + 边框加强）
  - 「复制到」保留在 MoreVertical 三点菜单（子菜单场景）
affects:
  - 21-micro-animation
  - 任何复用 ProviderCard 组件的 phase

# Tech tracking
tech-stack:
  added: []
  patterns:
    - TooltipProvider 在按钮组外层包裹一次，避免重复嵌套
    - 图标按钮使用 variant="ghost" size="icon-sm"，hover:text-destructive 实现危险操作变色
    - 有子菜单的操作保留在 DropdownMenu，简单操作外露为图标按钮

key-files:
  created: []
  modified:
    - src/components/provider/ProviderCard.tsx

key-decisions:
  - "切换按钮使用 ArrowRightLeft 图标（lucide-react），比 ToggleRight 更直观表达双向切换"
  - "复制到保留在 MoreVertical 菜单：有子菜单的操作在平铺图标区域无法优雅呈现"
  - "otherClis.length === 0 时不渲染三点菜单，避免出现空菜单"
  - "TooltipProvider 包裹整个操作区域（delayDuration=300ms），而非每个按钮单独包裹"

patterns-established:
  - "图标按钮组：TooltipProvider > Tooltip > TooltipTrigger(asChild) + Button + TooltipContent"
  - "卡片 hover 升起：shadow-sm(默认) → shadow-md(hover)，hover:-translate-y-0.5，transition-all duration-200"

requirements-completed: [HOME-01, HOME-02]

# Metrics
duration: 8min
completed: 2026-03-15
---

# Phase 18 Plan 01: ProviderCard 操作外露与 hover 升起效果 Summary

**ProviderCard 四个操作（编辑/复制/测试/删除）从三点菜单外露为始终可见的图标按钮，配合 Tooltip 与 hover:text-destructive，并为卡片添加 shadow-md + translateY 升起过渡效果**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-15T07:52:41Z
- **Completed:** 2026-03-15T08:00:30Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- 编辑（Pencil）、复制（Copy）、测试（Play）、删除（Trash2）四个图标按钮始终可见，无需展开菜单
- 切换按钮（ArrowRightLeft）在最左侧，仅非活跃卡片显示
- 每个按钮有 Tooltip，删除按钮 hover 时变红（hover:text-destructive）
- 「复制到」因含子菜单保留在 MoreVertical 三点菜单，无其他 CLI 时完全不渲染
- 卡片添加 shadow-sm 默认阴影，hover 时升起为 shadow-md + translateY(-2px)，活跃卡片 hover 时橙色边框加强

## Task Commits

每个任务原子提交：

1. **Task 1: 卡片操作按钮外露 + hover 升起效果** - `2de2601` (feat)

**Plan 元数据:** (待 final commit)

## Files Created/Modified

- `src/components/provider/ProviderCard.tsx` - 重构：操作图标外露 + hover 升起效果，引入 Tooltip 四件套和 lucide 图标

## Decisions Made

- 使用 ArrowRightLeft 图标表示切换（比 ToggleRight 更语义化，表达双向切换）
- TooltipProvider 在按钮组外层包裹一次（delayDuration=300ms），避免每个 Tooltip 重复嵌套
- 无其他 CLI 时（otherClis.length === 0）完全不渲染 MoreVertical 菜单，避免出现空 DropdownMenu

## Deviations from Plan

无 - 按计划精确执行，无偏差。

## Issues Encountered

无。构建一次通过，无 TypeScript 错误。

## User Setup Required

无 - 无需外部服务配置。

## Next Phase Readiness

- ProviderCard 卡片结构已稳定（操作区域、hover 效果就位）
- Phase 21 微动效可在此基础上叠加更精细的动画参数
- 首页交互效率已提升：用户无需展开菜单即可直接操作 Provider

---
*Phase: 18-homepage-layout*
*Completed: 2026-03-15*
