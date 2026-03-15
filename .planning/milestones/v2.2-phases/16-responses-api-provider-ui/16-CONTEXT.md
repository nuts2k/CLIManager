# Phase 16: Responses API + Provider UI - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

两条并行开发路线：(A) Responses API 转换层——将 Anthropic Messages API 请求转换为 OpenAI Responses API 格式，并将上游响应（非流式和流式）转换回 Anthropic 格式；(B) Provider 编辑 UI 模型映射配置——在 ProviderDialog 中新增默认模型和模型映射对的可视化编辑。

</domain>

<decisions>
## Implementation Decisions

### 协议类型 UI 呈现
- 三个平级选项：Anthropic / OpenAI Chat Completions / OpenAI Responses
- 直接使用技术名称，中英文相同（无需翻译差异）
- 旧 `open_ai_compatible` 值前端加载时自动映射为 `open_ai_chat_completions`（Rust serde alias 已处理反序列化，前端 TypeScript 类型同步更新）
- 仅选择 OpenAI 类型（Chat Completions 或 Responses）时显示模型映射相关字段，Anthropic 时隐藏

### 模型映射 UI 交互
- 放在现有 Collapsible 高级设置区域内，protocol_type 下方
- 默认目标模型：单独一个输入框，placeholder 示例如 "gpt-4o"
- 模型映射对：动态行列表，每行两个输入框（源模型名 → 目标模型名）+ 删除按钮，底部"+ 添加映射"按钮
- 源模型名输入框 placeholder 示例如 "claude-sonnet-4-20250514"，目标模型名输入框 placeholder 示例如 "gpt-4o"
- 保存反馈复用现有 Provider 保存 toast，无需额外提示
- 映射数据随 Provider 一起保存，通过现有 update_provider 命令传递到 Rust 后端

### Responses API 转换层
- 独立模块：新建 responses_request.rs / responses_response.rs / responses_stream.rs，与 Chat Completions 模块并行，可共享工具函数但不强耦合
- 端点重写：`/v1/messages` → `/v1/responses`
- 降级策略沿用 Phase 14：已知不兼容（thinking blocks, BatchTool）静默丢弃，可能兼容（cache_control）透传，JSON Schema 不兼容字段递归清理
- handler.rs 中 OpenAiResponses 从现有透传路径拆出，新增独立转换分支（与 OpenAiChatCompletions 分支并列）

### Claude's Discretion
- Responses API 请求/响应的具体字段映射（根据 OpenAI Responses API 文档）
- Responses API 流式事件的具体状态机设计
- 模块间共享工具函数的抽取方式
- 前端 TypeScript 类型更新细节
- Tauri 命令参数传递方式（upstream_model / upstream_model_map 如何通过现有 update/create provider 命令传入）

</decisions>

<specifics>
## Specific Ideas

- handler.rs 当前第 126 行 `ProtocolType::Anthropic | ProtocolType::OpenAiResponses` 需要拆开，OpenAiResponses 走独立转换分支
- handler.rs 第 230 行 `_ =>` 透传分支同理需要拆开
- ProviderDialog.tsx 的 Select 组件（第 261-278 行）需要增加第三个选项
- provider.ts TypeScript 类型 `ProtocolType` 需要新增 `"open_ai_chat_completions"` 和 `"open_ai_responses"` 变体
- Provider struct 的 `upstream_model` 和 `upstream_model_map` 字段已存在于 Rust 端，前端 Provider 接口和 ProviderFormData 需要同步扩展

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `translate::request::anthropic_to_openai()`: Chat Completions 请求转换纯函数——Responses API 版本可参考结构
- `translate::request::build_proxy_endpoint_url()`: 端点重写——可直接复用，传入 "/responses"
- `translate::response::openai_to_anthropic()`: 非流式响应转换——Responses API 版本需新实现
- `translate::stream::create_anthropic_sse_stream()`: 流式转换——Responses API 流格式不同，需新实现
- `apply_upstream_model_mapping()` (handler.rs:33): 模型映射——可直接复用于 Responses API 分支
- `ProviderDialog.tsx`: 现有 Provider 编辑对话框——扩展高级设置区域
- `Collapsible` 组件: 已用于高级设置折叠——模型映射在其内部
- `Select` / `Input` / `Button` / `Label`: shadcn/ui 组件库——直接可用

### Established Patterns
- handler.rs 步骤 A-J 线性流程: 新分支复用相同步骤结构
- `ProtocolType` match 分支: 凭据注入（第 168-177 行）已处理三变体
- `#[serde(default)]` + `#[serde(skip_serializing_if)]`: 向前兼容模式
- 纯函数 + `#[cfg(test)] mod tests`: 全项目测试模式
- i18n `useTranslation()`: 所有 UI 文本通过 t() 函数

### Integration Points
- `handler.rs`: 协议路由分支需拆分 OpenAiResponses 为独立转换路径
- `translate/mod.rs`: 新增 Responses API 子模块导出
- `provider.ts`: TypeScript 类型更新（ProtocolType 三变体 + upstream_model/upstream_model_map 字段）
- `ProviderDialog.tsx`: 表单扩展（协议选项 + 模型映射 UI）
- `ProviderFormData`: 新增 upstreamModel 和 upstreamModelMap 字段
- `useProviders.ts` / Tauri 命令: 创建/更新 Provider 时传递新字段
- i18n JSON: 新增协议名称和模型映射相关翻译 key

</code_context>

<deferred>
## Deferred Ideas

- 通用设置中管理预设映射模板 + 一键应用到 Provider — 未来功能，可作为独立 phase

</deferred>

---

*Phase: 16-responses-api-provider-ui*
*Context gathered: 2026-03-14*
