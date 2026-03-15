---
gsd_state_version: 1.0
milestone: v2.3
milestone_name: 前端调整及美化
status: planning
stopped_at: Completed 22-02-PLAN.md (托盘图标生成)
last_updated: "2026-03-15T09:56:28.437Z"
last_activity: 2026-03-15 — v2.3 roadmap created (Phases 17-22)
progress:
  total_phases: 6
  completed_phases: 6
  total_plans: 9
  completed_plans: 9
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-15)

**Core value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容
**Current focus:** Phase 17 — 设计基础（CSS 变量配色 + 间距/圆角规范）

## Current Position

Phase: 17 of 22 (设计基础)
Plan: — (not yet planned)
Status: Ready to plan
Last activity: 2026-03-15 — v2.3 roadmap created (Phases 17-22)

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Historical Velocity:**
- v1.0: 12 plans, ~1.12 hours total (avg 6min/plan)
- v1.1: 3 plans, ~25 min total (avg 8min/plan)
- v2.0: 7 plans, ~35 min total (avg 5min/plan)
- v2.1: 5 plans, ~39 min total (avg 8min/plan)
- v2.2: 10 plans, ~57 min total (avg 6min/plan)
- Combined: 37 plans across 5 milestones

## Accumulated Context

### Decisions

（v2.2 决策已归档至 .planning/milestones/v2.2-ROADMAP.md）

v2.3 设计决策（roadmap 阶段）：
- Phase 17 先行：CSS 变量体系是所有视觉工作的基础，其他 Phase 依赖它
- Phase 21 依赖 Phase 18：微动效需要卡片结构稳定后才能叠加动效
- ICON 独立为最后一个 Phase：纯设计资产，不阻塞其他前端工作
- [Phase 17-design-foundation]: 品牌橙色 #F97316 映射为 oklch(0.702 0.183 56.518)，通过 --brand-accent CSS 变量引用，status-active 与 brand-accent 取相同值保持品牌一致性
- [Phase 17-design-foundation]: 语义色命名原则：status-success/warning/active 而非具体色相名，未来换色只需修改 :root 定义
- [Phase 17-design-foundation]: Card 组件从 rounded-xl 统一为 rounded-lg，使卡片圆角与对话框规范一致
- [Phase 17-design-foundation]: 间距阶梯 CSS 变量（--space-xs 至 --space-2xl）作文档锚点，业务组件仍直接用 Tailwind 工具类
- [Phase 18-homepage-layout]: ProviderCard 四个操作从三点菜单外露为始终可见图标按钮，使用 ArrowRightLeft/Pencil/Copy/Play/Trash2 图标，「复制到」因子菜单保留在 MoreVertical 菜单
- [Phase 18-homepage-layout]: 状态圆点提取到 Switch 条件分支外，确保 disabled 和正常两种状态下均可见
- [Phase 18-homepage-layout]: Tab 绿点加 animate-pulse 传达服务活跃动态感，开关旁圆点仅做静态指示不加脉冲
- [Phase 18-homepage-layout]: TooltipProvider 在按钮组外层包裹一次（delayDuration=300ms），卡片 hover 升起效果：shadow-sm→shadow-md + translateY(-2px)
- [Phase 19-01]: 移除 Collapsible 改三分区平铺，upstreamModel 验证失败不再需要 setAdvancedOpen
- [Phase 19-01]: Input 组件已内置 aria-invalid:border-destructive，ProviderDialog 无需额外条件 className
- [Phase 20-tab]: Tab 栏使用 variant=line 下划线风格，居左对齐，defaultValue=general 确保每次打开停留通用 Tab
- [Phase 21-header]: --header-bg 色值 0.160 0.02 275 介于 background/card 之间，AppShell 改为始终渲染两视图用 opacity 实现过渡
- [Phase 22-app-icon]: SVG 设计：1024x1024 画布，深色背景 #111827，中心橙圆 r=88+内圈镂空 r=44，5个有机分布外围节点，连接线 10px；qlmanage 渲染 SVG→PNG；保留 app-icon.svg 供 Plan 02 托盘图标派生
- [Phase 22-app-icon]: 托盘图标用 Python Pillow 手工绘制（qlmanage 渲染透明 SVG 为白底，不符合 template 要求）
- [Phase 22-app-icon]: tray-icon-template.png 和 @2x 均保持 44x44，与 lib.rs include_bytes! 完全兼容

### Pending Todos

None.

### Blockers/Concerns

- UX-01 端口冲突检测依赖脆弱的中文子串匹配（v2.0 遗留，低优先级）

## Session Continuity

Last session: 2026-03-15T09:52:40.143Z
Stopped at: Completed 22-02-PLAN.md (托盘图标生成)
Resume file: None
