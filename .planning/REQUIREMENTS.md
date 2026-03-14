# Requirements: CLIManager

**Defined:** 2026-03-14
**Core Value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容

## v2.2 Requirements

Requirements for v2.2 协议转换。让 Claude Code 通过代理模式使用 OpenAI 兼容的 Provider。

### 协议路由

- [x] **ROUT-01**: 代理模式下，Provider 协议类型为 OpenAiCompatible 时自动启用协议转换路径
- [x] **ROUT-02**: Provider 协议类型为 Anthropic 时请求直接透传，零回归

### 请求转换

- [x] **REQT-01**: 系统提示正确转换（顶层 `system` 字段 → messages 数组首条 system 消息）
- [x] **REQT-02**: 消息数组格式转换（text/tool_use/tool_result content blocks → OpenAI 对应格式）
- [x] **REQT-03**: 工具定义转换（`input_schema` → `function.parameters`，添加 `type:"function"` 包装）
- [x] **REQT-04**: 端点重写（`/v1/messages` → `/v1/chat/completions`）
- [x] **REQT-05**: 图片/多模态内容转换（base64 content block → `image_url` data URL）
- [x] **REQT-06**: JSON Schema 清理（移除 OpenAI 不兼容的 `format` 字段等）
- [x] **REQT-07**: cache_control 字段透传到 OpenAI 请求
- [x] **REQT-08**: 标准参数透传与重命名（`stop_sequences` → `stop` 等）

### 响应转换

- [x] **RESP-01**: 非流式文本响应转换（choices → content blocks）
- [x] **RESP-02**: 非流式工具调用响应转换（`tool_calls` → `tool_use` content blocks，arguments 反序列化）
- [x] **RESP-03**: stop_reason/finish_reason 映射（stop→end_turn, length→max_tokens, tool_calls→tool_use）
- [x] **RESP-04**: usage 字段映射（prompt_tokens→input_tokens, completion_tokens→output_tokens）
- [x] **RESP-05**: 错误响应（4xx/5xx）直接透传，不经转换处理

### 流式 SSE 转换

- [x] **STRM-01**: 文本 delta 事件序列转换（OpenAI content delta → Anthropic message_start/content_block_start/text_delta/content_block_stop/message_delta/message_stop 完整事件序列）
- [x] **STRM-02**: 工具调用流式转换，含 Deferred Start pending buffer（等待 id+name 就绪后才发 content_block_start）
- [x] **STRM-03**: 多工具并发流式支持（按 index 独立追踪每个工具调用状态）
- [x] **STRM-04**: 流结束事件映射（finish_reason → message_delta stop_reason + message_stop）

### 模型映射

- [x] **MODL-01**: Provider 数据模型支持存储默认目标模型名（缺省映射）
- [x] **MODL-02**: Provider 数据模型支持存储任意个模型名映射对（Anthropic 名 → 目标名）
- [x] **MODL-03**: 代理转换时按映射表自动替换请求中的模型名（精确匹配优先，无匹配时用默认模型）
- [x] **MODL-04**: Provider 编辑 UI 支持配置默认模型和映射对

### Responses API

- [x] **RAPI-01**: Provider 可配置目标 API 格式（Chat Completions 或 Responses）
- [x] **RAPI-02**: 选择 Responses 格式时，请求自动转换为 Responses API 格式
- [x] **RAPI-03**: Responses API 非流式响应正确转换回 Anthropic 格式
- [x] **RAPI-04**: Responses API 流式事件正确转换为 Anthropic SSE 格式

## Future Requirements

### 反向协议转换

- **REVT-01**: Codex 可通过代理使用 Anthropic Provider（OpenAI→Anthropic 方向转换）
- **REVT-02**: 反向流式 SSE 转换

### 高级网关功能

- **GATE-01**: OAuth 桥接
- **GATE-02**: 自动 Failover / 熔断器
- **GATE-03**: 流量监控与可视化

## Out of Scope

| Feature | Reason |
|---------|--------|
| 反向转换（OpenAI→Anthropic） | v2.2 只做 Anthropic→OpenAI 方向，反向留 v3.0 |
| 模型名称自动发现 | 用户手动配置映射，不做 Provider API 探测 |
| WebSocket 流式传输 | Claude Code 用标准 HTTP SSE，不需要 WebSocket |
| 多 choices 响应处理 | Claude Code 不发 `n > 1`，只处理 `choices[0]` |
| 请求体未知字段精确过滤 | OpenAI Provider 通常忽略未知字段，只过滤已知不兼容字段 |
| prompt_cache_key 注入 | Provider 数据模型无 cache_key 字段，初版不支持 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| MODL-01 | Phase 14 | Complete |
| MODL-02 | Phase 14 | Complete |
| REQT-01 | Phase 14 | Complete |
| REQT-02 | Phase 14 | Complete |
| REQT-03 | Phase 14 | Complete |
| REQT-04 | Phase 14 | Complete |
| REQT-05 | Phase 14 | Complete |
| REQT-06 | Phase 14 | Complete |
| REQT-07 | Phase 14 | Complete |
| REQT-08 | Phase 14 | Complete |
| RESP-01 | Phase 14 | Complete |
| RESP-02 | Phase 14 | Complete |
| RESP-03 | Phase 14 | Complete |
| RESP-04 | Phase 14 | Complete |
| RESP-05 | Phase 14 | Complete |
| STRM-01 | Phase 14 | Complete |
| STRM-02 | Phase 14 | Complete |
| STRM-03 | Phase 14 | Complete |
| STRM-04 | Phase 14 | Complete |
| ROUT-01 | Phase 15 | Complete |
| ROUT-02 | Phase 15 | Complete |
| MODL-03 | Phase 15 | Complete |
| RAPI-01 | Phase 16 | Complete |
| RAPI-02 | Phase 16 | Complete |
| RAPI-03 | Phase 16 | Complete |
| RAPI-04 | Phase 16 | Complete |
| MODL-04 | Phase 16 | Complete |

**Coverage:**
- v2.2 requirements: 27 total
- Mapped to phases: 27
- Unmapped: 0

---
*Requirements defined: 2026-03-14*
*Last updated: 2026-03-14 — traceability updated after roadmap restructure (Phases 14-16, 3-phase structure)*
