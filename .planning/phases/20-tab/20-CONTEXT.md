# Phase 20: 设置页 Tab 化 - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

将设置页从单列滚动长页重构为三 Tab 分组布局（通用/高级/关于），内容按功能归类。不新增设置项，只重组现有内容的布局结构。

</domain>

<decisions>
## Implementation Decisions

### 字段归属分组
- 通用 Tab：语言选择（目前只有一个设置项，简洁明了，未来主题切换等自然归入）
- 高级 Tab：代理模式开关 + 测试配置（timeout/test model）+ 导入 CLI 配置
- 关于 Tab：应用 Logo + 版本号 + 更新检查 + GitHub Releases 链接

### Tab 视觉风格
- 使用 tabs.tsx 的 variant="line" 下划线风格（类似 GitHub Settings）
- Tab 栏居左对齐、自然宽度，不平分不占满
- Tab 切换无过渡动效，内容直接替换（Phase 21 微动效可后续追加）

### Tab 状态持久化
- 不持久化，每次打开设置页默认停留「通用」Tab
- 无需 localStorage 或后端存储

### 关于页富化
- 顶部展示应用 Logo（先用现有图标，Phase 22 新图标完成后自动替换）
- 保持现有内容：版本号 + 更新检查按钮/状态 + GitHub Releases 链接
- 不额外加应用描述文字、协议信息或开发者信息

### Claude's Discretion
- 高级 Tab 内各区块之间的分隔样式（Separator 或间距）
- Logo 在关于页的具体尺寸和布局位置
- Tab 栏与 Header 之间的间距处理
- 高级 Tab 内代理/测试/导入三个区块的排列顺序

</decisions>

<specifics>
## Specific Ideas

- Tab 风格选择 line variant 是为了与暗色主题融合更好，避免填充式背景在深色界面下显得过重
- 「通用」Tab 内容虽少但留有扩展空间（主题切换等未来功能自然归入）
- 关于页加 Logo 提升正式感，现有图标是过渡方案

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- tabs.tsx: shadcn/ui Tabs 组件已就绪，支持 variant="line"（TabsList/TabsTrigger/TabsContent）
- AboutSection.tsx: 关于区块已独立组件化，可直接放入「关于」TabsContent
- useSettings hook: settings CRUD 逻辑不变，各 Tab 共享
- useProxyStatus hook: 代理状态数据，高级 Tab 使用
- useUpdater hook: 更新检查逻辑，关于 Tab 使用
- CSS 变量: Phase 17 建立的 --brand-accent、间距/圆角变量可复用

### Established Patterns
- shadcn/ui 组件: Tabs, Button, Input, Label, Select, Switch, Separator
- i18next: 所有用户可见文案通过 t() 翻译，需为 Tab 标签名添加翻译 key
- Tailwind CSS v4 + CSS 变量: 设计 token 通过 CSS 变量引用

### Integration Points
- SettingsPage props: onBack 和 onShowImport 接口不变
- AppShell.tsx: 设置页入口路由不变，只是内部布局重构
- i18n locales: 需添加 Tab 标签名翻译 key（settings.tabGeneral/tabAdvanced/tabAbout）

</code_context>

<deferred>
## Deferred Ideas

None -- 讨论未超出 Phase 20 范围

</deferred>

---

*Phase: 20-tab*
*Context gathered: 2026-03-15*
