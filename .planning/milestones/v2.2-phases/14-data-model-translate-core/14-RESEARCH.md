# Phase 14: 数据模型 + 转换核心 - Research

**Researched:** 2026-03-14
**Domain:** Rust 协议转换（Anthropic ↔ OpenAI），serde 数据模型扩展，流式 SSE 状态机
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**模型映射字段设计**
- 新增 `upstream_model: Option<String>` — 代理转换使用的默认目标模型名
- 新增 `upstream_model_map: Option<HashMap<String, String>>` — 任意个模型名映射对（源模型名 → 目标模型名）
- 两个新字段与现有 `model`/`model_config` **完全独立** — 现有字段是 CLI 配置文件用（surgical patch），新字段仅代理转换用
- 命名用 `upstream_` 前缀，明确区分用途
- 映射优先级：精确匹配 → upstream_model → 保留原模型名

**ProtocolType 扩展为三变体**
- `Anthropic` — 不变
- `OpenAiChatCompletions` — 原 `OpenAiCompatible`，Chat Completions API
- `OpenAiResponses` — 新增，Responses API
- 旧 JSON 中的 `"open_ai_compatible"` 通过 serde alias 向前兼容，反序列化为 `OpenAiChatCompletions`
- **不再需要**单独的 `upstream_api_format` 字段 — protocol_type 已完全描述上游协议

**base_url 路径策略**
- 放宽 base_url 校验，允许包含路径（如 `https://openrouter.ai/api/v1`）
- 端点重写以 `/v1` 为锚点智能去重：
  - 无路径：补全完整端点（`/v1/chat/completions`）
  - 路径含 `/v1`：替换 `/v1` 之后的部分为目标端点后缀（`/chat/completions` 或 `/responses`）
  - 路径含 `/v1/responses`：视 ProtocolType 替换为正确端点
- 修改 `normalize_origin_base_url()` 或新增变体函数

**转换降级策略**
- 已知不兼容内容（thinking blocks, BatchTool）→ 静默丢弃
- 可能兼容的内容（cache_control, 未知请求字段）→ 透传（OpenAI Provider 通常忽略）
- 不兼容 JSON Schema 字段（`format: "uri"` 等）→ 递归清理移除
- Claude 有裁量权根据具体内容类型决定丢弃还是透传

### Claude's Discretion
- 转换函数内部结构设计（模块拆分、辅助函数命名）
- 流式 SSE 状态机的具体状态定义和转换逻辑
- 单元测试用例的组织方式
- bytes / futures crate 的具体使用方式

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| MODL-01 | Provider 数据模型支持存储默认目标模型名（缺省映射） | `Provider` struct 新增 `upstream_model: Option<String>`，`#[serde(default)]` + `#[serde(skip_serializing_if)]` 保证向前兼容 |
| MODL-02 | Provider 数据模型支持存储任意个模型名映射对 | `upstream_model_map: Option<HashMap<String, String>>`，同样使用 serde optional 模式 |
| REQT-01 | 系统提示正确转换（顶层 `system` 字段 → messages 数组首条 system 消息） | cc-switch transform.rs 第 23-39 行提供字符串和数组两种格式的完整实现蓝图 |
| REQT-02 | 消息数组格式转换（text/tool_use/tool_result content blocks → OpenAI 对应格式） | cc-switch transform.rs 第 108-235 行 `convert_message_to_openai()` 涵盖所有 content block 类型 |
| REQT-03 | 工具定义转换（`input_schema` → `function.parameters`，添加 `type:"function"` 包装） | cc-switch transform.rs 第 71-94 行（含 BatchTool 过滤和 clean_schema 调用） |
| REQT-04 | 端点重写（`/v1/messages` → `/v1/chat/completions`） | 需修改 `normalize_origin_base_url()` 或新增变体；锚点智能去重策略已在 CONTEXT.md 锁定 |
| REQT-05 | 图片/多模态内容转换（base64 content block → `image_url` data URL） | cc-switch transform.rs 第 147-158 行（`data:{media_type};base64,{data}` 格式） |
| REQT-06 | JSON Schema 清理（移除 OpenAI 不兼容的 `format` 字段等） | cc-switch transform.rs 第 237-257 行 `clean_schema()` 递归清理实现 |
| REQT-07 | cache_control 字段透传到 OpenAI 请求 | cc-switch transform.rs 已实现透传策略（system/text/tool 三处均透传） |
| REQT-08 | 标准参数透传与重命名（`stop_sequences` → `stop` 等） | cc-switch transform.rs 第 53-68 行（max_tokens/temperature/top_p/stop/stream） |
| RESP-01 | 非流式文本响应转换（choices → content blocks） | cc-switch transform.rs 第 278-305 行，处理 text/output_text/refusal 内容类型 |
| RESP-02 | 非流式工具调用响应转换（`tool_calls` → `tool_use` content blocks，arguments 反序列化） | cc-switch transform.rs 第 313-336 行（含 legacy `function_call` 兼容） |
| RESP-03 | stop_reason/finish_reason 映射 | cc-switch transform.rs 第 369-384 行（stop→end_turn, length→max_tokens, tool_calls→tool_use, content_filter→end_turn） |
| RESP-04 | usage 字段映射（prompt_tokens→input_tokens, completion_tokens→output_tokens） | cc-switch transform.rs 第 387-415 行（含 cache token 映射） |
| RESP-05 | 错误响应（4xx/5xx）直接透传，不经转换处理 | handler 层按 HTTP status code 判断，translate 层不涉及；request.rs/response.rs 只处理成功路径 |
| STRM-01 | 文本 delta 事件序列转换（完整 Anthropic SSE 事件序列） | cc-switch streaming.rs 第 219-263 行（message_start→content_block_start→text_delta→content_block_stop→message_delta→message_stop） |
| STRM-02 | 工具调用流式转换，含 Deferred Start pending buffer | cc-switch streaming.rs 第 279-347 行（核心复杂点，id/name 未就绪时缓冲 arguments） |
| STRM-03 | 多工具并发流式支持（按 index 独立追踪每个工具调用状态） | cc-switch streaming.rs `ToolBlockState` + `HashMap<usize, ToolBlockState>` 按 OpenAI index 路由 |
| STRM-04 | 流结束事件映射（finish_reason → message_delta stop_reason + message_stop） | cc-switch streaming.rs 第 396-511 行（含 late-start 保障机制） |
</phase_requirements>

---

## Summary

Phase 14 需要在现有的 CLIManager Rust 后端中实现三个独立的纯函数/流适配器模块，并完成 Provider 数据模型的两个字段扩展。核心技术域为 serde 序列化模式、`serde_json::Value` 动态 JSON 操作、以及基于 `async-stream` 宏的异步流式状态机。

参考蓝图已经非常完整：cc-switch 的 `transform.rs`（775 行）和 `streaming.rs`（744 行）涵盖了绝大部分需要实现的逻辑。CLIManager 版本的实现应在参考 cc-switch 的同时做出以下简化：去掉 `cache_key` 注入参数（Out of Scope）、去掉 `log::` 宏（或保留）、函数签名去掉 `ProxyError` 依赖（使用内联 Error 或 anyhow）。

关键发现：`async-stream` crate 尚未出现在 CLIManager 的 Cargo.lock 中，需要显式添加到 Cargo.toml。`bytes`（1.11.1）和 `futures`（0.3.32）已作为传递依赖锁定，只需声明为直接依赖即可。

**Primary recommendation:** 严格按照 Wave 1（数据模型）→ Wave 2（三路并行）的执行顺序，Wave 2 的三个 Plan 都依赖 MODL-01/02 完成后的 provider.rs 变更，以免并行实现时结构体定义不稳定。

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde` | 1.x（已有） | 序列化/反序列化数据模型 | Rust 事实标准，项目已使用 |
| `serde_json` | 1.x（已有） | JSON 动态操作（`Value`） | 转换函数使用 `Value` 动态映射，无需新 typed struct |
| `bytes` | 1.11.1（传递依赖） | 流式字节缓冲（`Bytes`） | reqwest/axum 生态标准，已锁定版本 |
| `futures` | 0.3.32（传递依赖） | `Stream` trait + `StreamExt` | Rust 异步流标准，已锁定版本 |
| `async-stream` | 0.3（需新增） | `stream!` 宏简化 SSE 状态机 | cc-switch 已验证方案，无等价简洁替代 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio` | 1.x（已有） | `tokio::pin!` + `stream.next().await` | stream.rs 中驱动异步流 |
| `std::collections::HashMap` | std | 工具调用按 index 状态追踪 | STRM-03 多工具并发 |
| `std::collections::HashSet` | std | 追踪已打开的 tool block index | STRM-04 关闭阶段 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `async-stream` 宏 | 手写 `Stream` impl（Pin + poll） | 手写代码量极大，SSE 状态机逻辑本就复杂，无必要 |
| `serde_json::Value` | 强类型结构体 + serde | 强类型需要全量定义 Anthropic+OpenAI schema，未知字段自动丢失（不兼容 REQT-07 透传需求） |
| `futures::stream` | `tokio-stream` | tokio-stream 未在 lock 中，引入额外依赖；futures 已满足需求 |

### Installation

```toml
# src-tauri/Cargo.toml — 新增显式依赖
bytes = "1"
futures = "0.3"
async-stream = "0.3"
```

> `bytes` 和 `futures` 从 dev-dependencies 提升为 dependencies（或同时存在两处），`async-stream` 为全新直接依赖。

---

## Architecture Patterns

### Recommended Project Structure

```
src-tauri/src/
├── provider.rs           # Wave 1：ProtocolType 扩展 + Provider 新字段
├── proxy/
│   ├── mod.rs
│   ├── state.rs          # UpstreamTarget 扩展（model_map 信息）
│   ├── handler.rs        # Phase 15 才修改
│   ├── server.rs
│   ├── error.rs          # 可能新增 TransformError variant
│   └── translate/        # Wave 2：新建子模块
│       ├── mod.rs        # pub use request::...; pub use response::...; pub use stream::...
│       ├── request.rs    # anthropic_to_openai() + clean_schema() + convert_message_to_openai()
│       ├── response.rs   # openai_to_anthropic()
│       └── stream.rs     # create_anthropic_sse_stream() + ToolBlockState + 辅助结构体
```

### Pattern 1: serde Optional 新字段（向前兼容）

**What:** 利用 `#[serde(default)]` + `#[serde(skip_serializing_if = "Option::is_none")]` 实现新字段对旧 JSON 文件的向前兼容。
**When to use:** Provider struct 所有新增字段必须采用此模式。

```rust
// 参考：provider.rs 中 notes 和 model_config 字段的现有实现
#[serde(default)]
pub model_config: Option<ModelConfig>,
#[serde(skip_serializing_if = "Option::is_none")]
pub notes: Option<String>,

// Phase 14 新增字段（相同模式）：
#[serde(default, skip_serializing_if = "Option::is_none")]
pub upstream_model: Option<String>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub upstream_model_map: Option<HashMap<String, String>>,
```

### Pattern 2: serde alias 向前兼容（ProtocolType 重命名）

**What:** 使用 `#[serde(rename = "...", alias = "...")]` 将旧序列化名称映射到新 variant。
**When to use:** ProtocolType::OpenAiCompatible 重命名为 OpenAiChatCompletions 时，确保旧 JSON 文件不破坏。

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    Anthropic,
    // 序列化为 "open_ai_chat_completions"，但 "open_ai_compatible" 也能反序列化
    #[serde(alias = "open_ai_compatible")]
    OpenAiChatCompletions,
    OpenAiResponses,
}
```

> **验证需求：** serde 的 `rename_all = "snake_case"` 与 `alias` 组合会将 `OpenAiChatCompletions` 序列化为 `"open_ai_chat_completions"`，同时接受 `"open_ai_compatible"` 反序列化。此行为已由 serde 文档确认（HIGH 信心）。

### Pattern 3: 纯函数 + `serde_json::Value` 动态映射

**What:** 转换函数接受 `Value` 并返回 `Value`（或 `Result<Value, ...>`），不依赖任何应用状态。
**When to use:** request.rs 和 response.rs 中所有转换函数。

```rust
// request.rs 示例签名
pub fn anthropic_to_openai(body: Value) -> Result<Value, TranslateError> { ... }
pub fn clean_schema(schema: Value) -> Value { ... }  // 纯无副作用函数

// response.rs 示例签名
pub fn openai_to_anthropic(body: Value) -> Result<Value, TranslateError> { ... }
```

> 与 cc-switch 不同，Phase 14 不需要 `cache_key: Option<&str>` 参数（Out of Scope）。函数签名更简洁。

### Pattern 4: async-stream 宏驱动 SSE 状态机

**What:** 使用 `async_stream::stream!` 宏在 async 生成器风格中 `yield` SSE 字节块，维护内部可变状态。
**When to use:** stream.rs 中 `create_anthropic_sse_stream()` 函数。

```rust
use async_stream::stream;
use bytes::Bytes;
use futures::stream::Stream;

pub fn create_anthropic_sse_stream(
    upstream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send {
    stream! {
        // 状态变量在此初始化
        let mut buffer = String::new();
        // ...
        tokio::pin!(upstream);
        while let Some(chunk) = upstream.next().await {
            // yield 产出 SSE 事件
            yield Ok(Bytes::from("event: ...\ndata: ...\n\n"));
        }
    }
}
```

### Pattern 5: `_in` 变体隔离文件系统依赖

**What:** 项目已有的测试隔离模式，核心逻辑以 `_in` 为后缀的内部函数实现，公开函数仅做参数准备。
**When to use:** translate 模块中的纯函数不依赖文件系统，因此 `_in` 变体不适用；直接使用纯函数即可。

### Anti-Patterns to Avoid

- **使用 `base_url.trim_end_matches('/') + path`：** 当 base_url 包含路径时（如 `https://openrouter.ai/api/v1`），简单拼接会产生 `/v1/v1/chat/completions`。必须使用锚点去重逻辑。
- **在 translate 层依赖 `AppHandle` 或 `ProxyState`：** 转换函数必须是纯函数，只操作 JSON Value。依赖应用状态会破坏可测试性。
- **对 `finish_reason` 使用穷举 match 不加 wildcard：** OpenAI 兼容 Provider 可能返回非标准 finish_reason，必须有 `other => "end_turn"` 的 fallback。
- **在 `stream!` 宏中使用阻塞操作：** `stream!` 在异步上下文运行，不能调用阻塞 I/O。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SSE 事件生成器 | 手写 `Stream` impl + `Poll` | `async-stream` 宏 | 手写 Pin + Waker 实现极复杂，async-stream 已在 cc-switch 中验证 |
| JSON Schema 递归清理 | 字符串正则替换 | `clean_schema(Value) -> Value` 递归函数 | 正则无法处理嵌套 schema，必须递归遍历 Object |
| finish_reason 映射表 | 枚举序列化 | `match` + 字符串常量 | 映射集合小且明确，match 最清晰，enum 增加无必要的序列化层 |
| arguments JSON 拼接 | 手动字符串拼接 | `pending_args.push_str()` + 延迟 `serde_json::from_str` | 流式 arguments 跨多个 chunk 传输，不能逐 chunk 解析，必须缓冲后再解析 |

**Key insight:** 流式工具调用转换的核心复杂度来自"Deferred Start"问题：OpenAI 的 chunk 中 `id` 和 `name` 不一定在第一个包含该工具的 chunk 中出现，而 Anthropic 的 `content_block_start` 必须同时包含 `id` 和 `name`。任何手写方案都必须独立实现这一缓冲机制。

---

## Common Pitfalls

### Pitfall 1: ProtocolType 重命名序列化破坏旧 Provider 文件

**What goes wrong:** 将 `OpenAiCompatible` 重命名为 `OpenAiChatCompletions` 后，若不加 `alias`，存量 Provider JSON 文件中的 `"open_ai_compatible"` 无法反序列化，程序崩溃。
**Why it happens:** serde 的 `rename_all = "snake_case"` 只处理序列化，不自动接受旧名称。
**How to avoid:** 必须在新 variant 上加 `#[serde(alias = "open_ai_compatible")]`。
**Warning signs:** 测试 `test_protocol_type_openai_compatible_serde` 反序列化步骤失败。

### Pitfall 2: base_url 含路径时端点拼接重复 /v1

**What goes wrong:** `https://openrouter.ai/api/v1` + `/v1/chat/completions` = `https://openrouter.ai/api/v1/v1/chat/completions`（404）。
**Why it happens:** 现有 handler.rs 直接拼接 `base_url + path`，假设 base_url 不含路径。
**How to avoid:** 实现锚点去重：检测 base_url 是否已包含 `/v1`，如有则只追加 `/chat/completions`（端点后缀）。
**Warning signs:** 集成测试中对 openrouter.ai 类 Provider 发起请求时，上游返回 404。

### Pitfall 3: Deferred Start pending buffer 顺序错误

**What goes wrong:** id/name 到达后，pending_args 应先于 immediate_delta 发出。若顺序反转，Anthropic 客户端收到无效的 `input_json_delta`（在 content_block_start 之前）。
**Why it happens:** 逻辑分支处理顺序不正确。
**How to avoid:** 严格按 cc-switch 第 318-346 行的顺序：`should_start` → `pending_after_start` → `immediate_delta`。用单独变量从 mutable borrow 中提取数据，再进行 yield。
**Warning signs:** 单元测试 `test_streaming_delays_tool_start_until_id_and_name_ready` 失败。

### Pitfall 4: 多工具状态共享可变借用冲突

**What goes wrong:** `tool_blocks_by_index` 是 `HashMap<usize, ToolBlockState>`，在 `for tool_call in tool_calls` 循环内同时需要修改 state 并用修改结果 yield。Rust 借用检查器不允许。
**Why it happens:** `entry().or_insert_with()` 返回可变引用，后续 yield 需要 immutable 上下文。
**How to avoid:** cc-switch 的解法（第 280-347 行）：在 mutable borrow 块内提取所有所需数据到局部变量（let tuple），borrow 结束后再 yield。这是此场景的标准 Rust 解法。
**Warning signs:** 编译错误 `cannot borrow ... as immutable because it is also borrowed as mutable`。

### Pitfall 5: tool_result 内容格式不一致

**What goes wrong:** Anthropic 的 `tool_result` content 可以是字符串或数组，转换到 OpenAI `tool` role 消息时，若不统一处理，工具返回值可能以 `[{"type":"text","text":"..."}]` 的数组形式传给 OpenAI，部分 Provider 不接受。
**Why it happens:** Anthropic SDK 允许 tool_result.content 为 string 或 ContentBlock[]。
**How to avoid:** 匹配 `Value::String(s)` 直接使用，其他情况用 `serde_json::to_string()` 序列化（cc-switch transform.rs 第 179-184 行方案）。
**Warning signs:** 工具调用 round-trip 测试中上游返回 400。

### Pitfall 6: `clean_schema` 遗漏嵌套 $defs / definitions

**What goes wrong:** 仅清理 `properties` 和 `items`，遗漏 `$defs` / `definitions` 中的 `format` 字段，某些复杂工具 schema 仍触发上游 400。
**Why it happens:** cc-switch 的 `clean_schema` 当前只处理 `properties` 和 `items`（第 246-254 行）。
**How to avoid:** 视测试结果决定是否扩展。当前实现与 cc-switch 保持一致，处理最常见场景；已知需求仅为 REQT-06 中的 `format: "uri"` 类型。
**Warning signs:** 使用 JSON Schema `$defs` 的工具请求返回 400。

---

## Code Examples

Verified patterns from official sources (cc-switch codebase):

### serde alias 向前兼容

```rust
// Source: provider.rs（Phase 14 实现）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    Anthropic,
    #[serde(alias = "open_ai_compatible")]
    OpenAiChatCompletions,
    OpenAiResponses,
}
```

### Provider 新字段（完整向前兼容模式）

```rust
// Source: provider.rs（Phase 14 实现）
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Provider {
    // ... 现有字段不变 ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_model_map: Option<HashMap<String, String>>,
}
```

### Deferred Start 核心结构（stream.rs）

```rust
// Source: cc-switch/src-tauri/src/proxy/providers/streaming.rs 第 81-87 行
#[derive(Debug, Clone)]
struct ToolBlockState {
    anthropic_index: u32,
    id: String,
    name: String,
    started: bool,
    pending_args: String,
}
```

### finish_reason 映射（response.rs 和 stream.rs 共用）

```rust
// Source: cc-switch/src-tauri/src/proxy/providers/transform.rs 第 372-384 行
fn map_finish_reason(r: &str) -> &'static str {
    match r {
        "stop" => "end_turn",
        "length" => "max_tokens",
        "tool_calls" | "function_call" => "tool_use",
        "content_filter" => "end_turn",
        _ => "end_turn",  // 未知 finish_reason 降级为 end_turn
    }
}
```

### base_url 端点重写逻辑

```rust
// Source: CONTEXT.md 端点重写策略（Phase 14 实现）
/// 将 base_url 与目标端点后缀（如 `/chat/completions`）合并，
/// 以 `/v1` 为锚点避免路径重复。
pub fn build_upstream_url(base_url: &str, endpoint_suffix: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if let Some(v1_pos) = trimmed.find("/v1") {
        // base_url 已含 /v1，替换 /v1 之后的部分
        format!("{}/v1{}", &trimmed[..v1_pos], endpoint_suffix)
    } else {
        // 无路径，直接补全
        format!("{}/v1{}", trimmed, endpoint_suffix)
    }
}
// 示例：
// build_upstream_url("https://openrouter.ai/api/v1", "/chat/completions")
//   => "https://openrouter.ai/api/v1/chat/completions"
// build_upstream_url("https://api.openai.com", "/chat/completions")
//   => "https://api.openai.com/v1/chat/completions"
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `ProtocolType::OpenAiCompatible` | `ProtocolType::OpenAiChatCompletions` + `OpenAiResponses` | Phase 14 | 更准确描述上游 API 格式；需要 serde alias 保持向前兼容 |
| base_url 仅允许 origin（无路径） | base_url 允许包含路径前缀 | Phase 14 | 支持 openrouter.ai、together.ai 等需要路径的 Provider |
| 无转换层（直接透传） | translate/ 纯函数转换层 | Phase 14 | 解耦格式转换与请求转发，可独立单元测试 |

**Deprecated/outdated:**
- `normalize_origin_base_url()` 中拒绝路径的校验规则：Phase 14 需要放宽或新增变体（用于代理转发场景，不用于用户输入校验）

---

## Open Questions

1. **`TranslateError` 独立 enum 还是复用 `ProxyError`？**
   - What we know: cc-switch 使用 `ProxyError::TransformError(String)`。CLIManager 的 `ProxyError` 目前没有此 variant。
   - What's unclear: translate 层是否应该与 proxy error 解耦（translate 模块独立 Error 类型 → 在 handler 层转换为 ProxyError）？
   - Recommendation: 在 `proxy/error.rs` 的 `ProxyError` 中新增 `TranslateError(String)` variant，与 cc-switch 保持一致，避免引入独立 crate。

2. **`normalize_origin_base_url()` 是修改还是新增变体？**
   - What we know: 现有函数在 provider.rs 中，有多个测试覆盖"拒绝路径"的行为。
   - What's unclear: 修改现有函数会破坏现有测试；新增变体（如 `build_proxy_url()`）更安全但分散了逻辑。
   - Recommendation: 新增 `build_proxy_endpoint_url(base_url: &str, endpoint_suffix: &str) -> String` 放在 `proxy/translate/request.rs`，不修改现有 `normalize_origin_base_url()`。

3. **`UpstreamTarget` 是否需要在 Phase 14 携带 model_map 信息？**
   - What we know: CONTEXT.md `code_context` 提到 UpstreamTarget 需要扩展。MODL-03（实际使用模型映射）在 Phase 15，不在 Phase 14。
   - What's unclear: translate 函数的签名需要接收 model_name 参数，还是调用方在调用前做映射再传入？
   - Recommendation: Phase 14 的 `anthropic_to_openai()` 直接透传 body 中的 `model` 字段（不做映射），模型名映射逻辑由 Phase 15 的 handler 在调用转换函数之前处理。这样 translate 层保持纯粹。

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust 内置 `cargo test`（rustc test harness） |
| Config file | `src-tauri/Cargo.toml`（`[dev-dependencies]`） |
| Quick run command | `cargo test --package cli-manager translate` |
| Full suite command | `cargo test --package cli-manager` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| MODL-01 | `upstream_model` 字段序列化/反序列化/缺省值 | unit | `cargo test --package cli-manager provider::tests::test_upstream_model` | ❌ Wave 0 |
| MODL-02 | `upstream_model_map` 字段序列化/反序列化/缺省值 | unit | `cargo test --package cli-manager provider::tests::test_upstream_model_map` | ❌ Wave 0 |
| MODL-01+02 | 旧 JSON（无新字段）向前兼容不崩溃 | unit | `cargo test --package cli-manager provider::tests::test_new_fields_backward_compat` | ❌ Wave 0 |
| REQT-01 | system string → system message；system array → multiple system messages | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ Wave 0 |
| REQT-02 | text/tool_use/tool_result/image content blocks 转换 | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ Wave 0 |
| REQT-03 | tools `input_schema` → `function.parameters` + BatchTool 过滤 | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ Wave 0 |
| REQT-04 | base_url 含路径时端点去重拼接 | unit | `cargo test --package cli-manager proxy::translate::request::tests::test_build_proxy_endpoint_url` | ❌ Wave 0 |
| REQT-05 | base64 image → `image_url` data URL | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ Wave 0 |
| REQT-06 | `clean_schema` 递归移除 format 字段 | unit | `cargo test --package cli-manager proxy::translate::request::tests::test_clean_schema` | ❌ Wave 0 |
| REQT-07 | cache_control 透传（system/text/tool） | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ Wave 0 |
| REQT-08 | stop_sequences→stop、max_tokens 等参数映射 | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ Wave 0 |
| RESP-01 | 文本响应 choices → content blocks | unit | `cargo test --package cli-manager proxy::translate::response::tests` | ❌ Wave 0 |
| RESP-02 | 工具调用响应转换（含 legacy function_call） | unit | `cargo test --package cli-manager proxy::translate::response::tests` | ❌ Wave 0 |
| RESP-03 | finish_reason 穷举映射 | unit | `cargo test --package cli-manager proxy::translate::response::tests` | ❌ Wave 0 |
| RESP-04 | usage 字段重命名（含 cache tokens） | unit | `cargo test --package cli-manager proxy::translate::response::tests` | ❌ Wave 0 |
| RESP-05 | 4xx/5xx 不调用 `openai_to_anthropic()`（handler 层逻辑，不在 translate 层测试） | manual | n/a — Phase 15 handler 集成测试覆盖 | ❌ Out of scope for Phase 14 |
| STRM-01 | 文本 delta → 完整 Anthropic SSE 事件序列 | unit（async） | `cargo test --package cli-manager proxy::translate::stream::tests` | ❌ Wave 0 |
| STRM-02 | Deferred Start：id/name 未就绪时缓冲 args | unit（async） | `cargo test --package cli-manager proxy::translate::stream::tests::test_streaming_delays_tool_start` | ❌ Wave 0 |
| STRM-03 | 多工具 index 独立路由，互不干扰 | unit（async） | `cargo test --package cli-manager proxy::translate::stream::tests::test_streaming_tool_calls_routed_by_index` | ❌ Wave 0 |
| STRM-04 | finish_reason → message_delta + message_stop | unit（async） | `cargo test --package cli-manager proxy::translate::stream::tests` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test --package cli-manager translate`（仅跑 translate 模块的 unit tests）
- **Per wave merge:** `cargo test --package cli-manager`（全套 221+ 个测试全绿）
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src-tauri/src/proxy/translate/mod.rs` — 模块声明文件
- [ ] `src-tauri/src/proxy/translate/request.rs` — `anthropic_to_openai()` + 测试
- [ ] `src-tauri/src/proxy/translate/response.rs` — `openai_to_anthropic()` + 测试
- [ ] `src-tauri/src/proxy/translate/stream.rs` — `create_anthropic_sse_stream()` + 测试
- [ ] `src-tauri/Cargo.toml` — 新增 `bytes = "1"` + `futures = "0.3"` + `async-stream = "0.3"` 依赖
- [ ] `src-tauri/src/proxy/mod.rs` — 新增 `pub mod translate;`

---

## Sources

### Primary (HIGH confidence)

- cc-switch codebase `src-tauri/src/proxy/providers/transform.rs` — anthropic_to_openai() 和 openai_to_anthropic() 完整实现（775 行，含测试）
- cc-switch codebase `src-tauri/src/proxy/providers/streaming.rs` — create_anthropic_sse_stream() 完整实现（744 行，含测试）
- CLIManager `src-tauri/src/provider.rs` — 现有 Provider struct 和 ProtocolType enum 实现
- CLIManager `src-tauri/src/proxy/state.rs` — UpstreamTarget struct 现有定义
- CLIManager `src-tauri/Cargo.lock` — bytes 1.11.1 / futures 0.3.32 已锁定版本确认
- serde 文档（`#[serde(alias)]` 功能）— alias 与 rename_all 组合行为已知

### Secondary (MEDIUM confidence)

- CLIManager `src-tauri/Cargo.toml` — `[dev-dependencies]` 已有 futures 0.3，确认需提升为 dependencies
- cc-switch `src-tauri/Cargo.toml` — async-stream = "0.3" 直接依赖确认（cc-switch 已验证可用）

### Tertiary (LOW confidence)

- 无

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — bytes/futures 版本已在 Cargo.lock 确认，async-stream 已在 cc-switch 验证
- Architecture: HIGH — provider.rs 现有模式直接可用，translate/ 目录结构符合项目惯例
- Pitfalls: HIGH — Deferred Start 和 serde alias 问题均来自对代码库的直接分析，非推测

**Research date:** 2026-03-14
**Valid until:** 2026-04-14（serde/async-stream 版本稳定，30 天有效）
