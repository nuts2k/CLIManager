# Phase 21: 微动效与 Header 提升 - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

可交互元素加入流畅的过渡动效，Header 视觉品牌感提升。不新增功能，只增强现有 UI 的视觉反馈和品牌表现力。

</domain>

<decisions>
## Implementation Decisions

### Header 品牌视觉
- Header 左侧加应用 Logo 图标（先用现有图标，Phase 22 新图标后自动替换）
- Header 背景色改为比内容区略深的颜色（如 bg-card 或自定义 --header-bg），配合底部 border 形成层次感
- Header 高度保持 h-12 (48px) 不变
- 应用名 "CLIManager" 字体加粗（font-bold），"CLI" 部分用品牌橙色 --brand-accent，"Manager" 保持白色

### 状态切换动效
- Provider 激活切换：卡片橙色边框和背景色淡入淡出过渡（确保 border-color 和 bg 在 transition 属性中）
- 代理模式切换：仅依赖 Switch 组件自带的圆点位移 + 背景色变化动效，不加额外视觉反馈
- Switch 组件已有 transition-all，足够清晰

### 页面视图切换
- 首页 ↔ 设置页之间加淡入淡出过渡（~150ms opacity transition）
- 旧视图淡出 + 新视图淡入，避免内容突变感

### 按钮/开关微动效
- 保持 shadcn/ui 按钮默认的 transition-all 效果，不额外自定义
- 卡片操作图标按钮保持现有 ghost 变体 hover 效果
- 删除按钮 hover 变红已在 Phase 18 实现，保持不变

### Claude's Discretion
- 淡入淡出的具体实现方式（CSS transition vs React 动画库）
- Header --header-bg 的具体色值（在暗色主题下的微调）
- Logo 图标在 Header 中的具体尺寸（建议 20-24px 范围内）
- 全局 transition duration 值的统一标准（150-300ms 范围内）
- 缓动函数选择（ease-out vs ease-in-out）

</decisions>

<specifics>
## Specific Ideas

- "CLI" 品牌色 + "Manager" 白色的双色处理，让应用名本身成为视觉标识
- Header 微深背景色不需要太强的对比度，只要能感知到层次区分即可
- 页面切换淡入淡出应该足够快（~150ms），让用户感觉流畅而非等待

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- Header.tsx: 极简结构（h1 + Button），改造成本低
- ProviderCard.tsx: 已有 transition-all duration-200，hover 效果 Phase 18 已实现
- Switch 组件: 已有 transition-all（圆点位移 + 背景色变化）
- Button 组件: 已有 transition-all，各变体有 hover 样式
- CSS 变量: --brand-accent (#F97316 / oklch), --status-active, bg-card 等已定义
- public/icon.png: Phase 20 已复制到 public 目录（关于页 Logo 用），Header 可复用

### Established Patterns
- Tailwind CSS v4 + CSS 变量: 设计 token 通过 CSS 变量引用
- transition-all duration-200: ProviderCard 已建立的过渡时间标准
- i18next: 如果 Header 文字需要国际化
- lucide-react: 图标库统一使用

### Integration Points
- AppShell.tsx: view state 切换（"main" | "settings"），淡入淡出需在这里实现
- Header props: onNavigate 回调不变，只改视觉样式
- ProviderCard className: 已有 transition-all，需确认 border-color 和 bg 在 transition 中

</code_context>

<deferred>
## Deferred Ideas

None -- 讨论未超出 Phase 21 范围

</deferred>

---

*Phase: 21-header*
*Context gathered: 2026-03-15*
