# Phase 14: 数据模型 + 转换核心 - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Provider 数据模型扩展（upstream_model、upstream_model_map、ProtocolType 三变体）+ 三个纯函数转换模块（请求转换 anthropic_to_openai、非流式响应转换 openai_to_anthropic、流式 SSE 状态机 create_anthropic_sse_stream），可独立于 handler 单元测试。

</domain>

<decisions>
## Implementation Decisions

### 模型映射字段设计
- 新增 `upstream_model: Option<String>` — 代理转换使用的默认目标模型名
- 新增 `upstream_model_map: Option<HashMap<String, String>>` — 任意个模型名映射对（源模型名 → 目标模型名）
- 两个新字段与现有 `model`/`model_config` **完全独立** — 现有字段是 CLI 配置文件用（surgical patch），新字段仅代理转换用
- 命名用 `upstream_` 前缀，明确区分用途
- 映射优先级：精确匹配 → upstream_model → 保留原模型名

### ProtocolType 扩展为三变体
- `Anthropic` — 不变
- `OpenAiChatCompletions` — 原 `OpenAiCompatible`，Chat Completions API
- `OpenAiResponses` — 新增，Responses API
- 旧 JSON 中的 `"open_ai_compatible"` 通过 serde alias 向前兼容，反序列化为 `OpenAiChatCompletions`
- **不再需要**单独的 `upstream_api_format` 字段 — protocol_type 已完全描述上游协议

### base_url 路径策略
- 放宽 base_url 校验，允许包含路径（如 `https://openrouter.ai/api/v1`）
- 端点重写以 `/v1` 为锚点智能去重：
  - 无路径：补全完整端点（`/v1/chat/completions`）
  - 路径含 `/v1`：替换 `/v1` 之后的部分为目标端点后缀（`/chat/completions` 或 `/responses`）
  - 路径含 `/v1/responses`：视 ProtocolType 替换为正确端点
- 修改 `normalize_origin_base_url()` 或新增变体函数

### 转换降级策略
- 参考 cc-switch 混合策略：
  - 已知不兼容内容（thinking blocks, BatchTool）→ 静默丢弃
  - 可能兼容的内容（cache_control, 未知请求字段）→ 透传（OpenAI Provider 通常忽略）
  - 不兼容 JSON Schema 字段（`format: "uri"` 等）→ 递归清理移除
- Claude 有裁量权根据具体内容类型决定丢弃还是透传

### Claude's Discretion
- 转换函数内部结构设计（模块拆分、辅助函数命名）
- 流式 SSE 状态机的具体状态定义和转换逻辑
- 单元测试用例的组织方式
- bytes / futures crate 的具体使用方式

</decisions>

<specifics>
## Specific Ideas

- cc-switch 的 `transform.rs`（775行）和 `streaming.rs`（744行）是完整可参考的蓝图，但不受其局限
- 调研结论：零新 crate，`bytes` 和 `futures` 仅需显式声明（已是传递依赖）
- 流式工具调用的 Deferred Start pending buffer 是核心复杂点（cc-switch streaming.rs 第 280-347 行）
- 所有转换函数必须是纯函数/流适配器，不依赖 AppHandle 或 ProxyState

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ProtocolType` enum（provider.rs:79）— 已有 Anthropic/OpenAiCompatible，需扩展
- `Provider` struct（provider.rs:97）— 需新增 upstream_model/upstream_model_map 字段
- `UpstreamTarget` struct（state.rs:8）— 需扩展携带模型映射信息
- `serde_json::Value` 已在 handler.rs 中使用 — 转换函数使用相同类型

### Established Patterns
- `#[serde(default)]` + `#[serde(skip_serializing_if)]` — 向前兼容新字段（参考 model_config, notes）
- `_in` 内部函数变体 — 测试隔离无需 mock 文件系统路径
- 纯函数 + `#[cfg(test)] mod tests` — 全项目一致的测试模式（221 tests）

### Integration Points
- `proxy/` 子目录新增 `translate/` 模块（request.rs, response.rs, stream.rs, mod.rs）
- `provider.rs` — ProtocolType 扩展和 Provider 新字段
- `proxy/state.rs` — UpstreamTarget 扩展
- Phase 15（handler 集成）将消费这些纯函数

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 14-data-model-translate-core*
*Context gathered: 2026-03-14*
