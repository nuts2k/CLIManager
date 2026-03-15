# Phase 19: Provider 编辑改进 - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Provider 编辑对话框加宽、可滚动、字段分组清晰、验证反馈明确。不新增功能字段或能力，只改进现有 ProviderDialog 的布局、分组、滚动和验证体验。

</domain>

<decisions>
## Implementation Decisions

### 分区视觉样式
- 三个分区（基础信息 / 协议设置 / 模型配置）用标题 + 分割线分隔
- 分区标题用粗体小字（text-sm font-semibold），颜色略淡（text-muted-foreground），不抢字段标签的视觉层级
- 分割线用 border-border 常规细线

### 对话框尺寸与滚动
- 对话框宽度从 sm:max-w-md (~448px) 加宽到 max-w-xl (~576px)
- DialogHeader 和 DialogFooter 固定不动，中间表单区域 max-h + overflow-y-auto 纵向滚动
- 保存/取消按钮始终可见

### 字段归属
- 基础信息：name, apiKey, baseUrl（三个必填字段）
- 协议设置：protocolType, upstreamModel, upstreamModelMap（上游模型和模型映射仅 OpenAI 协议时显示，已有条件渲染逻辑）
- 模型配置：model, testModel, haikuModel, sonnetModel, opusModel, reasoningEffort, notes（notes 放在模型配置区最末尾）

### 字段布局
- model config 的 4 个字段（haiku/sonnet/opus/reasoningEffort）保持 2x2 grid 紧凑布局
- 其余字段单列全宽
- 加宽后 2x2 grid 每个输入框更宽敞

### 折叠策略
- 去掉原有的 Collapsible 高级设置折叠，三个分区全部平铺展开
- 通过滚动处理内容过长的情况
- 创建模式和编辑模式显示完全一致的分区结构

### 字段帮助文字
- 每个字段加有意义的 placeholder 提示（如 Base URL 显示 "https://api.anthropic.com"）
- 不加额外描述文字，保持简洁
- ModelConfig 的 placeholder 全部国际化（通过 i18next t() 翻译）

### 验证反馈
- 验证错误同时显示红色小字 + 输入框红色边框（border-destructive）
- 已有 aria-invalid 属性，配合边框变色视觉关联更明确
- 保持现有验证规则不变：name/apiKey/baseUrl 非空 + OpenAI 协议时 upstreamModel 非空

### Claude's Discretion
- 具体 placeholder 文案（各字段的示例值选择）
- 滚动区域的 max-h 具体值（视窗口高度调整）
- 分割线与分区标题的具体间距微调
- 滚动条样式（是否自定义 scrollbar）

</decisions>

<specifics>
## Specific Ideas

- 分区风格类似 GitHub Settings 页面的分组方式：标题 + 分割线，轻量不累赘
- 固定 Footer 确保保存按钮始终可见，用户不需要滚到底部才能保存

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- ProviderDialog.tsx: 已有完整表单逻辑（state, validation, save），只需重构布局结构
- dialog.tsx: DialogContent 支持 className 覆盖，可直接传 max-w-xl
- Collapsible 组件: 将被移除，改为平铺分区
- Input/Label/Select/Button: shadcn/ui 组件已就绪
- zod formSchema: 现有验证逻辑保留，只改显示样式

### Established Patterns
- i18next: 所有用户可见文案通过 t() 翻译，需为新 placeholder 和分区标题添加翻译 key
- CSS 变量: Phase 17 建立的 --brand-accent, 间距/圆角变量
- aria-invalid: 已在 name/apiKey/baseUrl 输入框设置，需配合 border-destructive 样式
- showModelMapping 条件渲染: 协议设置区的上游模型/映射已有条件显示逻辑

### Integration Points
- ProviderDialog props: 接口不变（open, onOpenChange, mode, provider, cliId, onSave）
- ProviderFormData: 表单数据结构不变，只改布局
- Collapsible import: 移除后需清理 import 和 advancedOpen state

</code_context>

<deferred>
## Deferred Ideas

None -- 讨论未超出 Phase 19 范围

</deferred>

---

*Phase: 19-provider*
*Context gathered: 2026-03-15*
