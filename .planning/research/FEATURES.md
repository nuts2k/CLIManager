# Feature Research

**Domain:** 协议转换层 — Anthropic Messages API ↔ OpenAI Chat Completions API
**Researched:** 2026-03-14
**Confidence:** HIGH（cc-switch 参考实现 + Anthropic 官方文档 + OpenAI 官方文档 + 现有 CLIManager 代理架构分析）

---

## Feature Landscape

### Table Stakes（用户预期的基本功能）

这些特性是 Anthropic→OpenAI 协议转换层的基线。缺少任何一项 = Claude Code 无法正常使用 OpenAI 兼容的 Provider，功能残缺。

| Feature | 为何必须 | 复杂度 | 备注 |
|---------|---------|--------|------|
| **请求格式转换：消息数组** | Claude Code 发送 Anthropic `messages` 数组格式；OpenAI Provider 只接受 `messages` 数组但字段结构不同。core 差异：content 可能是字符串或数组，角色相同但语义处理不同 | MEDIUM | Anthropic: `{"role":"user","content": str or array}`；OpenAI: `{"role":"user","content": str or array of content parts}` |
| **请求格式转换：系统提示** | Anthropic 的 `system` 是顶层独立字段（字符串或 content block 数组）；OpenAI 没有顶层 system 字段，system prompt 必须作为 `{"role":"system","content":"..."}` 消息放到 messages 数组开头 | LOW | 支持两种 Anthropic 格式：`system: "string"` 和 `system: [{type:"text", text:"..."}]` |
| **请求格式转换：工具定义** | Anthropic tools 结构：`{name, description, input_schema}`；OpenAI tools 结构：`{type:"function", function:{name, description, parameters}}`。字段名和嵌套层次不同 | MEDIUM | `input_schema` → `parameters`；包裹在 `function` 子对象中；`type:"function"` 固定值需添加。需过滤 Anthropic 特有的 `BatchTool` 类型 |
| **请求格式转换：标准参数透传** | `max_tokens`, `temperature`, `top_p`, `stream` 等参数名相同，直接透传。`stop_sequences` → `stop` 字段名不同 | LOW | 大部分参数一一对应；只有 `stop_sequences`→`stop` 有重命名 |
| **响应格式转换：非流式** | OpenAI 响应结构：`{id, choices:[{message:{role,content,tool_calls}, finish_reason}], usage:{prompt_tokens, completion_tokens}}`；Anthropic 期望：`{id, type:"message", role:"assistant", content:[{type:"text",text:"..."}], stop_reason, usage:{input_tokens, output_tokens}}` | MEDIUM | finish_reason 映射：`"stop"→"end_turn"`，`"length"→"max_tokens"`，`"tool_calls"→"tool_use"` |
| **响应格式转换：工具调用（非流式）** | OpenAI 工具调用放在 `message.tool_calls[]`，函数参数是 JSON 字符串；Anthropic 期望 `content` 数组中的 `{type:"tool_use", id, name, input}` 块，input 是 JSON 对象（不是字符串） | MEDIUM | 需要 JSON 反序列化：`arguments`（字符串）→ `input`（对象）。`stop_reason` 必须为 `"tool_use"` |
| **流式 SSE 转换：基本文本** | OpenAI 流式格式：`data: {id, choices:[{delta:{content:"..."},finish_reason:null}]}\n\n` → `data: [DONE]`；Anthropic 流式格式：`message_start` → `content_block_start` → `content_block_delta{type:"text_delta"}` → `content_block_stop` → `message_delta` → `message_stop` 事件序列 | HIGH | 需要有状态流转换器：追踪 message_id、content index 计数器、当前块类型 |
| **流式 SSE 转换：工具调用** | OpenAI 流式工具调用：分批到来的 `delta.tool_calls[{index, id?, function:{name?, arguments?}}]`；Anthropic 期望：`content_block_start{type:"tool_use", id, name}` → `content_block_delta{type:"input_json_delta", partial_json:"..."}` 序列 | HIGH | 关键挑战：OpenAI 可能先发 arguments 再发 id/name；需要 pending buffer 等待 id+name 就绪后才能发出 content_block_start。多个并发工具调用通过 `index` 字段区分，各自独立追踪 |
| **流式 SSE 转换：finish_reason/stop_reason** | OpenAI finish_reason 在最后一个 choices chunk 中；Anthropic 用 `message_delta` 事件携带 `stop_reason`；之后发 `message_stop` | MEDIUM | `message_delta` 之后必须紧接 `message_stop`；OpenAI 的 `[DONE]` 对应 Anthropic 的 `message_stop` |
| **Usage 字段映射** | OpenAI: `{prompt_tokens, completion_tokens}`；Anthropic: `{input_tokens, output_tokens}`；流式下 usage 通常在最后一个 chunk 中 | LOW | 直接重命名两个字段；如上游返回 `prompt_tokens_details.cached_tokens`，映射为 `cache_read_input_tokens` |
| **错误响应透传** | 上游 OpenAI Provider 返回 4xx/5xx 时，错误体不需要转换，直接透传状态码和响应体；Claude Code 会读取错误信息展示给用户 | LOW | 不转换错误响应；只需确保非 200 响应不进入转换路径 |
| **协议路由：按 Provider 自动选择** | 代理层需要知道当前 Provider 是 Anthropic 协议还是 OpenAI 兼容协议，据此决定是透传还是转换 | LOW | 基于已有的 `ProtocolType` 枚举（`Anthropic` vs `OpenAiCompatible`）；OpenAiCompatible = 需要转换 |
| **端点重写：/v1/messages → /v1/chat/completions** | Claude Code 固定请求 `/v1/messages`；OpenAI Provider 接受 `/v1/chat/completions`；转换模式下必须改写目标端点 | LOW | 直接字符串替换；只在 `needs_transform` 为 true 时生效 |

---

### Differentiators（竞争优势特性）

这些特性提升兼容性和稳定性，但非转换层基本可用的前提。

| Feature | 价值主张 | 复杂度 | 备注 |
|---------|---------|--------|------|
| **图片/多模态内容转换** | Anthropic 图片格式：`{type:"image", source:{type:"base64", media_type:"image/jpeg", data:"..."}}` → OpenAI 格式：`{type:"image_url", image_url:{url:"data:image/jpeg;base64,..."}}`；让 Claude Code 的视觉功能在 OpenAI Provider 上可用 | MEDIUM | 仅支持 base64 → data URL 转换；不支持 Anthropic URL/file 类型图片（OpenAI Provider 可能支持直接 URL，但转换逻辑更复杂）|
| **工具调用多 index 并发支持** | 单次响应中 Claude Code 可能调用多个工具；OpenAI 通过 `index` 字段区分，Anthropic 通过 content block index 区分；需要独立状态机追踪每个工具 | HIGH | cc-switch 参考实现中有完整的 `ToolBlockState` 映射表（HashMap<index, ToolBlockState>），值得复用设计 |
| **cache_control 透传** | Anthropic prompt caching 的 `cache_control` 字段在转换到 OpenAI 格式后透传给支持此字段的 OpenAI 兼容 Provider（如某些中转服务）；不丢失 caching 元数据 | LOW | cc-switch 实现中已处理：system block、text block、tool 定义的 `cache_control` 字段透传到 OpenAI 请求 |
| **cache token 字段映射（响应）** | OpenAI 标准：`usage.prompt_tokens_details.cached_tokens`；某些兼容服务直接返回 `cache_read_input_tokens`；Anthropic 格式：`usage.cache_read_input_tokens` / `cache_creation_input_tokens`；正确映射让 UI 展示准确的 cache token 消耗 | LOW | 两个来源都要处理：先检查直接字段，再检查嵌套 details |
| **JSON Schema 清理（clean_schema）** | Anthropic 的 tool input_schema 可能包含 `"format":"uri"` 等 OpenAI 不支持的 JSON Schema 格式，直接透传会导致 400 错误 | LOW | 递归遍历 schema，移除已知不兼容字段（`format: "uri"` 等）；同时处理嵌套的 properties 和 items |
| **tool_choice 字段透传** | Anthropic 和 OpenAI 的 `tool_choice` 语法大致兼容（`"auto"`, `"none"`, `{type:"tool", name:...}`）；直接透传通常可工作 | LOW | 当前 cc-switch 直接透传 `tool_choice`，不做结构转换，实践中工作正常 |
| **旧版 function_call 格式兼容** | 部分 OpenAI 兼容 Provider 返回旧版 `message.function_call` 而非 `message.tool_calls`；响应转换层需处理两种格式 | LOW | cc-switch 响应转换中已处理 legacy `function_call` → Anthropic `tool_use` 的路径 |
| **content_filter finish_reason 处理** | OpenAI 的 `content_filter` finish_reason 没有 Anthropic 对应值；映射为 `end_turn` 保持流程可继续 | LOW | 已在 cc-switch stop_reason 映射表中处理 |

---

### Anti-Features（明确不做）

| Anti-Feature | 为何不做 | 替代方案 |
|--------------|---------|---------|
| **反向转换：OpenAI → Anthropic（Codex 使用 Anthropic Provider）** | PROJECT.md 明确列为 Out of Scope：v2.2 只做 Anthropic→OpenAI 方向 | v3.0 全功能网关里程碑再实现 |
| **OpenAI Responses API 格式支持** | cc-switch 有 `transform_responses.rs` 支持新 Responses API 格式，但结构更复杂。v2.2 优先完成 Chat Completions 格式，Responses API 留后续 | 当前 v2.2 只需 Chat Completions 格式 |
| **流量监控与 token 使用统计** | Out of Scope，独立里程碑（v2.3） | 代理层可记录日志，UI 展示留给 v2.3 |
| **请求体中未知字段的精确过滤** | Claude Code 可能发送一些 Anthropic 特有字段（如 `thinking`, `betas`）；OpenAI Provider 遇到未知字段通常忽略（不报错）；精确过滤增加复杂度、维护成本高，但收益小 | 只过滤已知的不兼容字段（如 `BatchTool`），其他未知字段直接透传 |
| **模型名称映射（Claude 名 → OpenAI 名）** | 用户自己在 Provider 配置中填写正确的模型名；代理层不做自动模型名映射 | 格式转换层只做结构转换，不做模型名映射（cc-switch `transform.rs` 的设计原则：注释明确说明"模型映射由上游统一处理") |
| **prompt_cache_key 注入** | cc-switch 有注入 `prompt_cache_key` 的逻辑（用于某些 OpenAI 兼容服务的 cache routing）；CLIManager 的 Provider 数据模型尚无 `cache_key` 字段，加入会扩大本里程碑范围 | 初版不注入，后续有需要时在 Provider 模型中加字段 |
| **多 choices 响应处理** | OpenAI 响应可能包含多个 choices（当 `n > 1`）；Claude Code 不会发送 `n > 1`，始终只需要第一个 choice | 只处理 `choices[0]`，简化实现 |
| **WebSocket 流式传输** | OpenAI Responses API 支持 WebSocket 模式；Claude Code 用标准 HTTP SSE；不需要 WebSocket 支持 | 仅支持 HTTP SSE |

---

## Feature Dependencies

```
[协议路由（ProtocolType 检测）]
    └──requires──> [现有 ProtocolType 枚举（Anthropic / OpenAiCompatible）]
    └──决定──> [是否进入转换路径 vs 透传路径]

[请求格式转换（anthropic_to_openai）]
    └──requires──> [协议路由]
    └──requires──> [系统提示转换]
    └──requires──> [消息数组转换]
        └──requires──> [tool_use content block → tool_calls 转换]
        └──requires──> [tool_result content block → tool role 消息转换]
        └──requires──> [image content block → image_url 转换]
    └──requires──> [工具定义转换（input_schema → parameters）]
    └──produces──> [OpenAI 格式请求体]

[端点重写]
    └──requires──> [协议路由（needs_transform 标志）]
    └──produces──> [/v1/chat/completions 目标端点]

[响应格式转换（openai_to_anthropic）]
    └──requires──> [非流式响应检测（非 SSE）]
    └──requires──> [tool_calls → tool_use content block 转换]
    └──requires──> [finish_reason → stop_reason 映射]
    └──requires──> [usage 字段重命名]
    └──produces──> [Anthropic 格式响应体]

[流式 SSE 转换（create_anthropic_sse_stream）]
    └──requires──> [流式响应检测（Content-Type: text/event-stream / request.stream: true）]
    └──requires──> [有状态流转换器（message_id, content index 计数器）]
    └──requires──> [文本 delta 转换（OpenAI content → Anthropic text_delta）]
    └──requires──> [工具调用流式转换（delta.tool_calls → input_json_delta）]
        └──requires──> [pending buffer（等待 id+name 就绪）]
        └──requires──> [工具块状态映射（HashMap<index, ToolBlockState>）]
    └──requires──> [finish_reason → stop_reason 映射（流式版本）]
    └──produces──> [Anthropic SSE 事件流]

[图片转换] ──enhances──> [请求格式转换]
[cache_control 透传] ──enhances──> [请求格式转换]
[JSON Schema 清理] ──requires──> [工具定义转换]

[非流式转换路径]
    └──requires──> [请求格式转换]
    └──requires──> [响应格式转换]
    └──conflicts──> [流式转换路径]（二者互斥，由 request.stream 字段决定）

[流式转换路径]
    └──requires──> [请求格式转换]
    └──requires──> [流式 SSE 转换]
    └──conflicts──> [非流式转换路径]
```

### 依赖关键说明

- **流式检测 requires 请求体解析**：代理层在转换模式下必须解析请求 JSON 才能读取 `stream` 字段决定走哪条路径。现有 `proxy_handler` 的透传模式不解析请求体（raw bytes 透传），转换模式需要先 `serde_json::from_bytes`。
- **流式工具调用 depends on 有状态 buffer**：OpenAI 流式协议下，tool call 的 `id` 和 `name` 可能在 `arguments` 之后到达。转换器必须缓存未完成的工具块，等 id+name 均就绪后再发出 `content_block_start`。
- **端点重写 depends on 协议路由**：必须在发送请求前就知道 `needs_transform`，才能改写 URL 路径。这要求在 `build_url` 阶段（请求构建时）就做判断，不能在响应阶段才处理。
- **响应转换不能与流式转换共存**：收到响应后，通过 Content-Type 头（`text/event-stream`）或请求中的 `stream: true` 判断走哪条路径。两条路径互斥，不能叠加应用。

---

## MVP Definition

### v2.2 Launch With（必做）

- [ ] **协议路由**：检测 Provider `ProtocolType == OpenAiCompatible`，决定进入转换路径 — 无此功能转换层完全无法工作
- [ ] **请求格式转换：系统提示**：`system` 字段 → messages 数组首条 `{role:"system"}` — Claude Code 在几乎所有请求中都带 system prompt
- [ ] **请求格式转换：消息数组**：字符串 content 直接透传；content block 数组中 text/tool_use/tool_result 各类型正确转换 — 核心对话流程
- [ ] **请求格式转换：工具定义**：`input_schema` → `function.parameters`；添加 `type:"function"` 包装；过滤 BatchTool — Claude Code 重度依赖工具调用
- [ ] **端点重写**：`/v1/messages` → `/v1/chat/completions` — 没有端点重写转换无意义
- [ ] **非流式响应转换**：choices → content blocks；finish_reason → stop_reason；usage 重命名 — 基本请求-响应循环
- [ ] **非流式工具调用响应转换**：`tool_calls` → `tool_use` content blocks；arguments JSON 字符串反序列化为对象 — 工具调用核心
- [ ] **流式 SSE 转换：文本**：OpenAI `content` delta → Anthropic `text_delta` 事件序列（message_start → content_block_start → content_block_delta → content_block_stop → message_delta → message_stop）— 流式是 Claude Code 默认模式
- [ ] **流式 SSE 转换：工具调用**：delta.tool_calls with pending buffer → content_block_start/delta 序列 — 工具调用流式版本
- [ ] **流式 SSE 转换：finish_reason**：最终 message_delta 携带 stop_reason — 没有正确 stop_reason，Claude Code 无法判断生成完毕

### Add After Validation（v2.2.x）

- [ ] **图片/多模态转换**：base64 图片 → `image_url` data URL — 触发条件：用户开始在 Claude Code 中传图片给 OpenAI Provider
- [ ] **cache_control 透传**：保留 cache_control 字段透传 — 触发条件：用户使用支持 prompt caching 的 OpenAI 兼容 Provider
- [ ] **旧版 function_call 格式兼容**：处理部分 Provider 的旧版响应格式 — 触发条件：遇到返回 function_call 而非 tool_calls 的 Provider

### Future Consideration（v3.0+）

- [ ] **反向转换（OpenAI→Anthropic）**：Codex 使用 Anthropic Provider — 在 v3.0 全功能网关里程碑
- [ ] **OpenAI Responses API 格式支持**：新 Responses API 结构不同于 Chat Completions — cc-switch 已有参考实现 (`transform_responses.rs`)
- [ ] **模型名称映射配置**：允许用户配置模型名映射表 — 当前用户自行填写正确模型名

---

## Feature Prioritization Matrix

| Feature | 用户价值 | 实现成本 | 优先级 |
|---------|---------|---------|--------|
| 协议路由（ProtocolType 检测） | HIGH | LOW | P1 |
| 请求转换：系统提示 | HIGH | LOW | P1 |
| 请求转换：消息数组（text/tool_use/tool_result） | HIGH | MEDIUM | P1 |
| 请求转换：工具定义 | HIGH | MEDIUM | P1 |
| 端点重写 /v1/messages → /v1/chat/completions | HIGH | LOW | P1 |
| 非流式响应转换（文本） | HIGH | MEDIUM | P1 |
| 非流式响应转换（工具调用） | HIGH | MEDIUM | P1 |
| 流式 SSE 转换（文本） | HIGH | HIGH | P1 |
| 流式 SSE 转换（工具调用 + pending buffer） | HIGH | HIGH | P1 |
| 流式 finish_reason → stop_reason | HIGH | LOW | P1 |
| Usage 字段映射 | MEDIUM | LOW | P1 |
| JSON Schema 清理 | MEDIUM | LOW | P1 |
| 图片/多模态转换 | MEDIUM | MEDIUM | P2 |
| cache_control 透传 | LOW | LOW | P2 |
| cache token 字段映射 | LOW | LOW | P2 |
| 旧版 function_call 兼容 | LOW | LOW | P2 |
| content_filter finish_reason 处理 | LOW | LOW | P2 |

**优先级说明：**
- P1: v2.2 必须实现，缺少则 Claude Code 无法用 OpenAI Provider
- P2: 提升兼容性和稳定性，v2.2.x 或按需加入
- P3: 超出 v2.2 范围

---

## 协议格式对照速查

### 请求：系统提示

| 方向 | 格式 |
|------|------|
| Anthropic（输入） | `{"system": "You are helpful"}` 或 `{"system": [{"type":"text","text":"...","cache_control":{...}}]}` |
| OpenAI（输出） | `{"messages": [{"role":"system","content":"You are helpful"}, ...]}` |

### 请求：工具定义

| 字段 | Anthropic | OpenAI |
|------|-----------|--------|
| 顶层结构 | `tools: [{name, description, input_schema}]` | `tools: [{type:"function", function:{name, description, parameters}}]` |
| Schema 字段名 | `input_schema` | `parameters` |
| 类型标识 | 无（隐式） | `type: "function"` 必须 |
| 特殊类型 | `type: "BatchTool"` 需过滤 | 无对应 |

### 请求：消息中的工具调用（assistant 发出的）

| 字段 | Anthropic | OpenAI |
|------|-----------|--------|
| 位置 | `message.content` 数组中的 `{type:"tool_use", id, name, input}` | `message.tool_calls: [{id, type:"function", function:{name, arguments}}]` |
| 参数格式 | `input`: JSON 对象 | `function.arguments`: JSON 字符串 |

### 请求：工具结果（user 回传的）

| 字段 | Anthropic | OpenAI |
|------|-----------|--------|
| 角色 | user（content 数组中有 `{type:"tool_result", tool_use_id, content}`）| tool（独立消息：`{role:"tool", tool_call_id, content}`）|
| 变化 | 1 条 user 消息含多个 tool_result | 每个 tool_result 变成 1 条独立的 tool 角色消息 |

### 响应：stop_reason / finish_reason 映射

| OpenAI finish_reason | Anthropic stop_reason |
|---------------------|----------------------|
| `"stop"` | `"end_turn"` |
| `"length"` | `"max_tokens"` |
| `"tool_calls"` | `"tool_use"` |
| `"function_call"` | `"tool_use"`（旧版兼容）|
| `"content_filter"` | `"end_turn"`（无精确对应）|

### 流式事件序列对照

| 阶段 | OpenAI SSE | Anthropic SSE |
|------|-----------|---------------|
| 开始 | 第一个 chunk（含 id, model）| `event: message_start` |
| 文本开始 | delta.content 首次非空 | `event: content_block_start {type:"text"}` |
| 文本片段 | `delta.content: "..."` | `event: content_block_delta {type:"text_delta", text:"..."}` |
| 文本结束 | finish_reason 到来 | `event: content_block_stop` |
| 工具调用开始 | `delta.tool_calls[{index, id, function:{name}}]` | `event: content_block_start {type:"tool_use", id, name}` |
| 工具参数片段 | `delta.tool_calls[{function:{arguments:"..."}}]` | `event: content_block_delta {type:"input_json_delta", partial_json:"..."}` |
| 工具调用结束 | finish_reason 到来 | `event: content_block_stop` |
| 终止 | `finish_reason: "stop"/"tool_calls"` | `event: message_delta {stop_reason:"..."}` |
| 流结束 | `data: [DONE]` | `event: message_stop` |

---

## 与现有架构的集成点

基于对 `src-tauri/src/proxy/handler.rs` 和 `state.rs` 的分析：

| 集成点 | 现状 | v2.2 需要的变更 |
|--------|------|----------------|
| `ProxyState.UpstreamTarget` | 已有 `ProtocolType` 字段 | 无需改动；`OpenAiCompatible` 即转换触发条件 |
| `proxy_handler` | 读取 raw bytes 透传 | 转换模式下需解析 JSON 并调用转换函数，然后序列化回 bytes 转发 |
| 响应路径 | `Body::from_stream(upstream_resp.bytes_stream())` 直接流式透传 | 转换模式下：非流式需缓冲全部响应体再转换；流式需接入 `create_anthropic_sse_stream` 流转换器 |
| 端点 URL | 直接拼接 `base_url + path` | 转换模式下将 path 中的 `/v1/messages` 替换为 `/v1/chat/completions` |
| 请求头 `Accept` | 透传 | 可能需要确保转发到 OpenAI Provider 时 Accept 头兼容；通常无需改动 |

---

## Sources

- [Anthropic Messages API 文档](https://docs.anthropic.com/en/api/messages) — 请求/响应格式规范 [HIGH]
- [Anthropic Streaming Messages 文档](https://docs.anthropic.com/en/api/messages-streaming) — SSE 事件序列规范 [HIGH]
- [Anthropic Errors 文档](https://docs.anthropic.com/en/api/errors) — 错误类型与 HTTP 状态码映射 [HIGH]
- [OpenAI Chat Completions API 参考](https://platform.openai.com/docs/api-reference/chat/create) — 请求/响应格式规范 [HIGH]
- [cc-switch transform.rs](../../../cc-switch/src-tauri/src/proxy/providers/transform.rs) — `anthropic_to_openai` 和 `openai_to_anthropic` 参考实现 [HIGH]
- [cc-switch streaming.rs](../../../cc-switch/src-tauri/src/proxy/providers/streaming.rs) — `create_anthropic_sse_stream` 流式转换参考实现（含多工具调用 + pending buffer）[HIGH]
- [cc-switch models/anthropic.rs](../../../cc-switch/src-tauri/src/proxy/providers/models/anthropic.rs) — Anthropic 数据模型定义 [HIGH]
- [cc-switch models/openai.rs](../../../cc-switch/src-tauri/src/proxy/providers/models/openai.rs) — OpenAI 数据模型定义 [HIGH]
- [cc-switch claude.rs](../../../cc-switch/src-tauri/src/proxy/providers/claude.rs) — `needs_transform` 判断逻辑 + `transform_request`/`transform_response` 实现 [HIGH]
- [CLIManager src/proxy/handler.rs](../../../src-tauri/src/proxy/handler.rs) — 现有代理处理器架构 [HIGH]
- [CLIManager src/proxy/state.rs](../../../src-tauri/src/proxy/state.rs) — `UpstreamTarget` + `ProtocolType` 字段 [HIGH]

---

*Feature research for: 协议转换层（v2.2 milestone）*
*Researched: 2026-03-14*
