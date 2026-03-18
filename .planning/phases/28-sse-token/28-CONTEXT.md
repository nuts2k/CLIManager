# Phase 28: 流式 SSE Token 提取 - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

三种协议（Anthropic、OpenAI Chat Completions、OpenAI Responses API）的流式 SSE 请求在 stream 完全结束后，正确提取 token 用量（input_tokens、output_tokens、cache_creation_tokens、cache_read_tokens）、stop_reason、TTFB 和 duration，UPDATE 已有日志行并 emit 更新事件到前端。

</domain>

<decisions>
## Implementation Decisions

### Token 提取时机与写入策略（Phase 26/27 已锁定）
- Stream EOF 后统一解析 token，不在中途提取
- Phase 27 已为流式请求写入基础元数据（token/duration/ttfb 留 null），Phase 28 在 EOF 后 UPDATE 同一行填充
- UPDATE 完成后通过 `traffic-log` 事件 emit type="update" 更新前端

### Token 数据位置（三协议）
- **Anthropic 原始流**: `message_delta` 事件的 `usage` 字段包含 input_tokens/output_tokens/cache_creation_input_tokens/cache_read_input_tokens；`message_delta.delta.stop_reason` 含停止原因
- **OpenAI Chat Completions 流**: 含 `finish_reason` 的最后一个 chunk 中 `usage` 字段包含 prompt_tokens/completion_tokens；缓存在 `usage.prompt_tokens_details.cached_tokens`；stream.rs 中已有 `Usage` 结构体和 `extract_cache_read_tokens()` 函数
- **OpenAI Responses API 流**: `response.completed` 事件的 `response.usage` 包含 input_tokens/output_tokens；缓存字段在 `response.usage.input_token_details.cached_tokens`（Phase 27 研究确认）

### 耗时字段（Phase 26 已锁定）
- TTFB: 代理向上游发出 reqwest 请求 → 收到上游响应第一字节（handler 中 `upstream_resp` 返回的时间点即为 TTFB）
- Duration: handler 全生命周期（从收到客户端请求到响应完全发送完毕，含流式 stream 全部传输）

### stop_reason 策略（Phase 27 已锁定）
- 存原始值，不做跨协议映射（Anthropic 存 end_turn/max_tokens 等，OpenAI 存 stop/length 等）

### 缓存 Token 提取
- Phase 26 已预留 cache_creation_tokens 和 cache_read_tokens 列
- Phase 28 对三协议流式响应均提取缓存 token（不再留 null）
- Responses API 缓存字段位置：`usage.input_token_details.cached_tokens`（creation=None，与 Chat Completions 一致）

### Claude's Discretion
- 流内 token 数据累积的具体实现方式（channel、闭包回调、Arc<Mutex> 等）
- UPDATE SQL 的行定位标识（rowid、created_at 组合等）
- TTFB 时间点如何从 handler 传递到流结束时的 UPDATE 逻辑
- Responses API 缓存字段是否实际存在于流式 response.completed 事件中（不存在时优雅降级为 None）

</decisions>

<specifics>
## Specific Ideas

- Phase 27 已建立的双轨事件模式（new + update）正好适配：流式请求开始时 emit type="new"（token=null），stream 结束后 UPDATE 并 emit type="update"（token 填充），前端看到"进行中"→"完成"的状态变化
- 三种流式处理函数中 token 数据已在流内可见（stream.rs 的 Usage 结构、responses_stream.rs 的 response.completed 解析、Anthropic 的 message_delta），只需增加"提取并回传"机制

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `stream.rs::Usage` 结构体: 已定义含 prompt_tokens/completion_tokens/prompt_tokens_details/cache_read_input_tokens/cache_creation_input_tokens 的反序列化结构
- `stream.rs::extract_cache_read_tokens()`: 已有 OpenAI 缓存 token 兼容提取函数
- `handler.rs::extract_anthropic_tokens()` / `extract_openai_chat_tokens()` / `extract_responses_tokens()`: 非流式 token 提取函数，可参考字段映射
- `handler.rs::send_error_log()`: 错误日志发送模式（构建 LogEntry + try_send），可复用模式
- `traffic/log.rs::LogEntry`: 18 字段日志结构，流式请求已写入基础字段
- `traffic/mod.rs::TrafficDb`: insert_request_log 已实现，需新增 update 方法

### Established Patterns
- `create_anthropic_sse_stream()`: 逐 chunk 按行解析 SSE，维护 buffer + 状态机，在 finish_reason chunk 中已解析 Usage
- `create_responses_anthropic_sse_stream()`: 按 SSE block 解析，在 `response.completed` 事件中已提取 usage
- `create_anthropic_reverse_model_stream()`: 纯 Anthropic 流按行扫描替换 model，当前不解析 usage——需扩展
- handler 中流式分支: `Body::from_stream(...)` 后直接发送 LogEntry（token=None），无等待 stream 结束的机制

### Integration Points
- handler.rs: 流式分支需增加 stream 结束后的回调/通知机制，用于 UPDATE 日志和 emit 更新事件
- traffic/mod.rs 或 traffic/log.rs: 需新增 `update_streaming_log()` 方法（UPDATE request_logs SET token 字段 WHERE 定位条件）
- 三个流式创建函数: 需增加 token 数据的"提取并回传"能力（当前只做协议转换，不输出 token）

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 28-sse-token*
*Context gathered: 2026-03-18*
