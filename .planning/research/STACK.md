# Stack Research

**Domain:** Anthropic → OpenAI 协议转换（axum 0.8 代理层扩展，v2.2 里程碑）
**Researched:** 2026-03-14
**Confidence:** HIGH

---

## 里程碑范围说明

本文档只覆盖 v2.2 协议转换所需的**增量**栈变化。以下技术已在 v2.0/v2.1 验证，**不重复研究**：

- Tauri 2.10, React 19, Vite 7, shadcn/ui, Tailwind CSS v4, i18next
- serde, serde_json (preserve_order), toml_edit, notify, uuid, chrono
- reqwest 0.12 (+stream), axum 0.8, tower-http 0.6, tokio

---

## 核心发现摘要

v2.2 协议转换**不需要引入任何新 crate**。所有能力可由以下手段实现：

1. `serde_json::Value` —— 动态 JSON 转换（已有）
2. `bytes` —— SSE 流 item 类型（**已作为传递依赖锁定在 Cargo.lock**，仅需显式声明）
3. `futures` —— 流组合子 StreamExt（**已作为传递依赖锁定在 Cargo.lock**，仅需显式声明）
4. `reqwest` bytes_stream() + `axum` Body::from_stream() —— 流式代理管道（已有，v2.0 已用）

cc-switch 参考实现（`transform.rs` 775行 + `streaming.rs` 744行）用同款方案完整实现了 Anthropic ↔ OpenAI Chat Completions 双向协议转换，包含工具调用、图片/多模态、流式 SSE。

**Cargo.toml 只需新增 2 行显式声明。**

---

## Recommended Stack

### 新增显式依赖（共 2 项）

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `bytes` | `"1"` | SSE 流转换中的 `Bytes` 类型——axum Body::from_stream 和 reqwest bytes_stream 的共同 item 类型 | tokio-rs 生态标配；已作为传递依赖锁定为 1.11.1（验证：`Cargo.lock` 中 `name = "bytes" / version = "1.11.1"`）；cc-switch 同款（`bytes = "1.5"`）；无额外下载开销 |
| `futures` | `"0.3"` | `Stream`、`StreamExt`、`stream::once` 等流组合子——SSE 转换层逐 chunk 映射所需 | futures-rs 生态标准库；已作为传递依赖锁定为 0.3.32（验证：`Cargo.lock` 中 `name = "futures" / version = "0.3.32"`）；cc-switch 同款（`futures = "0.3"`）；`dev-dependencies` 中已声明，升为 `dependencies` 无风险 |

### 无需引入的新 crate

| 需要的能力 | 现有满足方案 |
|------------|-------------|
| JSON 请求体解析与序列化 | `serde_json` 已有，`Value` 动态访问足以应对所有字段转换 |
| HTTP 请求构建与发送 | `reqwest` 0.12 已有，`.body(bytes)`、`.bytes_stream()` v2.0 已用 |
| 流式 SSE 响应写出 | `axum::body::Body::from_stream()` v2.0 proxy 中已用 |
| 异步运行时 | `tokio` 已有 |
| OpenAI API 客户端 | **不需要**（见「不使用」） |
| Anthropic API 客户端 | **不需要**（见「不使用」） |

---

## Cargo.toml 变更（最小化）

```toml
# src-tauri/Cargo.toml —— 在现有 [dependencies] 中追加两行
bytes = "1"
futures = "0.3"
```

```toml
# [dev-dependencies] 中已有 futures = "0.3"，保持不变
```

---

## 协议转换层技术方案

### 请求体转换（非流式）

使用已有的 `axum::body::to_bytes` 读全量请求体，`serde_json::from_slice` 解析为 `Value`，逐字段映射转换后 `serde_json::to_vec` 序列化，`reqwest::RequestBuilder::body(bytes)` 发出。

**关键字段映射（Anthropic Messages → OpenAI Chat Completions）：**

| Anthropic 字段 | OpenAI 字段 | 转换规则 |
|---------------|------------|---------|
| `system` (str 或 array) | `messages[0]` with `role: "system"` | 字符串直接用，array 取 `text` 字段 |
| `messages[].content` (array of blocks) | `messages[].content` (string 或 parts array) | text 块 → 文本；image 块 → `image_url` data URI；tool_use 块 → `tool_calls`；tool_result 块 → 新 `role: "tool"` 消息 |
| `tools[].input_schema` | `tools[].function.parameters` | 包裹为 `{"type": "function", "function": {...}}` |
| `max_tokens` (必填) | `max_tokens` (可选) | 直接透传 |
| URL 路径 `/v1/messages` | `/v1/chat/completions` | handler 层重写路径 |

### 响应体转换（非流式）

上游返回 OpenAI 格式响应，用 `reqwest::Response::bytes().await` 全量读取，`serde_json::from_slice` 解析，字段映射后序列化为 Anthropic 格式，`axum::Body::from(bytes)` 返回。

**关键字段映射（OpenAI Chat Completions → Anthropic Messages）：**

| OpenAI 字段 | Anthropic 字段 | 转换规则 |
|------------|---------------|---------|
| `choices[0].message.content` | `content[].type: "text"` | 包裹为 content block |
| `choices[0].message.tool_calls` | `content[].type: "tool_use"` | `function.arguments` (string) → `input` (parsed JSON) |
| `choices[0].finish_reason` | `stop_reason` | `"stop"` → `"end_turn"`；`"tool_calls"` → `"tool_use"` |
| `usage.prompt_tokens` | `usage.input_tokens` | 直接映射 |
| `usage.completion_tokens` | `usage.output_tokens` | 直接映射 |

### 流式 SSE 转换

这是唯一需要 `bytes` + `futures` 的场景：

```
reqwest bytes_stream()
  → Stream<Item = Result<Bytes, reqwest::Error>>
  → 状态机：逐行解析 SSE，缓冲工具调用 delta，生成 Anthropic 格式事件
  → Stream<Item = Result<Bytes, io::Error>>
  → axum Body::from_stream()
```

**OpenAI SSE chunk 结构：**
```
data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","choices":[{"delta":{"content":"hello"},"finish_reason":null}]}
```

**对应 Anthropic SSE 事件序列：**
```
event: message_start
data: {"type":"message_start","message":{"id":"msg_xxx","type":"message","role":"assistant","content":[],"model":"..."}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hello"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":N}}

event: message_stop
data: {"type":"message_stop"}
```

**状态机需跟踪（参考 cc-switch streaming.rs）：**
- `message_id`、`model`（从第一个 chunk 的 `id`/`model` 提取）
- `has_sent_message_start`（第一个 content chunk 前发送 message_start + content_block_start）
- `tool_call_accumulators`（按 `index` 聚合 tool call delta，`finish_reason: "tool_calls"` 时才输出）
- `content_index`（文本 block 0，工具调用 block 从 1 开始）

---

## 与现有 proxy 的集成点

| 现有文件 | v2.2 变化方向 |
|----------|--------------|
| `proxy/handler.rs` | 核心改动：检测上游 `protocol_type == OpenAiCompatible` 时进入转换路径：转换请求体 → 改写 URL 路径 → 根据 `is_stream` 分叉处理响应（流式/非流式） |
| `proxy/state.rs` | `UpstreamTarget` 目前已有 `protocol_type: ProtocolType` 字段，OpenAiCompatible 分支触发转换，**可能无需改结构** |
| `provider.rs` | `ProtocolType::OpenAiCompatible` 已存在，无需新枚举值 |
| 新增 `proxy/translate/mod.rs` | 协议转换模块入口 |
| 新增 `proxy/translate/request.rs` | `anthropic_to_openai(Value) -> Result<Value>` 纯函数 |
| 新增 `proxy/translate/response.rs` | `openai_to_anthropic(Value) -> Result<Value>` 纯函数 |
| 新增 `proxy/translate/streaming.rs` | OpenAI SSE stream → Anthropic SSE stream 转换器 |

---

## 不使用

| 避免引入 | 原因 | 替代方案 |
|----------|------|---------|
| `async-openai` / `openai` crate | 面向 API 调用方设计的 typed client，无法用于协议桥接；引入大量无用依赖（reqwest、tokio-tungstenite 等）| 手写 `serde_json::Value` 字段映射 |
| `anthropic-rs` / `anthropic` crate | 同上，且对 Messages API streaming 的支持不完整 | 手写转换函数 |
| `async-stream` crate | cc-switch 使用它做流生成器宏，但 `futures::stream::StreamExt` 组合子（`.flat_map()`、`.filter_map()`）完全可替代，不值得多一个依赖 | `futures::StreamExt` |
| `hyper` 直接依赖 | axum 0.8 已包含 hyper，不需要直接操作底层 | axum 高层 API |
| `regex` crate | SSE 行解析用标准库 `str::strip_prefix("data: ")` 和 `str::trim()` 足够 | 标准库字符串方法 |
| `base64` crate | v2.2 范围内图片转换只需将 Anthropic base64 data 直接拼成 `data:{media_type};base64,{data}` URI，无需 encode/decode | 字符串拼接 |

---

## 版本兼容性

| Package | 锁定版本 | 兼容性说明 |
|---------|---------|-----------|
| `bytes = "1"` | 1.11.1 (Cargo.lock 已有) | 与 axum 0.8、reqwest 0.12、tokio 1 完全兼容，同属 tokio-rs 生态统一维护 |
| `futures = "0.3"` | 0.3.32 (Cargo.lock 已有) | 与 tokio 1 完全兼容；`dev-dependencies` 中已声明为 `"0.3"`，升为 `dependencies` 使用相同版本说明符，无冲突 |
| `serde_json = "1"` | 已有（preserve_order feature） | 协议转换层直接复用，字段遍历顺序对转换逻辑无影响 |

---

## 替代方案考量

| 推荐 | 替代 | 不选替代的原因 |
|------|------|---------------|
| 手写 `serde_json::Value` 转换 | 引入 `openai` / `anthropic` typed SDK | SDK 不适合桥接场景；cc-switch 775行 transform.rs 证明手写方案完整可行且可测试 |
| `futures::StreamExt` 组合子 | `async-stream` 生成器宏 | `async-stream` 需额外依赖；`StreamExt::flat_map` + 状态封装已足够，cc-switch streaming.rs 744行验证 |
| 全量读取非流式响应再转换 | 流式读取非流式响应 | 非流式 OpenAI 响应体通常 < 10KB，全量读取简单可靠；`reqwest::Response::bytes().await` 一行即可 |
| 新增 `proxy/translate/` 子模块 | 直接在 `handler.rs` 内写转换逻辑 | 转换代码量（~1000行估算）放在 handler.rs 会使其过于臃肿；独立模块便于单元测试（参考 v2.0 221个 lib tests 模式） |

---

## cc-switch 参考实现验证

cc-switch 的协议转换实现经研究验证如下（**仅供参考，不受局限**）：

| 文件 | 行数 | 实现内容 | 可参考点 |
|------|------|---------|---------|
| `providers/transform.rs` | 775 | `anthropic_to_openai()` + `openai_to_anthropic()` 纯函数，`serde_json::Value` 操作 | 字段映射逻辑、schema 清理、工具调用转换 |
| `providers/streaming.rs` | 744 | OpenAI Chat Completions SSE → Anthropic SSE 状态机，使用 `bytes` + `futures::StreamExt` | 状态机结构、SSE 行解析、tool call delta 聚合 |
| `providers/models/anthropic.rs` | 107 | Anthropic 类型定义（参考，不直接用） | 数据模型边界理解 |
| `providers/models/openai.rs` | 116 | OpenAI 类型定义（参考，不直接用） | 数据模型边界理解 |

CLIManager v2.2 应**直接手写 `serde_json::Value` 转换**，不照搬 cc-switch 的结构体方式（cc-switch 的 typed structs 限制了对未知字段的兼容性）。

---

## Sources

- 当前项目 `src-tauri/Cargo.lock` — `bytes` 1.11.1, `futures` 0.3.32 已作为传递依赖存在（HIGH）
- 当前项目 `src-tauri/Cargo.toml` — 现有依赖清单，确认无重复引入（HIGH）
- cc-switch 参考 `cc-switch/src-tauri/src/proxy/providers/transform.rs` — 775行，`serde_json::Value` 方案完整可行性验证（HIGH）
- cc-switch 参考 `cc-switch/src-tauri/src/proxy/providers/streaming.rs` — 744行，`bytes` + `futures::StreamExt` SSE 转换方案验证（HIGH）
- cc-switch `cc-switch/src-tauri/Cargo.toml` — `bytes = "1.5"`, `futures = "0.3"` 依赖选型交叉验证（HIGH）
- [crates.io/crates/bytes](https://crates.io/crates/bytes) — 当前稳定版 1.11.1（HIGH）
- [crates.io/crates/futures](https://crates.io/crates/futures) — 当前稳定版 0.3.32（HIGH）
- 当前项目 `proxy/handler.rs` — 现有 `Body::from_stream(upstream_resp.bytes_stream())` 模式，v2.2 流式转换的直接扩展点（HIGH）

---

*Stack research for: Anthropic→OpenAI 协议转换，axum 0.8 代理层扩展（v2.2）*
*Researched: 2026-03-14*
