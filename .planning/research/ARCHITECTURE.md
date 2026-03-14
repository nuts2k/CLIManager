# Architecture Research: 协议转换集成架构

**Domain:** Anthropic→OpenAI 协议转换，集成到现有 axum 0.8 代理管道
**Researched:** 2026-03-14
**Confidence:** HIGH（基于现有代码库直接分析 + cc-switch 参考实现）

---

## 现有代理管道回顾

在设计集成点之前，先明确当前 `proxy_handler` 的请求管道步骤：

```
步骤 A  获取上游目标（UpstreamTarget，含 protocol_type）
步骤 B  提取方法/路径/查询字符串
步骤 C  读取请求 body bytes（200MB 上限）
步骤 D  拼接上游 URL（base_url + path + query）
步骤 E  过滤 hop-by-hop headers
步骤 F  检测占位凭据（PROXY_MANAGED）
步骤 G  注入真实凭据（按 protocol_type 注入 x-api-key 或 Bearer）
步骤 H  发送请求到上游
步骤 I  透传响应 status + headers
步骤 J  流式透传响应 body
```

协议转换需要在步骤 C 和 D 之间插入请求转换，在步骤 I 和 J 之间插入响应转换。

---

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Claude Code CLI（端口 15800）                  │
│                  发出 Anthropic Messages API 请求                  │
└─────────────────────────────┬───────────────────────────────────┘
                              │  POST /v1/messages（Anthropic 格式）
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   CLIManager 本地代理（axum 0.8）                  │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────┐     │
│  │                     proxy_handler                        │     │
│  │                                                         │     │
│  │  1. 读取 UpstreamTarget（含 protocol_type）               │     │
│  │  2. 读取请求 body                                         │     │
│  │  3. 【判断点】protocol_type == OpenAiCompatible？          │     │
│  │     │                                                    │     │
│  │     ├── 是 → 【请求转换】                                  │     │
│  │     │      ┌──────────────────────────────────────┐      │     │
│  │     │      │ translate_request()                   │      │     │
│  │     │      │  anthropic_to_openai(body)            │      │     │
│  │     │      │  改写 path: /v1/messages               │      │     │
│  │     │      │           → /v1/chat/completions       │      │     │
│  │     │      │  改写 headers: x-api-key → Bearer      │      │     │
│  │     │      └──────────────────────────────────────┘      │     │
│  │     │                                                    │     │
│  │     └── 否 → 直传（现有逻辑不变）                           │     │
│  │                                                         │     │
│  │  4. 发送请求到上游                                         │     │
│  │  5. 读取上游响应                                           │     │
│  │  6. 【判断点】was_translated？                            │     │
│  │     │                                                    │     │
│  │     ├── 是（非流式）→ 【响应转换】                           │     │
│  │     │      openai_to_anthropic(body)                     │     │
│  │     │      重写 Content-Type: application/json            │     │
│  │     │                                                    │     │
│  │     ├── 是（流式 SSE）→ 【流式响应转换】                      │     │
│  │     │      create_anthropic_sse_stream(upstream_stream)  │     │
│  │     │      重写 Content-Type: text/event-stream          │     │
│  │     │                                                    │     │
│  │     └── 否 → 直传（现有逻辑不变）                           │     │
│  └─────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┴──────────────────────┐
          ▼                                          ▼
┌─────────────────────┐                  ┌──────────────────────┐
│  Anthropic 官方 API  │                  │  OpenAI 兼容 Provider │
│  api.anthropic.com  │                  │  （直传，无需转换）    │
│  （直传，无需转换）   │                  │  OpenRouter, etc.    │
└─────────────────────┘                  └──────────────────────┘
```

### Component Responsibilities

| 组件 | 职责 | 状态 |
|------|------|------|
| `proxy/handler.rs` → `proxy_handler()` | 请求管道编排，判断是否需要转换，调用转换函数 | 已有，需扩展 |
| `proxy/state.rs` → `UpstreamTarget` | 持有 `protocol_type`，作为转换决策依据 | 已有，无需改动 |
| `proxy/translate/` → `translate_request()` | Anthropic 请求 → OpenAI 请求结构转换，路径重写 | 新建 |
| `proxy/translate/` → `translate_response()` | OpenAI 响应 → Anthropic 响应结构转换（非流式） | 新建 |
| `proxy/translate/` → `translate_stream()` | OpenAI SSE 流 → Anthropic SSE 流实时转换 | 新建 |
| `proxy/error.rs` → `ProxyError` | 新增 `TransformError(String)` 变体 | 已有，需扩展 |
| `provider.rs` → `ProtocolType` | 已有 `Anthropic` / `OpenAiCompatible` 枚举 | 已有，无需改动 |

---

## Recommended Project Structure

```
src-tauri/src/proxy/
├── mod.rs                # 已有：pub use 新增 translate 子模块
├── error.rs              # 已有：新增 TransformError 变体
├── handler.rs            # 已有：扩展 proxy_handler 插入转换逻辑
├── server.rs             # 已有：无需改动
├── state.rs              # 已有：无需改动（UpstreamTarget.protocol_type 已存在）
└── translate/            # 新建子模块
    ├── mod.rs            # pub use 导出，定义转换入口函数
    ├── request.rs        # anthropic_to_openai() 请求结构转换
    ├── response.rs       # openai_to_anthropic() 响应结构转换
    └── stream.rs         # create_anthropic_sse_stream() 流式转换
```

### Structure Rationale

- **`translate/` 子模块**：转换逻辑与转发逻辑完全分离，单独可测。cc-switch 将转换放在 `proxy/providers/transform.rs` + `streaming.rs`，功能正确但与 Provider 路由逻辑耦合过深。独立 `translate/` 更清晰。
- **`handler.rs` 内联判断**：转换决策（是否需要转换、是否流式）保留在 handler 中，translate 子模块只负责纯函数转换，不感知 axum 上下文。
- **不引入 trait 抽象**：v2.2 只做 Anthropic→OpenAI 单向转换，无需 Provider adapter 抽象层（cc-switch 的 `ProviderAdapter` trait 是为支持 Codex/Gemini 多协议设计的，我们不需要）。

---

## Architectural Patterns

### Pattern 1: 请求管道内的条件转换分支

**What:** 在现有 `proxy_handler` 内，步骤 C（读取 body）之后、步骤 D（拼接 URL）之前，根据 `upstream.protocol_type` 执行条件分支：
- `ProtocolType::Anthropic` → 直传路径，不修改 body/path（现有行为保留）
- `ProtocolType::OpenAiCompatible` → 转换路径，调用 `translate_request()`

同样地，在步骤 H（发送请求）之后，根据"是否已转换"标志决定如何处理响应：已转换则调用 `translate_response()` 或 `translate_stream()`，否则直传。

**When to use:** 每个请求只经历一次判断，零开销分支。条件写在同一个函数内，便于追踪完整请求生命周期。

**Trade-offs:**
- 好处：handler 内逻辑线性可读，调试容易
- 坏处：handler 函数变长，需注意保持关注点分离（判断逻辑 vs 转换细节）

**核心实现骨架（扩展后的 proxy_handler）：**

```rust
pub async fn proxy_handler(
    State(state): State<ProxyState>,
    req: axum::extract::Request,
) -> Result<Response, ProxyError> {
    let upstream = state.get_upstream().await.ok_or(ProxyError::NoUpstreamConfigured)?;

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let headers = req.headers().clone();

    // 步骤 C：读取 body
    let body_bytes = axum::body::to_bytes(req.into_body(), 200 * 1024 * 1024).await
        .map_err(|e| ProxyError::Internal(format!("读取请求体失败: {}", e)))?;

    // 【新增】步骤 C.5：检查是否需要协议转换
    let needs_translation = matches!(upstream.protocol_type, ProtocolType::OpenAiCompatible)
        && path == "/v1/messages";

    // 【新增】转换请求（body + path）
    let (final_body, final_path) = if needs_translation {
        let body_json: Value = serde_json::from_slice(&body_bytes)
            .map_err(|e| ProxyError::TransformError(format!("解析请求体失败: {}", e)))?;
        let is_streaming = body_json.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);
        let openai_body = translate::request::anthropic_to_openai(body_json)?;
        (serde_json::to_vec(&openai_body)
            .map_err(|e| ProxyError::TransformError(e.to_string()))?,
         "/v1/chat/completions".to_string())
    } else {
        (body_bytes.to_vec(), path.clone())
    };

    // 步骤 D：拼接上游 URL（使用可能已重写的 path）
    let upstream_url = format!("{}{}{}", upstream.base_url.trim_end_matches('/'), final_path, query);

    // 步骤 E-G：构建请求、过滤 headers、注入凭据（现有逻辑不变）
    // ...

    // 步骤 H：发送请求
    let upstream_resp = req_builder.body(final_body).send().await
        .map_err(|e| ProxyError::UpstreamUnreachable(e.to_string()))?;

    // 步骤 I：构建响应
    let status = upstream_resp.status();
    let resp_headers = upstream_resp.headers().clone();

    // 【新增】步骤 I.5：判断是否为流式响应
    let is_sse = resp_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/event-stream"))
        .unwrap_or(false);

    // 步骤 J：响应体处理
    if needs_translation && is_sse {
        // 流式转换路径
        let translated_stream = translate::stream::create_anthropic_sse_stream(
            upstream_resp.bytes_stream()
        );
        // ... 返回 SSE 响应
    } else if needs_translation {
        // 非流式转换路径
        let resp_bytes = upstream_resp.bytes().await
            .map_err(|e| ProxyError::Internal(e.to_string()))?;
        let openai_resp: Value = serde_json::from_slice(&resp_bytes)
            .map_err(|e| ProxyError::TransformError(e.to_string()))?;
        let anthropic_resp = translate::response::openai_to_anthropic(openai_resp)?;
        // ... 返回 JSON 响应
    } else {
        // 直传路径（现有逻辑）
        let body = Body::from_stream(upstream_resp.bytes_stream());
        // ...
    }
}
```

### Pattern 2: 纯函数转换层（无副作用）

**What:** `translate/request.rs`、`translate/response.rs`、`translate/stream.rs` 均为纯函数，输入 JSON 值/字节流，输出 JSON 值/字节流。不持有状态，不访问 `ProxyState`，无异步（stream 除外）。

这与 cc-switch 的 `proxy/providers/transform.rs` 设计一致，已验证可行：

- `anthropic_to_openai(body: Value) -> Result<Value, ProxyError>` — 纯结构转换
- `openai_to_anthropic(body: Value) -> Result<Value, ProxyError>` — 纯结构转换
- `create_anthropic_sse_stream(stream: impl Stream<...>) -> impl Stream<...>` — 流适配器

**When to use:** 转换逻辑单元测试极简，只需构造 JSON 值验证输出，无需启动代理服务器。

**Trade-offs:**
- 好处：测试覆盖率高，逻辑清晰
- 坏处：stream 转换函数签名较复杂（泛型 + lifetime），需要 `async-stream` crate

**注意（来自 cc-switch 经验）：** 流式工具调用存在"先到参数后到 id/name"的情况，需要缓冲 pending_args 直到 id 和 name 就绪再发送 `content_block_start`。这是流式转换最复杂的边缘情况。

### Pattern 3: 转换决策由 ProtocolType + path 双重判断

**What:** 仅当满足两个条件时触发转换：
1. `upstream.protocol_type == ProtocolType::OpenAiCompatible`
2. 请求路径为 `/v1/messages`（Anthropic Messages API 端点）

其他路径（如 `/v1/models`、`/health`）即使是 OpenAI 兼容 Provider 也直传，因为这些路径无需协议差异处理。

**When to use:** 避免误转换非 Messages API 的请求（如 Claude Code 也会发 `/v1/complete`、`/v1/token_count` 等）。

**Trade-offs:**
- 好处：精确，不影响其他 API 端点
- 坏处：如果 Claude Code 未来增加新端点需要更新判断逻辑

---

## Data Flow

### 请求转换流（非流式，protocol_type = OpenAiCompatible）

```
Claude Code CLI
    │  POST /v1/messages HTTP/1.1
    │  x-api-key: PROXY_MANAGED
    │  Content-Type: application/json
    │  Body: {
    │    "model": "claude-sonnet-4-5",
    │    "max_tokens": 8096,
    │    "system": "You are a helpful assistant.",
    │    "messages": [{"role": "user", "content": "Hello"}],
    │    "tools": [...],
    │    "stream": false
    │  }
    ▼
proxy_handler（步骤 A-C）
    │  检测：protocol_type == OpenAiCompatible && path == /v1/messages
    ▼
translate_request::anthropic_to_openai()
    │  system → messages[0]{role: system}
    │  messages → messages（role/content 结构转换）
    │  tools → tools（input_schema → parameters，name/desc 保留）
    │  max_tokens → max_tokens
    │  path 重写：/v1/messages → /v1/chat/completions
    ▼
proxy_handler（步骤 E-G）
    │  headers 过滤，注入凭据：
    │  x-api-key 占位 → Authorization: Bearer {api_key}
    │  POST /v1/chat/completions HTTP/1.1
    ▼
上游 OpenAI 兼容 Provider
    │  响应：{
    │    "id": "chatcmpl-xxx",
    │    "choices": [{
    │      "message": {"role": "assistant", "content": "Hello!"},
    │      "finish_reason": "stop"
    │    }],
    │    "usage": {"prompt_tokens": 10, "completion_tokens": 5}
    │  }
    ▼
translate_response::openai_to_anthropic()
    │  choices[0].message → content blocks
    │  finish_reason "stop" → stop_reason "end_turn"
    │  usage.prompt_tokens → usage.input_tokens
    │  usage.completion_tokens → usage.output_tokens
    ▼
Claude Code CLI
    │  响应：{
    │    "id": "chatcmpl-xxx",
    │    "type": "message",
    │    "role": "assistant",
    │    "content": [{"type": "text", "text": "Hello!"}],
    │    "stop_reason": "end_turn",
    │    "usage": {"input_tokens": 10, "output_tokens": 5}
    │  }
```

### 流式请求转换流（stream: true，protocol_type = OpenAiCompatible）

```
Claude Code CLI
    │  POST /v1/messages
    │  Body: { ..., "stream": true }
    ▼
proxy_handler → translate_request::anthropic_to_openai()
    │  请求转换同上，含 "stream": true
    ▼
上游 OpenAI 兼容 Provider（SSE 流式响应）
    │  Content-Type: text/event-stream
    │  data: {"id":"chatcmpl-xxx","choices":[{"delta":{"content":"Hello"}}]}
    │  data: {"choices":[{"delta":{"content":"!"}}]}
    │  data: {"choices":[{"delta":{},"finish_reason":"stop"}],"usage":{...}}
    │  data: [DONE]
    ▼
translate_stream::create_anthropic_sse_stream()（逐 chunk 转换）
    │  第一个有效 chunk → 生成 message_start 事件（含 usage.input_tokens）
    │  delta.content → content_block_start{type:text} + content_block_delta{text_delta}
    │  delta.tool_calls → content_block_start{type:tool_use} + input_json_delta
    │  finish_reason → 关闭所有 content blocks + message_delta{stop_reason} + message_stop
    │  [DONE] → message_stop
    ▼
Claude Code CLI
    │  event: message_start
    │  data: {"type":"message_start","message":{...,"usage":{"input_tokens":10}}}
    │
    │  event: content_block_start
    │  data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}
    │
    │  event: content_block_delta
    │  data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello!"}}
    │
    │  event: content_block_stop
    │  data: {"type":"content_block_stop","index":0}
    │
    │  event: message_delta
    │  data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}
    │
    │  event: message_stop
    │  data: {"type":"message_stop"}
```

### 直传流（protocol_type = Anthropic，保持现有行为）

```
Claude Code CLI → proxy_handler → 无转换 → Anthropic API → 直传响应
```

---

## New vs Modified Components

### 新增文件

| 文件 | 作用 | 关键内容 |
|------|------|----------|
| `src-tauri/src/proxy/translate/mod.rs` | 转换子模块入口 | pub use 导出，无业务逻辑 |
| `src-tauri/src/proxy/translate/request.rs` | 请求结构转换 | `anthropic_to_openai()`，纯函数，单元测试覆盖各 content 类型 |
| `src-tauri/src/proxy/translate/response.rs` | 响应结构转换 | `openai_to_anthropic()`，纯函数，单元测试覆盖 tool_calls/finish_reason 映射 |
| `src-tauri/src/proxy/translate/stream.rs` | 流式 SSE 转换 | `create_anthropic_sse_stream()`，async-stream，缓冲工具调用状态 |

### 修改现有文件

| 文件 | 修改内容 | 影响范围 |
|------|----------|----------|
| `src-tauri/src/proxy/mod.rs` | `pub mod translate;` 新增子模块声明 | 无运行时影响 |
| `src-tauri/src/proxy/error.rs` | 新增 `TransformError(String)` 变体，`IntoResponse` 返回 422 | ProxyError 的 HTTP 映射语义变化 |
| `src-tauri/src/proxy/handler.rs` | 在步骤 C-D 间插入转换分支，步骤 J 插入响应/流转换分支 | 核心修改，影响所有请求路径（但 Anthropic 直传路径代码逻辑不变） |
| `src-tauri/Cargo.toml` | 可能需要 `async-stream` + `bytes` crate（如不在 reqwest 依赖树中） | 构建依赖 |

**不需要修改的文件：**

| 文件 | 原因 |
|------|------|
| `proxy/state.rs` | `UpstreamTarget.protocol_type` 已存在，转换决策直接读取即可 |
| `proxy/server.rs` | 路由构建不变（`/health` + fallback `proxy_handler`） |
| `provider.rs` | `ProtocolType` 枚举已满足需求 |
| `commands/proxy.rs` | 上游配置逻辑不变，protocol_type 已经正确传入 `UpstreamTarget` |
| `storage/`, `adapter/`, `tray.rs` | 与代理转发层无关联 |

---

## Integration Points

### 转换决策集成点

| 集成位置 | 判断条件 | 处理方式 |
|----------|----------|----------|
| `proxy_handler` 步骤 C 之后 | `upstream.protocol_type == OpenAiCompatible && path == "/v1/messages"` | 调用 `translate::request::anthropic_to_openai()`，重写 path |
| `proxy_handler` 步骤 H 之后 | `needs_translation && is_sse` | 调用 `translate::stream::create_anthropic_sse_stream()` |
| `proxy_handler` 步骤 H 之后 | `needs_translation && !is_sse` | 全量读取响应 body，调用 `translate::response::openai_to_anthropic()` |

### UpstreamTarget 已有字段的使用

```
UpstreamTarget {
    api_key: String,         // 凭据注入（已有逻辑）
    base_url: String,        // 上游 URL 拼接（已有逻辑）
    protocol_type: ProtocolType,  // 【新用途】转换决策依据
}
```

`protocol_type` 字段已存在于 `UpstreamTarget` 且已在步骤 G 中用于凭据注入（Anthropic 用 `x-api-key`，OpenAI 用 `Authorization: Bearer`）。转换决策复用相同字段，无需扩展数据结构。

### 凭据注入与协议转换的交互

转换后请求发往 OpenAI 兼容 Provider，凭据格式必须是 `Authorization: Bearer`（而非 Anthropic 的 `x-api-key`）。**现有步骤 G 已按 `protocol_type` 注入正确格式**，与转换逻辑天然配合，无需额外处理。

---

## Build Order

基于文件依赖关系，推荐以下构建顺序（每步可独立验证）：

```
步骤 1：实现 translate/request.rs（纯函数，最小依赖）
  新增: src-tauri/src/proxy/translate/mod.rs（空模块入口）
  新增: src-tauri/src/proxy/translate/request.rs
    - anthropic_to_openai(body: Value) -> Result<Value, ProxyError>
    - 覆盖：string content，array content（text/image/tool_use/tool_result）
    - 工具调用格式转换：input_schema → parameters，clean_schema()
    - system → messages[0]{role: system}
  验证: cargo test proxy::translate::request（纯单元测试，不需要运行服务器）

步骤 2：实现 translate/response.rs（纯函数，最小依赖）
  新增: src-tauri/src/proxy/translate/response.rs
    - openai_to_anthropic(body: Value) -> Result<Value, ProxyError>
    - 覆盖：text content，tool_calls，function_call（遗留格式）
    - finish_reason 映射：stop→end_turn，length→max_tokens，tool_calls→tool_use
    - usage 映射：prompt_tokens→input_tokens，completion_tokens→output_tokens
    - cache tokens：prompt_tokens_details.cached_tokens → cache_read_input_tokens
  验证: cargo test proxy::translate::response

步骤 3：实现 translate/stream.rs（流适配器，需要 async-stream）
  新增: src-tauri/src/proxy/translate/stream.rs
    - create_anthropic_sse_stream(stream: impl Stream<...>) -> impl Stream<...>
    - 状态机：message_id，model，content_block_index，tool_blocks 缓冲
    - 处理：reasoning delta（thinking block），text delta，tool_calls delta
    - 工具调用延迟发送：pending_args 直到 id+name 就绪
    - finish_reason 处理：关闭所有块，发送 message_delta + message_stop
    - 错误处理：上游流错误 → SSE error 事件
  验证: cargo test proxy::translate::stream（使用 tokio::test + 模拟 SSE bytes）

步骤 4：扩展 ProxyError + 修改 mod.rs
  修改: src-tauri/src/proxy/error.rs
    - 新增 TransformError(String) 变体
    - IntoResponse 映射：422 Unprocessable Entity
  修改: src-tauri/src/proxy/mod.rs
    - pub mod translate;
  验证: cargo build（确保编译通过）

步骤 5：修改 handler.rs 插入转换逻辑（核心集成）
  修改: src-tauri/src/proxy/handler.rs → proxy_handler()
    - 步骤 C 后：检查 needs_translation（protocol_type + path 双重判断）
    - 条件调用 translate::request::anthropic_to_openai()，重写 path
    - 步骤 H 后：检查 is_sse（Content-Type: text/event-stream）
    - 条件分支：流式转换 / 非流式转换 / 直传
  验证: cargo test proxy::handler（现有测试全通过）
        cargo test proxy（全模块测试通过）

步骤 6：端到端集成测试
  新增: 在 proxy/mod.rs 或 handler.rs 的 tests 模块增加 e2e 测试
    - 启动 mock OpenAI 兼容上游（接收 Chat Completions 请求）
    - 向代理发送 Anthropic Messages API 请求（protocol_type = OpenAiCompatible）
    - 验证上游收到正确的 OpenAI 格式请求
    - 验证代理响应符合 Anthropic Messages API 格式
    - 流式版本：验证 SSE 事件序列正确
  验证: cargo test（全部 lib tests 通过，基准线 221 tests）
```

**依赖约束：**
- 步骤 1、2、3 相互独立，可并行
- 步骤 4 依赖步骤 1-3（需要 TransformError 在转换函数中使用）
- 步骤 5 依赖步骤 1-4
- 步骤 6 依赖步骤 5

---

## Anti-Patterns

### Anti-Pattern 1: 在流式路径缓冲完整响应后再转换

**What people do:** 等 SSE 流全部接收完毕，拼接成完整 JSON，调用非流式转换函数，然后"模拟"成 SSE 一次性输出。

**Why it's wrong:**
- Claude Code 对第一个 SSE token 的到达时间敏感（用于显示响应进度）
- 缓冲完整响应会使流式请求退化为阻塞请求，用户体验极差
- 大型响应（长文本 + 多工具调用）可能占用大量内存

**Do this instead:** 使用 `create_anthropic_sse_stream()` 逐 chunk 转换，token 到达即转发，不缓冲完整响应。

### Anti-Pattern 2: 在非流式路径使用流式转换

**What people do:** 统一使用流式转换处理所有响应（无论 `stream` 参数是否为 true）。

**Why it's wrong:**
- 非流式响应没有 SSE 格式，强行走流式解析路径会失败（JSON 不是 `data: {...}\n\n` 格式）
- 非流式响应 body 通常需要完整读取才能正确转换（如 `usage` 字段在最后）

**Do this instead:** 根据响应的 `Content-Type` header 判断：`text/event-stream` 走流式转换，`application/json` 走非流式转换。

### Anti-Pattern 3: 对所有 OpenAiCompatible 请求都转换路径

**What people do:** 只根据 `protocol_type == OpenAiCompatible` 决定是否转换，对该 Provider 的所有请求（包括 `/v1/models`、`/v1/token_count`）都走转换路径。

**Why it's wrong:**
- Claude Code 发送的不仅仅是 `/v1/messages` 请求，还有 `/v1/complete`、`/v1/token_count` 等
- 这些端点无需协议转换（OpenAI 兼容 Provider 通常也支持这些端点或可直传）
- 误转换会导致请求格式错误

**Do this instead:** 转换决策使用双重判断：`protocol_type == OpenAiCompatible && path == "/v1/messages"`。其他路径直传。

### Anti-Pattern 4: 复用 cc-switch 的 ProviderAdapter trait 架构

**What people do:** 引入 `ProviderAdapter` trait（参考 cc-switch），为每种协议定义一个 adapter，handler 通过动态分发调用。

**Why it's wrong:**
- v2.2 只做 Anthropic→OpenAI 单向，不需要多协议适配层
- trait 对象（`Box<dyn ProviderAdapter>`）引入不必要的动态分发开销
- cc-switch 的 adapter 与 Provider 数据库模型深度耦合，而我们的 Provider 数据模型更简洁

**Do this instead:** 纯函数 + 条件分支，直接在 handler 内判断。待 v3.0 需要 Gemini/其他协议时再考虑引入抽象。

### Anti-Pattern 5: 转换错误返回 502

**What people do:** 将所有代理错误（含转换失败）都返回 502 Bad Gateway。

**Why it's wrong:**
- 502 的语义是"上游不可达"，转换失败是代理内部错误
- Claude Code 对不同错误码有不同的重试和错误展示策略，错误码不准确影响调试

**Do this instead:** `TransformError` 返回 422 Unprocessable Entity（代理无法处理客户端发来的格式），与 502（上游不可达）和 503（无上游配置）区分。

---

## Scaling Considerations

这是桌面应用本地代理，不存在多用户并发压力。唯一关注点是单请求的内存和延迟：

| 关注点 | 当前设计 | 上限 |
|--------|----------|------|
| 非流式请求 body 内存 | 读 body bytes + 解析 JSON + 序列化 | 200MB 限制（继承现有） |
| 流式响应内存 | 逐 chunk 转换，不缓冲 | 单 chunk ~几 KB，无积累 |
| 工具调用 pending_args 缓冲 | 仅在 id/name 就绪前缓冲参数字符串 | 实际工具参数通常 <1KB |
| 转换延迟 | 单次 JSON 解析+序列化，通常 <1ms | 不影响用户体验 |

---

## Sources

- **现有代码库分析（HIGH confidence）：**
  - `/src-tauri/src/proxy/handler.rs` — 现有请求管道步骤
  - `/src-tauri/src/proxy/state.rs` — `UpstreamTarget.protocol_type` 字段已存在
  - `/src-tauri/src/proxy/error.rs` — 现有错误类型结构
  - `/src-tauri/src/provider.rs` — `ProtocolType` 枚举（Anthropic / OpenAiCompatible）

- **cc-switch 参考实现（HIGH confidence，直接代码分析）：**
  - `cc-switch/src-tauri/src/proxy/providers/transform.rs` — `anthropic_to_openai()` / `openai_to_anthropic()` 完整实现，含测试
  - `cc-switch/src-tauri/src/proxy/providers/streaming.rs` — `create_anthropic_sse_stream()` 完整实现，含工具调用缓冲逻辑

- **转换实现细节（HIGH confidence，来自 cc-switch 代码）：**
  - 工具调用延迟发送（pending_args）：streaming.rs 第 280-347 行
  - finish_reason 映射表：stop→end_turn，length→max_tokens，tool_calls→tool_use
  - usage cache tokens 映射：`prompt_tokens_details.cached_tokens` → `cache_read_input_tokens`
  - schema 清理（移除 `"format": "uri"`）：`clean_schema()` 函数

---
*Architecture research for: CLIManager v2.2 协议转换*
*Researched: 2026-03-14*
