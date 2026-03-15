# Phase 18: 首页布局优化 - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

首页 Provider 卡片操作直观可发现，空状态精致，代理状态指示清晰。不新增功能能力，只优化现有首页的交互体验和视觉表现。

</domain>

<decisions>
## Implementation Decisions

### 卡片操作按钮外露
- 编辑、复制、测试、删除四个操作全部外露为纯图标按钮（Pencil/Copy/Play/Trash 等 lucide 图标）
- 每个图标按钮配 Tooltip 显示操作名称
- 如空间不够降级为三个：编辑 + 测试 + 删除（复制留三点菜单）
- 「复制到」操作因有子菜单（选目标 CLI），仍保留在三点菜单或 popover 中
- 切换按钮放在操作按钮组的最左边（最靠近卡片内容一侧）
- 操作按钮始终可见，不再仅 hover 时显示
- 删除按钮默认与其他图标同色，hover 时变红色提示破坏性操作

### 卡片 hover 效果
- shadow-sm → shadow-md 阴影加深
- translateY(-1px) 微上移，模拟物理升起效果
- 边框从 border-border 变亮/加强（如 border-border → 更亮的边框色）
- 活跃卡片也有相同的 hover 微升起效果，保持其橙色边框和背景色
- transition 包含 shadow + transform + border-color，过渡时间与 Phase 21 微动效对齐

### 空状态设计
- 精致图标风格：保持 lucide 图标但放大，外层加品牌橙色淡底圆形装饰（brand-accent/10 背景 + brand-accent 图标色）
- 简洁功能型文案：标题「还没有 Provider」+ 描述「添加你的第一个 API Provider 开始使用」
- 只放「创建 Provider」按钮，不需要导入快捷入口
- 不使用自定义 SVG 插画，保持与应用整体图标风格一致

### 代理状态指示
- Tab 标签旁的代理绿点从 size-2 加大到 size-2.5，启用时加脉冲/呼吸动画（animate-pulse 或自定义 ring 扩散）
- 代理模式开关行旁加圆点指示：启用时绿色圆点，停用时灰色圆点（muted-foreground）
- 停用时灰色圆点始终显示（非隐藏），让用户明确看到「未启用」状态

### Claude's Discretion
- 具体图标选择（edit/pencil、play/zap 等的最终选定）
- 过渡动效时长（150-300ms 范围内）
- 脉冲动画的具体实现方式（animate-pulse vs 自定义 keyframes）
- 空状态图标装饰圆的具体尺寸

</decisions>

<specifics>
## Specific Ideas

- 切换按钮放最左边，符合「先选中再操作」的交互习惯
- 删除按钮 hover 变红，不在默认状态打扰视觉
- 空状态用品牌橙色淡底圆形装饰图标，与 Phase 17 建立的品牌色体系呼应
- 代理绿点脉冲动画传递「服务正在运行」的活跃感

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- ProviderCard.tsx: 已有 group hover 机制和 DropdownMenu，需重构为直接图标按钮
- EmptyState.tsx: 已有基础结构（图标+标题+描述+按钮），需增加装饰元素
- Badge 组件: 可用于活跃状态标识
- Tooltip 组件: 已有 TooltipProvider/Tooltip/TooltipTrigger/TooltipContent，可直接用于图标按钮提示
- Switch 组件: 代理开关已有，只需在旁边加圆点指示
- CSS 变量: --brand-accent, --status-active, --status-success 已在 Phase 17 定义

### Established Patterns
- lucide-react 图标库: 全应用统一使用 lucide 图标
- shadcn/ui 组件: Button(variant, size), Tooltip, Badge 等
- Tailwind CSS v4 + CSS 变量: 设计 token 通过 CSS 变量引用
- i18next 国际化: 所有用户可见文案通过 t() 翻译

### Integration Points
- ProviderCard props: onSwitch/onEdit/onCopy/onCopyTo/onTest/onDelete 回调已就绪
- useProxyStatus hook: 提供 proxyStatus.cli_statuses 代理状态数据
- ProviderTabs.tsx: Tab 标签 + 代理开关行的容器组件

</code_context>

<deferred>
## Deferred Ideas

None — 讨论未超出 Phase 18 范围

</deferred>

---

*Phase: 18-homepage-layout*
*Context gathered: 2026-03-15*
