# Pitfalls Research

**Domain:** Anthropic Messages API → OpenAI Chat Completions API 协议转换（HTTP 代理层）
**Researched:** 2026-03-14
**Confidence:** HIGH（基于 cc-switch 参考实现代码审查 + 官方 API 规范对比 + 协议转换工程实践）

---

## Critical Pitfalls

### Pitfall 1: 流式 SSE 事件类型体系根本不同，逐行转发不可行

**What goes wrong:**
将 OpenAI 的 `data: {...}` SSE 行直接转发给 Claude Code，Claude Code 拒绝解析并崩溃或报错，因为它期待 Anthropic 的命名事件体系（`event: message_start`、`event: content_block_start` 等），而 OpenAI 只发 `data:` 行，没有 `event:` 行。

**Why it happens:**
两个协议的流式模型在架构层面不同：

- **OpenAI Chat Completions（SSE）：** 每个 chunk 只有 `data:` 行，最后一条是 `data: [DONE]`。一个 chunk 携带增量 delta（`content`、`tool_calls`）。整个会话只有一种"消息"，没有显式的生命周期事件。

- **Anthropic SSE：** 每个 SSE 事件同时有 `event:` 和 `data:` 两行。事件类型包括：`message_start` → `content_block_start` → `content_block_delta`（循环） → `content_block_stop` → `message_delta` → `message_stop`。这是一个严格的状态机生命周期。

简单的 `data:` 行转发不匹配 Anthropic 的 `event:` 行要求。

**How to avoid:**
在代理层实现完整的 SSE 状态机转换器，而不是逐行转发。转换器需要：
1. 维护上游流状态：`message_id`、`current_model`、`has_sent_message_start` 标志
2. 在第一个 chunk 到来时生成 `event: message_start` + `event: content_block_start`
3. 将每个 delta 转换为对应的 `event: content_block_delta`
4. 在 `finish_reason` 到来时生成 `event: content_block_stop` + `event: message_delta` + `event: message_stop`
5. 将 `data: [DONE]` 转换为 `event: message_stop`

cc-switch 的 `create_anthropic_sse_stream` 函数（streaming.rs）是正确的参考实现。

**Warning signs:**
- Claude Code 日志出现 JSON 解析错误或"unexpected event type"
- 流式响应第一个 token 收到后 CLI 报错停止
- 非流式模式正常，流式模式必然崩溃

**Phase to address:**
协议转换核心阶段（Phase 1）— 流式转换是最核心的复杂性，必须第一个实现并完整测试。

---

### Pitfall 2: 工具调用流式分帧问题 — id 和 name 可能晚于 arguments 到达

**What goes wrong:**
流式工具调用转换产生无效的 Anthropic 事件序列：`content_block_start`（tool_use）中 `id` 或 `name` 为空字符串，导致 Claude Code 无法识别工具调用，工具执行失败。

**Why it happens:**
OpenAI 的流式 tool_calls delta 格式中，`id` 和 `name` 字段**不保证**在第一个 chunk 中出现，`arguments` 字段的片段可能先于 `id`/`name` 到达。例如：

```
chunk 1: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"a\":"}}]}}]}
chunk 2: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_0","type":"function","function":{"name":"get_weather"}}]}}]}
chunk 3: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"1}"}}]}}]}
```

如果在 chunk 1 就立即生成 `content_block_start`（此时 id/name 为空），Anthropic 协议会拒绝。

**How to avoid:**
实现延迟启动（Deferred Start）策略：
- 为每个 `tool_call.index` 维护状态（`id`、`name`、`started`、`pending_args`）
- 只有当 `id` 和 `name` 都已收到时，才生成 `content_block_start` 事件
- 在等待 id/name 期间，将 `arguments` 片段缓冲到 `pending_args`
- 一旦可以开始，先发送 `content_block_start`，再把缓冲的 `pending_args` 作为 `input_json_delta` 发出
- 在 `finish_reason` 到来时处理"晚启动"（late start）场景：对仍未启动但有内容的工具块，使用 fallback id/name 进行启动

cc-switch 的 `ToolBlockState` + `pending_args` 设计是正确实现。

**Warning signs:**
- 工具调用在流式模式下偶发性失败（取决于上游分帧方式）
- Claude Code 日志出现 "tool_use id is empty" 或类似错误
- 非流式工具调用正常，流式工具调用失败

**Phase to address:**
协议转换核心阶段（Phase 1）— 工具调用流式是最复杂的边缘情况，必须在核心阶段覆盖。

---

### Pitfall 3: 多并发工具调用的 index 映射错误

**What goes wrong:**
当 LLM 并发调用两个或多个工具时（如 `tool_calls` 数组有 index 0 和 index 1），转换后的 Anthropic `content_block` index 序列混乱，导致 Claude Code 将工具参数 delta 分配到错误的工具块。

**Why it happens:**
OpenAI 流式中多个工具调用使用 `index` 字段区分，且同一个 chunk 中可以同时出现不同 index 的 delta。如果转换器只维护单个"当前工具块"状态，而不是按 index 维护哈希表，多工具调用时会出现状态污染：工具 1 的 arguments 片段被写入工具 0 的 Anthropic block。

**How to avoid:**
使用 `HashMap<usize, ToolBlockState>` 按 OpenAI index 独立维护每个工具块状态，Anthropic content block index 是递增计数器（`next_content_index`），与 OpenAI index 完全独立。每次 delta 根据 `tool_call.index` 路由到正确的工具块状态，而不是假设工具调用是顺序单一的。

**Warning signs:**
- 单工具调用正常，多工具并发调用时参数混乱
- 工具执行时参数 JSON 不完整或是另一个工具的参数
- Claude Code 报"工具参数解析失败"

**Phase to address:**
协议转换核心阶段（Phase 1）— 在工具调用流式转换实现时必须覆盖多工具场景的测试。

---

### Pitfall 4: tool_result 消息结构错位 — Anthropic user 消息 vs OpenAI tool role

**What goes wrong:**
Anthropic 中工具结果是 `user` role 消息的 content 数组中的 `tool_result` block；OpenAI 中工具结果是独立的 `tool` role 消息。转换错误时，工具结果被附加到错误的消息上，或被当成普通文本内容，导致上游模型看不到工具执行结果，无法继续对话。

**Why it happens:**
消息角色语义完全不同：

```
Anthropic：
{"role": "user", "content": [{"type": "tool_result", "tool_use_id": "call_123", "content": "Sunny"}]}

OpenAI：
{"role": "tool", "tool_call_id": "call_123", "content": "Sunny"}
```

如果将 Anthropic user 消息的 content 数组简单地转换为 OpenAI user 消息的 content，`tool_result` block 会变成乱码文本而不是结构化的工具回应。

同时，一个 Anthropic user 消息可以同时包含 `tool_result` blocks 和 `text` blocks（混合内容），转换时需要将它们**拆分**为：先是 tool role 消息（每个 tool_result 一条），然后可能跟随 user 消息（text content）。

**How to avoid:**
- 转换 Anthropic messages 时，为每个 `tool_result` block 生成独立的 `{"role": "tool", "tool_call_id": ..., "content": ...}` 消息
- `tool_result` 的 `content` 可能是字符串或对象，需要统一序列化为字符串（`serde_json::to_string` 后传入）
- 处理混合内容：一次迭代中先收集 `tool_result` blocks（生成 tool 消息），再收集 `text` blocks（生成 user 消息），不要将两者混在同一条消息中

**Warning signs:**
- 工具调用后模型回应不包含工具结果，表现为"工具结果被忽略"
- 上游 API 返回 400 错误（tool 消息格式不符合规范）
- Claude Code 的多轮对话在工具调用后断开

**Phase to address:**
协议转换核心阶段（Phase 1）— tool_result 转换是非流式请求转换的必测路径。

---

### Pitfall 5: 现有代理的 body 是流式透传的，转换层无法访问

**What goes wrong:**
在现有代理（CLIManager v2.0）中，`proxy_handler` 读取完整 body 字节后**直接**将上游响应以 `Body::from_stream` 流式透传给客户端。添加协议转换时，开发者尝试在流式透传路径上插入 JSON 解析，发现无法同时"流式读取"和"读取完整 body"进行 JSON 转换，导致死锁或 body 被消耗两次。

**Why it happens:**
HTTP body 是一次性流。现有代理的透传路径：`upstream_resp.bytes_stream()` → `Body::from_stream()`，这条路径不缓冲响应体。如果要对响应进行 JSON 变换（非流式场景）或 SSE 逐事件变换（流式场景），必须在不同的代码路径处理：
- 非流式：先 `await resp.bytes()`，再 JSON 解析，再重新构造响应体
- 流式：不能读完整响应体再处理，需要逐 SSE 事件处理（边收 chunk 边发出转换后的 Anthropic SSE）

**How to avoid:**
在 handler 层根据 Provider 协议类型和 `stream` 字段分支：
1. **透传路径**（Anthropic Provider）：保持现有 `Body::from_stream` 透传，不改
2. **非流式转换路径**（OpenAI Provider + stream=false）：`resp.bytes().await` 完整读取 → JSON 转换 → 重新构造 Response body
3. **流式转换路径**（OpenAI Provider + stream=true）：`resp.bytes_stream()` 但不接 `Body::from_stream`，而是接入 SSE 状态机转换器（async-stream），输出 Anthropic SSE 格式的字节流

关键设计决策：转换路径必须在 handler 层做分支，不能在现有透传代码上"插入"中间层。

**Warning signs:**
- 尝试对 `reqwest::Response` 调用 `.bytes().await` 后再调用 `.bytes_stream()` 时编译报错（已移动）
- 转换路径的响应 body 为空
- 流式响应第一个 token 后挂起

**Phase to address:**
协议转换集成阶段（Phase 2，在现有代理基础上集成）— 架构分支是集成的首要问题，需要在设计阶段而非实现阶段解决。

---

### Pitfall 6: finish_reason 到 stop_reason 的映射遗漏边缘值

**What goes wrong:**
OpenAI 返回未映射的 `finish_reason`（如 `"content_filter"`），转换器将其原样透传给 Claude Code，Claude Code 遇到未知的 `stop_reason` 值报解析错误或行为异常。

**Why it happens:**
OpenAI 的 `finish_reason` 值集合与 Anthropic 的 `stop_reason` 值集合不完全对应：

| OpenAI finish_reason | Anthropic stop_reason | 说明 |
|---------------------|----------------------|------|
| `"stop"` | `"end_turn"` | 正常结束 |
| `"length"` | `"max_tokens"` | 达到最大 token |
| `"tool_calls"` | `"tool_use"` | 工具调用 |
| `"function_call"` | `"tool_use"` | 旧版工具调用 |
| `"content_filter"` | `"end_turn"` | 内容过滤，映射为 end_turn |
| `null` | `null` | 流式中间 chunk |
| 其他值（如 `"stop_sequence"`） | 未定义 | 需要默认映射 |

**How to avoid:**
穷举映射表，并为未知值提供默认值 `"end_turn"` + 记录警告日志：
```rust
match finish_reason {
    "stop" => "end_turn",
    "length" => "max_tokens",
    "tool_calls" | "function_call" => "tool_use",
    "content_filter" => "end_turn",
    other => {
        log::warn!("Unknown finish_reason: {other}, defaulting to end_turn");
        "end_turn"
    }
}
```
不要将 OpenAI 值直接透传给 Claude Code。流式中间 chunk 的 `finish_reason` 为 null，不应触发 message_delta 事件。

**Warning signs:**
- Claude Code 日志出现 "unknown stop_reason" 或类似错误
- 内容过滤场景（上游拒绝请求）导致 Claude Code 崩溃而不是优雅报错

**Phase to address:**
协议转换核心阶段（Phase 1）— 应在 stop_reason 映射函数的单元测试中覆盖所有枚举值。

---

### Pitfall 7: tool_use 的 input 字段是 JSON 对象，但流式传输的是字符串片段

**What goes wrong:**
Anthropic 非流式响应中 `tool_use.input` 是已解析的 JSON 对象；但流式 `input_json_delta` 传输的是原始 JSON 字符串片段（`partial_json`）。在非流式转换中，从 OpenAI 响应提取工具参数时，`function.arguments` 是 JSON 字符串（如 `"{\"location\":\"Tokyo\"}"`），需要 `serde_json::from_str` 解析为 JSON 值再填入 `input` 字段。如果忘记解析，`input` 字段变成字符串而非对象，Claude Code 解析失败。

**Why it happens:**
OpenAI 中 `function.arguments` 始终是 JSON 字符串（即使对象很简单）；Anthropic 中 `tool_use.input` 是 JSON 值。这是一个类型不匹配的设计差异，很容易被忽略。

**How to avoid:**
非流式响应转换时，对 `function.arguments` 进行显式 JSON 解析：
```rust
let args_str = func.get("arguments").and_then(|a| a.as_str()).unwrap_or("{}");
let input: Value = serde_json::from_str(args_str).unwrap_or(json!({}));
```
解析失败时 fallback 为空对象，避免 panic。流式路径的 `input_json_delta` 传的是字符串片段，不需要解析（Anthropic 流式协议本身定义 `partial_json` 就是字符串）。

**Warning signs:**
- 非流式工具调用中 Claude Code 报 "tool input is not an object"
- 工具参数在非流式模式下丢失或错误

**Phase to address:**
协议转换核心阶段（Phase 1）— 在工具调用单元测试中包含带参数的场景。

---

### Pitfall 8: thinking/extended_thinking blocks 无法在 OpenAI 协议中传递

**What goes wrong:**
Claude Code 在需要 extended thinking 的场景中向代理发送包含 `{"type": "thinking", "thinking": "...", "signature": "..."}` 的 assistant 消息。代理将这些 blocks 直接转发给 OpenAI 兼容上游，上游返回 400 错误（`"Extra inputs are not permitted"` 或 `"Unknown content block type"`），整个请求失败。

**Why it happens:**
Anthropic 的 thinking block 是 Anthropic 专有的内容块类型，OpenAI Chat Completions 协议没有对应概念。历史消息中的 thinking blocks（含 `signature` 字段）在转换为 OpenAI 格式时必须被静默丢弃，因为：
1. OpenAI 不认识 `thinking` 类型
2. `signature` 字段会触发 OpenAI 的额外字段校验错误
3. thinking 内容对 OpenAI 模型无意义（它们无法继续 Anthropic 的 thinking 链）

更深的问题：当代理接收到带 `budget_tokens` 的 `thinking` 配置参数时，应同样丢弃（OpenAI 没有这个字段），否则触发上游 400 错误。

**How to avoid:**
在请求转换层（`anthropic_to_openai`）对 content blocks 迭代时，遇到 `"thinking"` 或 `"redacted_thinking"` 类型直接跳过（不生成任何 OpenAI 消息内容）。同时过滤请求顶层的 `thinking` 字段（thinking budget 配置）。

注意：cc-switch 还维护了一个 `thinking_rectifier` 模块用于处理第三方渠道的 signature 校验错误（某些透传 Anthropic 协议的中间代理不接受 signature 字段），这是另一个边缘场景。对于 v2.2 的 OpenAI 转换，简单丢弃即可。

**Warning signs:**
- 上游 API 返回 400 + 包含 "signature" 或 "Extra inputs are not permitted" 的错误体
- 只在多轮工具使用对话中出现错误（第一轮无 thinking，第二轮可能有 thinking）
- 上游拒绝请求，但相同请求不通过代理直连 OpenAI 时正常

**Phase to address:**
协议转换核心阶段（Phase 1）— 在 content block 迭代的 match 中加入 `"thinking" => {}` 分支，确保 drop 而非 panic。

---

### Pitfall 9: 错误响应格式未转换 — 上游 OpenAI 错误直接返回给 Claude Code

**What goes wrong:**
上游 OpenAI 兼容 API 返回 4xx/5xx 错误时，响应体是 OpenAI 格式（`{"error": {"message": "...", "type": "...", "code": "..."}}`）。这个格式直接返回给 Claude Code，Claude Code 期待 Anthropic 格式（`{"type": "error", "error": {"type": "...", "message": "..."}}`），解析失败后 Claude Code 报告"未知错误"而不是具体原因。

**Why it happens:**
错误处理是最容易被忽略的转换场景。开发者通常先测试成功路径（200 响应），在成功路径转换完成后才发现错误路径还需要转换。OpenAI 和 Anthropic 的错误格式差异明显：

```json
// OpenAI 错误格式
{"error": {"message": "Invalid API key", "type": "authentication_error", "code": "invalid_api_key"}}

// Anthropic 错误格式
{"type": "error", "error": {"type": "authentication_error", "message": "Invalid API key"}}
```

**How to avoid:**
在响应处理层，检测上游响应 status code：
- 2xx：正常转换（非流式 JSON 转换 或 流式 SSE 转换）
- 4xx/5xx：读取响应体 → 尝试解析为 OpenAI 错误格式 → 转换为 Anthropic 错误格式 → 以原始 status code 返回

错误格式转换：
```rust
// OpenAI → Anthropic error format
let anthropic_error = json!({
    "type": "error",
    "error": {
        "type": openai_error["error"]["type"].as_str().unwrap_or("api_error"),
        "message": openai_error["error"]["message"].as_str().unwrap_or("Unknown error")
    }
});
```

**Warning signs:**
- Claude Code 遇到 API key 错误或 rate limit 时报"未知错误"而非具体原因
- Claude Code 无法区分 401（认证失败）和 429（限速）错误，行为不正确（如没有自动重试）

**Phase to address:**
协议转换核心阶段（Phase 1）— 错误路径测试必须与成功路径测试同时进行，不能遗漏。

---

### Pitfall 10: URL 路径重写遗漏 — /v1/messages 必须映射到 /v1/chat/completions

**What goes wrong:**
现有代理将 `/v1/messages` 请求直接转发到上游的 `/v1/messages`（透传路径）。当上游是 OpenAI 兼容 API 时，该端点不存在，上游返回 404。开发者发现 404 后临时在 URL 中硬编码 `/v1/chat/completions`，导致所有请求（包括不需要转换的 Anthropic Provider）都走了 `/v1/chat/completions`。

**Why it happens:**
现有代理的 URL 拼接逻辑（`handler.rs`）：`format!("{}{}{}",  base_url, path, query)`，其中 `path` 是原始请求路径（`/v1/messages`）。对于 OpenAI Provider，这个路径必须被替换为 `/v1/chat/completions`，但这个替换只应在需要格式转换时发生，不应影响 Anthropic Provider 的透传路径。

**How to avoid:**
在 handler 分支中，根据 Provider 类型决定目标端点：
- Anthropic Provider → 目标路径保持 `/v1/messages`（现有逻辑不变）
- OpenAI Provider（需要转换）→ 目标路径替换为 `/v1/chat/completions`

路径替换应在 handler 层显式控制，而不是通过 adapter 的 `build_url` 动态推导（避免产生难以追踪的隐式行为）。同时注意 `base_url` 去重复 `/v1/v1` 的问题：某些用户配置的 `base_url` 已包含 `/v1`，如果端点也以 `/v1/` 开头，需要去重（见 cc-switch `build_url` 中的 `while base.contains("/v1/v1")` 逻辑）。

**Warning signs:**
- 上游返回 404（Not Found）且 URL 仍是 `/v1/messages`
- OpenAI Provider 的请求总是 404，Anthropic Provider 正常

**Phase to address:**
协议转换集成阶段（Phase 2）— 路径路由是集成阶段的第一步，需要在 handler 层明确处理。

---

### Pitfall 11: system prompt 转换错误 — 数组格式 vs 字符串格式

**What goes wrong:**
Anthropic 请求中 `system` 字段可以是字符串（简单情况）或 `SystemBlock` 数组（带 `cache_control` 的情况）。如果转换器只处理字符串情况，数组格式的 system prompt 直接被丢弃，上游 OpenAI 模型完全失去 system prompt，行为发生根本变化（如丢失角色设定）。

**Why it happens:**
Claude Code 在大型项目中通常使用带 `cache_control` 的数组格式 system prompt（为了节省 token 成本）：

```json
// Anthropic 数组格式 system
"system": [
    {"type": "text", "text": "You are Claude, an AI assistant...", "cache_control": {"type": "ephemeral"}}
]
```

如果转换器只检查 `system.as_str()`，数组格式会 `as_str()` 返回 `None`，整个 system 被跳过。

**How to avoid:**
分两种情况处理：
```rust
if let Some(text) = system.as_str() {
    // 字符串 system → 单条 OpenAI system message
    messages.push(json!({"role": "system", "content": text}));
} else if let Some(arr) = system.as_array() {
    // 数组 system → 合并所有 text block 为 OpenAI system message
    for block in arr {
        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
            messages.push(json!({"role": "system", "content": text}));
        }
    }
}
```
注意：OpenAI Chat Completions 支持多条 system role 消息，可以保留每个 block 为独立消息（cc-switch 的做法），或合并为一条（更保守）。

**Warning signs:**
- Claude Code 的代码助手行为异常（无角色设定）
- 只在使用了 cache_control 的 Provider 时出现问题
- 对 system prompt 测试时只测试了简单字符串情况

**Phase to address:**
协议转换核心阶段（Phase 1）— system prompt 是高频路径，在最基础的单元测试中就应该覆盖数组格式。

---

### Pitfall 12: token 计数字段名不一致 — cache 相关字段尤其复杂

**What goes wrong:**
响应转换后，Claude Code 的 token 使用统计界面显示 0 或 NaN，或者 cache 命中显示不正确，导致用户看到错误的费用估算。

**Why it happens:**
两个协议的 usage 字段完全不同：

| 含义 | OpenAI | Anthropic |
|------|--------|-----------|
| 输入 tokens | `prompt_tokens` | `input_tokens` |
| 输出 tokens | `completion_tokens` | `output_tokens` |
| 缓存读取 | `prompt_tokens_details.cached_tokens` | `cache_read_input_tokens` |
| 缓存创建 | （无标准字段，部分服务有） | `cache_creation_input_tokens` |

问题点：
1. OpenAI 的缓存 tokens 在 `prompt_tokens_details.cached_tokens` 深层嵌套，容易被忽略
2. 部分 OpenAI 兼容服务直接返回 Anthropic 风格的缓存字段（`cache_read_input_tokens`），需要检测两种形式
3. 流式响应中 usage 可能在最后一个 chunk 才出现，也可能出现在每个 chunk（取决于上游）

**How to avoid:**
usage 转换函数：
```rust
let input_tokens = usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
let output_tokens = usage.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
// 优先使用直接字段（兼容服务），回退到嵌套字段（标准 OpenAI）
let cache_read = usage.get("cache_read_input_tokens")
    .and_then(|v| v.as_u64())
    .or_else(|| usage.pointer("/prompt_tokens_details/cached_tokens").and_then(|v| v.as_u64()));
```
流式响应中，usage 可能在 `message_start` 事件中需要提供初始值（通常 input_tokens），`message_delta` 中补充 output_tokens。如果上游流式 usage 为空，填 0，不要 panic。

**Warning signs:**
- Token 计数界面显示 0
- 使用支持缓存的 OpenAI 兼容 API 时，cache 命中不显示
- 流式模式 token 统计与非流式不一致

**Phase to address:**
协议转换核心阶段（Phase 1）— usage 映射是容易忽略的"小细节"，需要在测试用例中明确断言。

---

### Pitfall 13: cache_control headers 透传导致上游 400 错误

**What goes wrong:**
Anthropic 请求中的 `cache_control` 字段（如 `{"type": "ephemeral"}`）被原样保留在转换后的 OpenAI 请求中，上游 OpenAI 兼容 API 不认识这个字段，返回 400 错误（`"Extra inputs are not permitted"` 或 schema 校验失败）。

**Why it happens:**
Claude Code 在 system prompt、用户消息文本块、工具定义中广泛使用 `cache_control` 字段（这是 Anthropic 的 prompt caching 功能）。转换到 OpenAI 格式时，这些字段对 OpenAI 无意义且有害，必须被过滤。

但注意：**部分 OpenAI 兼容中间代理**（如某些 OpenRouter 版本）已支持透传 `cache_control`，可以将其传递到上游 Anthropic API 实现缓存。因此是否保留 `cache_control` 需要按 Provider 类型决定，不能无脑保留也不能无脑丢弃。

对于 v2.2（向标准 OpenAI 兼容 API 转发），应无条件丢弃 `cache_control` 字段。

**How to avoid:**
在 `convert_message_to_openai` 的 text block 处理中，生成 OpenAI content part 时不复制 `cache_control`：
```rust
// 正确：不传播 cache_control
content_parts.push(json!({"type": "text", "text": text}));
// 错误：意外透传
let mut part = json!({"type": "text", "text": text});
if let Some(cc) = block.get("cache_control") { part["cache_control"] = cc.clone(); }  // 不要这样做
```
工具定义的 `cache_control` 同理。

**Warning signs:**
- 上游返回 400 + 提及 "cache_control" 或 "extra fields"
- 只在长对话（需要缓存的情况）时出现错误
- 短对话（无 cache_control）正常，长对话异常

**Phase to address:**
协议转换核心阶段（Phase 1）— 需要显式测试：带 `cache_control` 的输入 → 转换后 `cache_control` 不出现在 OpenAI 请求中。

---

### Pitfall 14: JSON schema 清理不完整 — OpenAI 不支持某些 Anthropic 扩展

**What goes wrong:**
Claude Code 发送的工具定义 `input_schema` 中包含 `"format": "uri"` 等 Anthropic 接受但 OpenAI 不支持的 JSON schema 关键字，上游返回 400 或静默忽略（导致工具参数类型校验与预期不符）。

**Why it happens:**
Anthropic 的 `input_schema` 遵循 JSON Schema 草案，支持较多扩展关键字。OpenAI 的 `parameters` 字段只接受 OpenAI 支持的 JSON Schema 子集，部分关键字（如 `format`）会触发校验错误。

**How to avoid:**
实现 `clean_schema` 函数，递归清理不支持的关键字：
```rust
pub fn clean_schema(mut schema: Value) -> Value {
    if let Some(obj) = schema.as_object_mut() {
        // 移除 "format": "uri" 等不支持的 format
        if obj.get("format").and_then(|v| v.as_str()) == Some("uri") {
            obj.remove("format");
        }
        // 递归处理 properties
        for (_, value) in obj.get_mut("properties").and_then(|v| v.as_object_mut()).into_iter().flatten() {
            *value = clean_schema(value.clone());
        }
        // 递归处理 items
        if let Some(items) = obj.get_mut("items") {
            *items = clean_schema(items.clone());
        }
    }
    schema
}
```
在工具定义转换时对 `input_schema` 应用 `clean_schema`。

**Warning signs:**
- 上游返回 400 + "schema validation failed"
- 工具调用成功但参数类型不符合预期
- 复杂工具（带嵌套 schema）比简单工具更容易出错

**Phase to address:**
协议转换核心阶段（Phase 1）— 实现 `clean_schema` 后对带复杂 schema 的工具定义专项测试。

---

### Pitfall 15: 非流式转换路径缓冲了大 body，但代理有体积限制

**What goes wrong:**
对于非流式响应（OpenAI 格式），代理需要读取完整响应体（`resp.bytes().await`）进行 JSON 转换。如果上游响应体很大（如大量工具调用输出或长文本），完整缓冲会导致内存压力，在 Tauri 桌面应用环境（内存受限）中可能触发 OOM 或超时。

**Why it happens:**
非流式协议转换必须有"先读完再转换再发出"的缓冲步骤，这与流式透传的零缓冲架构矛盾。对于标准 Claude Code 使用场景（代码助手），响应体通常不大（< 32KB），但极端场景（完整文件生成）可能达到 200KB+。

**How to avoid:**
- 维持现有代理的 200MB 请求体限制（`axum::body::to_bytes(..., 200 * 1024 * 1024)`），并对响应体设置同样的上限
- Claude Code 的典型使用场景中响应体小，实际风险较低
- 如果上游 API 支持流式（大多数 OpenAI 兼容 API 支持），让 Claude Code 启用流式（`stream: true`），避开非流式大缓冲场景
- 记录非流式响应体大小到 debug 日志，方便排查潜在的大响应场景

**Warning signs:**
- 长文本生成场景响应超时
- 代理进程内存增长后 Tauri app 崩溃
- 非流式请求比流式请求慢很多

**Phase to address:**
协议转换集成阶段（Phase 2）— 在集成测试中包含大响应场景，确认没有超时或内存崩溃。

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| 只测试非流式转换，跳过流式测试 | 节省初期测试时间 | 流式路径有独立的 bug（工具分帧、状态机）无法发现 | **绝不可接受**，流式是主路径 |
| `serde_json::Value` 泛化处理而非强类型 Struct | 代码量少，灵活 | 字段名拼写错误静默失败，运行时才发现 | MVP 阶段可接受，但核心转换函数应有单元测试补偿 |
| 将 Anthropic message id 原样透传给 Claude Code | 省去 id 生成逻辑 | 上游 id 格式可能不符合 Anthropic 规范（如 `"chatcmpl-xxx"` vs `"msg_xxx"` 前缀），Claude Code 可能警告 | 可接受，Claude Code 对 id 格式通常宽容 |
| 不处理工具调用"晚启动"（late start）场景 | 代码简单 | 特定上游（先发 arguments 后发 id/name）会导致空 id/name 的 tool_use block | **不可接受**，这是真实上游行为 |
| 转换层不记录日志 | 代码简洁 | 出问题时完全没有调试信息，无法区分"转换错误"和"上游错误" | **不可接受**，debug 级别日志成本极低 |
| content_type 判断用字符串包含而非精确匹配 | 快速 | `application/json; charset=utf-8` 可能被 `"application/json"` 包含匹配忽略或匹配到错误类型 | 用 `starts_with` 替代 `contains`，可接受 |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| OpenAI Chat Completions API | 向 `/v1/messages` 发请求 | 必须改写为 `/v1/chat/completions`，请求/响应格式全部转换 |
| 工具调用流式 | 假设 id/name 在第一个 chunk | 实现延迟启动，等 id 和 name 都就绪才发 `content_block_start` |
| system prompt 转换 | 只处理字符串 system | 必须同时处理 `string` 和 `Vec<SystemBlock>` 两种格式 |
| usage 统计 | 只映射 prompt/completion tokens | 必须包含 `prompt_tokens_details.cached_tokens` → `cache_read_input_tokens` 的嵌套映射 |
| 错误响应 | 透传上游 OpenAI 格式错误体 | 必须转换为 Anthropic 错误格式（`{"type": "error", ...}`） |
| anthropic-version 请求头 | 将 Anthropic 头转发给 OpenAI | 向 OpenAI 发请求时不应包含 `anthropic-version`、`x-api-key` 等 Anthropic 专用头，应替换为 `Authorization: Bearer <key>` |
| 图片内容 | 丢弃 base64 图片块 | Anthropic `{"type": "image", "source": {...}}` → OpenAI `{"type": "image_url", "image_url": {"url": "data:...;base64,..."}}` |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| 非流式响应完整缓冲 + JSON 深拷贝 | 大响应体时内存峰值高 | 设置响应体大小上限；优先使用流式 | 响应体 > 1MB 时 |
| SSE buffer 分割问题：`\n\n` 分隔符被 chunk 截断 | 流式解析丢失事件 | buffer 必须跨 chunk 累积，找到完整 `\n\n` 才解析 | 上游实现按字节 flush 而非按行 flush 时 |
| 同步 JSON 解析阻塞 Tokio 任务 | 流式响应延迟增加 | `serde_json` 解析在 Tokio 任务中是可接受的（纯 CPU，非 IO）；避免在转换器中引入额外 spawn | 在超大 JSON body（> 10MB）时 |
| async-stream 状态机每 chunk 克隆大量数据 | CPU 使用率高 | 最小化克隆，使用引用或 `Arc` | 高并发时（v2.2 场景是单用户桌面 app，不是主要问题）|

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| 将 `x-api-key`（Anthropic）原样转发给 OpenAI | API key 泄露（错误的 header 到达错误的上游） | 在 header 重写时，根据 Provider 类型替换认证头；OpenAI 用 `Authorization: Bearer`，不转发 `x-api-key` |
| 不过滤 `anthropic-version` 等 Anthropic 专有头 | 上游 OpenAI 拒绝请求或日志暴露协议信息 | 在发往 OpenAI 的请求中，过滤 `anthropic-version`、`anthropic-beta` 等头 |
| 转换错误时将原始 body（含 API key 信息）记录到日志 | API key 可能出现在日志中 | 日志只记录 body 大小和类型，不记录完整内容；记录错误时只记录 error message，不记录完整请求体 |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| 协议转换失败时返回空响应 | Claude Code 卡住或报"连接中断" | 转换失败时返回 Anthropic 格式的错误响应（而不是 500 裸错误），包含 `"type": "error"` 和可读的错误消息 |
| 不区分"转换失败"和"上游失败" | 用户不知道是代理问题还是 Provider 问题 | 转换错误统一返回 Anthropic error type `"proxy_error"`，上游错误透传原始 type |
| 流式响应中途断开没有 message_stop 事件 | Claude Code 无限等待流结束 | 任何错误路径（上游错误、转换错误、连接断开）都应发送 `event: message_stop` 后再关闭连接 |

---

## "Looks Done But Isn't" Checklist

- [ ] **流式工具调用：** 验证并发两个工具调用（`tool_calls[0]` 和 `tool_calls[1]`）的流式转换，检查 Anthropic index 是否独立、参数不混淆
- [ ] **thinking blocks 过滤：** 发送含 `thinking` 类型 block 的历史消息，确认转换后不出现在 OpenAI 请求中，上游不返回 400
- [ ] **system prompt 数组格式：** 发送 `system: [{"type": "text", "text": "...", "cache_control": {...}}]`，确认 system prompt 出现在 OpenAI 请求的 messages[0] 中，且 `cache_control` 被过滤
- [ ] **tool_result 转换：** 多轮对话（user → assistant+tool_use → user+tool_result → assistant），确认 tool_result 变成 `role: "tool"` 消息，而非 user 消息内容
- [ ] **错误路径转换：** 用错误 API key，确认 Claude Code 收到的是 Anthropic 格式错误（`{"type": "error", ...}`），不是 OpenAI 格式
- [ ] **usage 统计：** 使用支持 cache 的上游，确认 `cache_read_input_tokens` 字段被正确映射
- [ ] **[DONE] sentinel 处理：** 流式响应末尾收到 `data: [DONE]`，确认 Claude Code 收到 `event: message_stop` 事件
- [ ] **透传路径不受影响：** Anthropic Provider（不需要转换）的请求仍然正常工作，没有被意外走入转换路径

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| 流式状态机 bug（Claude Code 崩溃） | HIGH | 定位具体触发场景 → 添加复现 unit test → 修复状态机逻辑 → 重新测试完整流式场景 |
| tool_result 转换错误（多轮对话断开） | MEDIUM | 在 `convert_message_to_openai` 中添加 `tool_result` 专项测试 → 修复分支逻辑 |
| thinking blocks 未过滤（上游 400） | LOW | 在 content block 迭代的 match 中添加 `"thinking" | "redacted_thinking" => {}` 分支 |
| 错误格式未转换（用户看不到错误原因） | LOW | 在响应处理层添加 4xx/5xx 分支，读取并转换错误体格式 |
| URL 路径未重写（上游 404） | LOW | 在 handler 层根据 Provider 类型替换目标路径 |
| cache_control 透传（上游 400） | LOW | 在 content block 转换时去掉 `cache_control` 复制逻辑 |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| SSE 事件类型体系完全不同 | Phase 1: 协议转换核心（SSE 转换器） | 流式请求收到完整 Anthropic SSE 事件序列 |
| 工具流式分帧 — id/name 晚于 arguments | Phase 1: 协议转换核心（工具流式） | 测试 id 在第 2 个 chunk 到达的场景 |
| 多并发工具调用 index 映射 | Phase 1: 协议转换核心（工具流式） | 发送 2 个并发工具调用，验证 delta 路由正确 |
| tool_result 消息结构错位 | Phase 1: 协议转换核心（非流式） | 多轮工具对话端到端测试 |
| 现有代理 body 流式透传 vs 缓冲冲突 | Phase 2: 集成（handler 架构分支） | 集成后透传路径和转换路径分别测试 |
| finish_reason 映射遗漏 | Phase 1: 协议转换核心 | 单元测试覆盖所有 finish_reason 枚举值 |
| tool_use input 字符串 vs 对象类型 | Phase 1: 协议转换核心 | 非流式工具调用验证 input 字段为 JSON 对象 |
| thinking blocks 透传 | Phase 1: 协议转换核心 | 带 thinking blocks 的请求不触发上游 400 |
| 错误响应格式未转换 | Phase 1: 协议转换核心 | 用错误 API key 触发 401，验证 Anthropic 格式错误体 |
| URL 路径重写 | Phase 2: 集成 | OpenAI Provider 请求到达 `/v1/chat/completions` |
| system prompt 数组格式 | Phase 1: 协议转换核心 | 数组格式 system 转换后出现在 messages[0] |
| token 计数字段不一致 | Phase 1: 协议转换核心 | usage 映射单元测试含缓存 token 场景 |
| cache_control 透传 | Phase 1: 协议转换核心 | 带 cache_control 输入，转换后 OpenAI 请求无此字段 |
| JSON schema 清理不完整 | Phase 1: 协议转换核心 | 带 `format: uri` schema 的工具不触发上游 400 |
| 非流式大 body 缓冲 | Phase 2: 集成 | 大响应体场景（16KB+）无超时或崩溃 |

---

## Sources

- `cc-switch/src-tauri/src/proxy/providers/transform.rs` — Anthropic↔OpenAI 完整请求/响应转换参考实现，包含 test suite（HIGH confidence）
- `cc-switch/src-tauri/src/proxy/providers/streaming.rs` — OpenAI SSE → Anthropic SSE 流式转换状态机参考实现，含并发工具/延迟启动/[DONE] sentinel 处理（HIGH confidence）
- `cc-switch/src-tauri/src/proxy/providers/transform_responses.rs` — OpenAI Responses API 转换（非 v2.2 目标，但 thinking/cache 处理模式可参考）（HIGH confidence）
- `cc-switch/src-tauri/src/proxy/providers/claude.rs` — Provider 类型检测、api_format 路由、URL 重写逻辑（HIGH confidence）
- `cc-switch/src-tauri/src/proxy/thinking_rectifier.rs` — thinking signature 错误场景分析（HIGH confidence）
- `CLIManager/src-tauri/src/proxy/handler.rs` — 现有代理 body 读取和流式透传架构（HIGH confidence）
- [Anthropic Messages API 官方文档](https://docs.anthropic.com/en/api/messages) — SSE 事件类型、content block 类型、usage 字段规范（HIGH confidence）
- [OpenAI Chat Completions API 官方文档](https://platform.openai.com/docs/api-reference/chat) — finish_reason 枚举值、tool_calls 流式格式、usage 字段规范（HIGH confidence）

---
*Pitfalls research for: CLIManager v2.2 协议转换（Anthropic Messages API → OpenAI Chat Completions API）*
*Researched: 2026-03-14*
