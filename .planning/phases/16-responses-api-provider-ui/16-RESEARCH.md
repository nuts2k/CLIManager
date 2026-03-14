# Phase 16: Responses API + Provider UI - Research

**Researched:** 2026-03-14
**Domain:** OpenAI Responses API 格式转换 + Tauri 前端 Provider UI 扩展
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**协议类型 UI 呈现**
- 三个平级选项：Anthropic / OpenAI Chat Completions / OpenAI Responses
- 直接使用技术名称，中英文相同（无需翻译差异）
- 旧 `open_ai_compatible` 值前端加载时自动映射为 `open_ai_chat_completions`（Rust serde alias 已处理反序列化，前端 TypeScript 类型同步更新）
- 仅选择 OpenAI 类型（Chat Completions 或 Responses）时显示模型映射相关字段，Anthropic 时隐藏

**模型映射 UI 交互**
- 放在现有 Collapsible 高级设置区域内，protocol_type 下方
- 默认目标模型：单独一个输入框，placeholder 示例如 "gpt-4o"
- 模型映射对：动态行列表，每行两个输入框（源模型名 → 目标模型名）+ 删除按钮，底部"+ 添加映射"按钮
- 源模型名输入框 placeholder 示例如 "claude-sonnet-4-20250514"，目标模型名输入框 placeholder 示例如 "gpt-4o"
- 保存反馈复用现有 Provider 保存 toast，无需额外提示
- 映射数据随 Provider 一起保存，通过现有 update_provider 命令传递到 Rust 后端

**Responses API 转换层**
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

### Deferred Ideas (OUT OF SCOPE)
- 通用设置中管理预设映射模板 + 一键应用到 Provider — 未来功能，可作为独立 phase
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| RAPI-01 | Provider 可配置目标 API 格式（Chat Completions 或 Responses） | 已锁定 UI 决策：三选项 Select + handler.rs 路由分支拆分 |
| RAPI-02 | 选择 Responses 格式时，请求自动转换为 Responses API 格式 | OpenAI Responses API 请求格式已研究，字段映射表在下方 |
| RAPI-03 | Responses API 非流式响应正确转换回 Anthropic 格式 | output 数组结构已研究，转换路径清晰 |
| RAPI-04 | Responses API 流式事件正确转换为 Anthropic SSE 格式 | SSE 事件类型序列已研究，状态机设计在下方 |
| MODL-04 | Provider 编辑 UI 支持配置默认模型和映射对 | 现有 ProviderDialog + ProviderFormData 扩展路径清晰 |
</phase_requirements>

---

## Summary

Phase 16 由两条并行路线组成：(A) 在 Rust 层新建 Responses API 转换模块，将 Anthropic Messages API 请求/响应格式转换为 OpenAI Responses API 格式；(B) 在 TypeScript 前端扩展 ProviderDialog，新增协议选项和模型映射 UI。

**关键发现**：OpenAI Responses API 与 Chat Completions API 在请求格式上有重要差异——使用 `input` 替代 `messages`，使用 `instructions` 替代系统 role 消息；响应使用 `output` 数组替代 `choices`，输出项 type 为 `output_text` 和 `function_call`。流式 SSE 事件名称完全不同（`response.output_text.delta` 等，而非 `data:` + OpenAI Chat 格式）。转换层需从 Anthropic 消息格式映射到这套新格式，再将响应映射回 Anthropic 格式。

**路线 B 前端路径极其清晰**：`provider.ts` 中 `ProtocolType` 需新增两个变体，`Provider` 接口需加 `upstream_model` 和 `upstream_model_map` 字段，`ProviderFormData` 需加对应字段，`ProviderDialog.tsx` 在 Collapsible 中扩展 UI，`update_provider` 命令通过 `Provider` 对象整体传递新字段（后端已支持）。

**Primary recommendation:** 路线 A 按 responses_request.rs / responses_response.rs / responses_stream.rs 三文件独立实现，复用 `build_proxy_endpoint_url()` 和 `clean_schema()`，然后在 handler.rs 拆开 `ProtocolType::OpenAiResponses` 为独立分支；路线 B 在 ProviderDialog 中直接扩展现有表单，后端已就绪。

---

## Standard Stack

### Core（Rust）
| 库 | 版本 | 用途 | 理由 |
|---|---|---|---|
| serde_json::Value | 1.x（已有） | 动态 JSON 字段映射 | 与现有 Chat Completions 转换层一致 |
| bytes | 1.x（已有） | SSE 字节流 | 已用于 stream.rs |
| futures / async-stream | 0.3.x（已有） | 流式 SSE 生成器 | 与现有 create_anthropic_sse_stream 一致 |
| reqwest | 0.12.x（已有） | 上游请求 / bytes_stream | 已用于 handler.rs |

### Core（TypeScript 前端）
| 库 | 版本 | 用途 | 理由 |
|---|---|---|---|
| shadcn/ui（Input, Button, Label） | 已有 | 模型映射行 UI | 已在 ProviderDialog 中使用 |
| react useState | 已有 | 动态映射对列表状态 | 与现有 form 状态模式一致 |
| i18next useTranslation | 已有 | 新增协议名和映射相关文案 key | 全项目 i18n 规范 |

### 无需新增依赖

所有实现均可在已有 Cargo.toml 依赖范围内完成，无需新增任何 crate。

---

## Architecture Patterns

### Responses API 转换模块结构

```
src-tauri/src/proxy/translate/
├── mod.rs                     # 新增 responses_request / responses_response / responses_stream 导出
├── request.rs                 # Chat Completions 请求转换（已有，不动）
├── response.rs                # Chat Completions 响应转换（已有，不动）
├── stream.rs                  # Chat Completions 流式转换（已有，不动）
├── responses_request.rs       # 新建：Anthropic → Responses API 请求转换
├── responses_response.rs      # 新建：Responses API 非流式响应 → Anthropic 转换
└── responses_stream.rs        # 新建：Responses API SSE 流 → Anthropic SSE 流
```

### handler.rs 路由分支结构（修改后）

```rust
match upstream.protocol_type {
    ProtocolType::OpenAiChatCompletions => { /* 现有逻辑不变 */ }
    ProtocolType::OpenAiResponses => {
        // 新建独立分支：与 Chat Completions 并列
        // 步骤 C 之后：调用 responses_request::anthropic_to_responses()
        // 端点：build_proxy_endpoint_url(&base_url, "/responses")
        // 步骤 J：调用 responses_response 或 responses_stream
    }
    ProtocolType::Anthropic => {
        // 保持透传
    }
}
```

### 前端 Provider 类型结构（修改后）

```typescript
// provider.ts
export type ProtocolType =
  | "anthropic"
  | "open_ai_chat_completions"   // 新增：替代旧 open_ai_compatible
  | "open_ai_responses";          // 新增

export interface Provider {
  // ... 现有字段不变 ...
  upstream_model?: string | null;         // 新增（Rust 端已有）
  upstream_model_map?: Record<string, string> | null;  // 新增（Rust 端已有）
}

export interface ProviderFormData {
  // ... 现有字段不变 ...
  upstreamModel: string;
  upstreamModelMap: Array<{ source: string; target: string }>;
}
```

### Pattern 1: Responses API 请求转换（RAPI-02）

**关键字段映射（Anthropic → Responses API）：**

| Anthropic 字段 | Responses API 字段 | 说明 |
|---|---|---|
| `system`（字符串或数组） | `instructions`（字符串） | 数组取第一个 text 块拼接 |
| `messages`（数组） | `input`（数组） | role 对应关系见下 |
| `messages[].role = "user"` | `input[].role = "user"` | content 格式转换 |
| `messages[].role = "assistant"` | `input[].role = "assistant"` | 文本：type="output_text"；工具：type="function_call" |
| `messages[].role = "user"` + tool_result 块 | `input[]` type="function_call_output" | call_id 映射 |
| `tools[].input_schema` | `tools[].parameters` | 同 Chat Completions，加 clean_schema |
| `tools[].type = "BatchTool"` | 过滤掉 | 沿用 Phase 14 降级策略 |
| `max_tokens` | `max_output_tokens` | **字段名不同** |
| `stop_sequences` | `stop` | 同 Chat Completions |
| `temperature`, `top_p` | 同名透传 | 无需转换 |
| `stream` | `stream` | 同名透传 |
| `model` | `model` | 由 handler 层模型映射后透传 |

**端点：** `build_proxy_endpoint_url(&base_url, "/responses")`（可复用现有函数）

**关键差异：** Responses API 工具定义中 `type: "function"` 包装层**不需要**，直接是 `{ type: "function", name, description, parameters, strict: false }`（但不加外层 `function: {}` 嵌套层）。

**示例代码（responses_request.rs 框架）：**

```rust
// Source: OpenAI Responses API 官方文档 + 现有 request.rs 参考模式
pub fn anthropic_to_responses(body: Value) -> Result<Value, ProxyError> {
    let mut result = json!({});

    // model 原样透传（handler 层已完成模型映射）
    if let Some(model) = body.get("model").and_then(|m| m.as_str()) {
        result["model"] = json!(model);
    }

    // max_tokens -> max_output_tokens（字段名不同！）
    if let Some(v) = body.get("max_tokens") {
        result["max_output_tokens"] = v.clone();
    }

    // system -> instructions（取第一个 text 块的文本）
    if let Some(system) = body.get("system") {
        let instructions_text = extract_system_text(system);
        if !instructions_text.is_empty() {
            result["instructions"] = json!(instructions_text);
        }
    }

    // messages -> input（角色和内容格式转换）
    // ... 见 Pattern 1b

    // stream, temperature, top_p, stop_sequences->stop 同名或简单映射
    // ...

    // tools 转换（参数结构与 Chat Completions 类似，去掉 function: {} 包装层）
    // ...

    Ok(result)
}
```

### Pattern 1b: messages 转换为 input

Responses API `input` 数组中，内容格式与 Chat Completions 存在差异：

| 场景 | Chat Completions 格式 | Responses API 格式 |
|---|---|---|
| user 文本 | `{role:"user", content:"text"}` | `{role:"user", content:[{type:"input_text",text:"..."}]}` 或 `{role:"user",content:"text"}` |
| assistant 文本 | `{role:"assistant", content:"text"}` | `{role:"assistant", content:[{type:"output_text",text:"..."}]}` |
| assistant 工具调用 | `{role:"assistant", tool_calls:[...]}` | `{type:"function_call", call_id:"...", name:"...", arguments:"..."}` 作为独立 input 项 |
| tool_result | `{role:"tool", tool_call_id:"...", content:"..."}` | `{type:"function_call_output", call_id:"...", output:"..."}` |

**注意**：Responses API 中 `function_call` 和 `function_call_output` 是独立的 input 项（非 messages 角色消息），这是关键差异。

### Pattern 2: 非流式响应转换（RAPI-03）

**Responses API 响应结构 → Anthropic 格式：**

```json
// Responses API 响应（output 数组）
{
  "id": "resp_abc123",
  "object": "response",
  "output": [
    {
      "id": "msg_xyz",
      "type": "message",
      "role": "assistant",
      "content": [
        { "type": "output_text", "text": "Hello world", "annotations": [] }
      ],
      "status": "completed"
    }
  ],
  "usage": {
    "input_tokens": 10,
    "output_tokens": 5,
    "total_tokens": 15
  }
}
```

**转换映射（responses_response.rs）：**

| Responses API 字段 | Anthropic 字段 | 说明 |
|---|---|---|
| `output[].type == "message"` | 提取其 content | 主响应消息 |
| `content[].type == "output_text"` | `{type:"text", text:...}` | 文本块 |
| `output[].type == "function_call"` | `{type:"tool_use", id:call_id, name:..., input:{...}}` | 工具调用，arguments 需反序列化 |
| `usage.input_tokens` | `usage.input_tokens` | 同名 |
| `usage.output_tokens` | `usage.output_tokens` | 同名 |
| `id`（"resp_" 前缀） | 替换为 "msg_" 前缀 | 保持 Anthropic id 格式 |
| `output[-1].status == "completed"` | `stop_reason: "end_turn"` | 终止原因推断 |
| function_call 存在 | `stop_reason: "tool_use"` | 工具调用终止 |

**finish_reason 推断逻辑（无 finish_reason 字段，需从 output 推断）：**

```rust
// 检查最后一个 output 项的类型来推断 stop_reason
let stop_reason = if output_items_have_function_call {
    "tool_use"
} else {
    "end_turn"  // status == "completed" 视为 end_turn
};
```

### Pattern 3: 流式 SSE 转换状态机（RAPI-04）

**Responses API SSE 事件流 → Anthropic SSE 事件序列：**

```
Responses API 事件序列：
  response.created
  response.output_item.added       (type: "message")
  response.content_part.added      (type: "output_text")
  response.output_text.delta       (delta: "Hello...")
  response.output_text.delta       ...
  response.output_text.done
  response.output_item.done
  response.completed               (包含 usage)

转换为 Anthropic SSE 序列：
  message_start
  content_block_start (text)
  content_block_delta (text_delta)...
  content_block_stop
  message_delta (stop_reason + usage)
  message_stop
```

**函数调用流：**

```
Responses API 事件序列：
  response.output_item.added       (type: "function_call", call_id, name)
  response.function_call_arguments.delta
  response.function_call_arguments.done
  response.output_item.done
  response.completed

转换为 Anthropic SSE 序列：
  content_block_start (tool_use, id=call_id, name)
  content_block_delta (input_json_delta)...
  content_block_stop
  message_delta (stop_reason: "tool_use")
  message_stop
```

**关键状态机设计：**

```rust
// responses_stream.rs 状态机
struct ResponsesStreamState {
    message_started: bool,
    response_id: String,
    model: String,
    // output_item 追踪：output_index → block 状态
    output_items: HashMap<u32, OutputItemState>,
    next_anthropic_index: u32,
    // 已打开的 block 集合（用于流结束时统一关闭）
    open_block_indices: HashSet<u32>,
}

enum OutputItemState {
    TextBlock { anthropic_index: u32, started: bool },
    FunctionCall { anthropic_index: u32, call_id: String, name: String, started: bool },
}
```

**与 Chat Completions stream.rs 的关键区别：**
- Responses API 中 `response.output_item.added` 同时携带 `call_id` 和 `name`，**无需 Deferred Start 机制**
- `response.completed` 携带 `usage`（`input_tokens` / `output_tokens`，命名与 Anthropic 相同）
- `message_start` 从 `response.created` 触发（而非首个 delta chunk）

### Anti-Patterns to Avoid

- **不要复用 stream.rs 的 Deferred Start**：Responses API 函数调用 item 在 `response.output_item.added` 时 id/name 已完整，不需要延迟启动逻辑
- **不要照搬 Chat Completions 的工具定义包装层**：Responses API 的 tools 定义没有 `function: {}` 外包装，直接放 `name`, `description`, `parameters`
- **不要混用模块**：responses_request.rs / responses_response.rs / responses_stream.rs 作为独立文件，不要在 Chat Completions 模块内添加分支
- **不要忘记 max_tokens → max_output_tokens 的字段名变化**

---

## Don't Hand-Roll

| 问题 | 不要手写 | 使用现有实现 | 理由 |
|---|---|---|---|
| URL 端点重写 | 自定义 URL 拼接 | `build_proxy_endpoint_url(&base_url, "/responses")` | 已覆盖 /v1 路径处理 |
| JSON Schema 清理 | 手写递归清理 | `clean_schema(schema)` | 已测试，移除 format/default |
| 模型名映射 | handler 内手写 | `apply_upstream_model_mapping()` | 已在 handler.rs:33 实现 |
| 凭据注入 | 复写认证逻辑 | handler.rs 步骤 G（bearer token 已支持 OpenAiResponses） | 凭据注入逻辑第 172 行已合并处理 |
| SSE 字节格式化 | 手写 SSE 格式 | `format_sse_event(event_type, data)` | 从 stream.rs 提取到共享位置或独立复制 |
| 动态行 UI | 手写 DOM 操作 | React useState + map 渲染 | 项目内已有同等模式 |

---

## Common Pitfalls

### Pitfall 1: max_tokens 字段名遗漏

**什么出错：** 发送到 Responses API 时沿用 `max_tokens`，导致上游忽略该参数，响应无限制输出
**根因：** Responses API 使用 `max_output_tokens`，Chat Completions 使用 `max_tokens`
**如何避免：** responses_request.rs 中明确将 `body.get("max_tokens")` 映射到 `result["max_output_tokens"]`
**警告信号：** 上游响应异常长或上游返回 4xx 参数错误

### Pitfall 2: assistant 消息的 content 格式错误

**什么出错：** 将 Anthropic 转换来的 assistant 消息的文本内容放为字符串或 Chat Completions 格式，Responses API 返回 400
**根因：** Responses API 的 assistant role 消息中文本 type 应为 `output_text`，而 user 消息文本 type 应为 `input_text`
**如何避免：** convert_message_to_responses() 中按 role 分路，assistant 用 `output_text`，user 用 `input_text`
**警告信号：** 上游返回 "Invalid content type for role" 类错误

### Pitfall 3: 工具调用 call_id vs id 混用

**什么出错：** 将 Responses API 的 `call_id` 作为 Anthropic `tool_use` 的 `id`，下一轮对话时工具结果匹配失败
**根因：** Responses API function_call 有两个字段：`id`（item 自身 id）和 `call_id`（实际要用的 id）；转换应使用 `call_id`
**如何避免：** responses_response.rs 和 responses_stream.rs 中都使用 `call_id` 字段映射到 Anthropic `tool_use.id`
**警告信号：** tool_use id 格式异常，Claude Code 工具调用后不响应

### Pitfall 4: 前端 ProtocolType 类型遗漏旧值兼容

**什么出错：** 存储中 `open_ai_compatible` 的 Provider 加载到前端时类型不匹配（TypeScript 类型不包含旧值），UI 展示空白
**根因：** Rust 端已通过 serde alias 处理反序列化，但前端 TS 类型没有覆盖旧值
**如何避免：** provider.ts 将 `ProtocolType` 更新为三值枚举（`anthropic` | `open_ai_chat_completions` | `open_ai_responses`），前端加载时做 `open_ai_compatible` → `open_ai_chat_completions` 映射（在列表加载或表单初始化时）
**警告信号：** 编辑旧 Provider 时 protocol_type 字段显示不正常

### Pitfall 5: 模型映射 UI 数据与后端格式不匹配

**什么出错：** 前端将 `upstream_model_map` 存为数组（映射对列表），而后端 Provider struct 期望 `HashMap<String, String>`
**根因：** UI 中映射对用数组形式展示更自然，但 Rust / JSON 序列化期望 key-value 对象
**如何避免：** `ProviderFormData` 内部用 `Array<{source: string; target: string}>`，保存时转换为 `Record<string, string>` 传给 `updateProvider`
**警告信号：** 保存后模型映射不生效，或控制台 serde 错误

### Pitfall 6: stream.rs 内部 format_sse_event 不可跨模块直接引用

**什么出错：** responses_stream.rs 尝试调用 stream.rs 的私有 `format_sse_event` 导致编译错误
**根因：** 该函数在 stream.rs 中是 `fn`（私有）
**如何避免：** 选择之一：(a) 在 responses_stream.rs 中直接内联相同的辅助函数（简单，不强耦合）；(b) 将其提升到 mod.rs 中作为 `pub(super) fn`
**警告信号：** 编译报 "function not found" 或 "private function"

---

## Code Examples

### Responses API 请求体示例（文本对话）

```json
// Source: OpenAI Responses API 官方文档 2025
{
  "model": "gpt-4o",
  "instructions": "You are a helpful assistant.",
  "input": [
    { "role": "user", "content": "Hello" }
  ],
  "max_output_tokens": 1024,
  "temperature": 0.7,
  "stream": false
}
```

### Responses API 请求体示例（含工具）

```json
// Source: OpenAI 官方文档 function calling
{
  "model": "gpt-4o",
  "input": [
    { "role": "user", "content": "What's the weather?" }
  ],
  "tools": [
    {
      "type": "function",
      "name": "get_weather",
      "description": "Get current weather",
      "parameters": {
        "type": "object",
        "properties": { "location": { "type": "string" } },
        "required": ["location"]
      }
    }
  ],
  "max_output_tokens": 1024
}
```

注意：与 Chat Completions 不同，**无 `function: {}` 包装层**，`name`/`description`/`parameters` 直接在 tool 对象上。

### 非流式 Responses API 响应

```json
// Source: OpenAI Responses API 文档 output 结构
{
  "id": "resp_68af403059...",
  "object": "response",
  "model": "gpt-4o",
  "output": [
    {
      "id": "msg_68af4033...",
      "type": "message",
      "role": "assistant",
      "status": "completed",
      "content": [
        { "type": "output_text", "text": "Hello!", "annotations": [] }
      ]
    }
  ],
  "usage": {
    "input_tokens": 10,
    "output_tokens": 3,
    "total_tokens": 13
  }
}
```

### 流式 SSE 事件示例（文本）

```
// Source: OpenAI Responses API Streaming Reference
event: response.created
data: {"type":"response.created","response":{"id":"resp_abc","object":"response","status":"in_progress","model":"gpt-4o",...},"sequence_number":0}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":0,"item":{"id":"msg_xyz","type":"message","role":"assistant","content":[],"status":"in_progress"},"sequence_number":1}

event: response.content_part.added
data: {"type":"response.content_part.added","item_id":"msg_xyz","output_index":0,"content_index":0,"part":{"type":"output_text","text":"","annotations":[]},"sequence_number":2}

event: response.output_text.delta
data: {"type":"response.output_text.delta","item_id":"msg_xyz","output_index":0,"content_index":0,"delta":"Hello","sequence_number":3}

event: response.output_text.done
data: {"type":"response.output_text.done","item_id":"msg_xyz","output_index":0,"content_index":0,"text":"Hello"}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":0,"item":{...}}

event: response.completed
data: {"type":"response.completed","response":{"id":"resp_abc","output":[...],"usage":{"input_tokens":10,"output_tokens":1,"total_tokens":11}}}
```

### 流式 SSE 事件示例（函数调用）

```
event: response.output_item.added
data: {"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_123","call_id":"call_abc","name":"get_weather","arguments":""}}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","item_id":"fc_123","output_index":0,"delta":"{\"location\":","sequence_number":5}

event: response.function_call_arguments.done
data: {"type":"response.function_call_arguments.done","item_id":"fc_123","output_index":0,"arguments":"{\"location\":\"Tokyo\"}","name":"get_weather"}

event: response.output_item.done
data: {...}

event: response.completed
data: {...,"usage":{"input_tokens":20,"output_tokens":10}}
```

### ProviderDialog 模型映射 UI 片段（TypeScript）

```tsx
// ProviderDialog.tsx - 高级设置区域内，protocol_type 下方
// 仅在 OpenAI 类型时显示
{(form.protocolType === "open_ai_chat_completions" ||
  form.protocolType === "open_ai_responses") && (
  <>
    {/* 默认目标模型 */}
    <div className="flex flex-col gap-1.5">
      <Label>{t("provider.upstreamModel")}</Label>
      <Input
        value={form.upstreamModel}
        onChange={(e) => updateField("upstreamModel", e.target.value)}
        placeholder="gpt-4o"
      />
    </div>

    {/* 模型映射对列表 */}
    <div className="flex flex-col gap-2">
      <Label className="text-muted-foreground text-xs">
        {t("provider.modelMapping")}
      </Label>
      {form.upstreamModelMap.map((pair, idx) => (
        <div key={idx} className="flex gap-2">
          <Input
            value={pair.source}
            onChange={(e) => updateModelMapEntry(idx, "source", e.target.value)}
            placeholder="claude-sonnet-4-20250514"
            className="flex-1"
          />
          <Input
            value={pair.target}
            onChange={(e) => updateModelMapEntry(idx, "target", e.target.value)}
            placeholder="gpt-4o"
            className="flex-1"
          />
          <Button variant="ghost" size="icon-xs" onClick={() => removeModelMapEntry(idx)}>
            <X className="size-3.5" />
          </Button>
        </div>
      ))}
      <Button variant="outline" size="sm" onClick={addModelMapEntry}>
        + {t("provider.addMapping")}
      </Button>
    </div>
  </>
)}
```

---

## State of the Art

| 旧方式 | 当前方式 | 说明 |
|---|---|---|
| `open_ai_compatible` 单一协议选项 | 三选项：Anthropic / OpenAI Chat Completions / OpenAI Responses | Phase 16 新增 |
| OpenAiResponses 走透传路径 | OpenAiResponses 走独立转换路径 | Phase 16 拆分 handler.rs |
| 前端无 upstream_model/map UI | ProviderDialog 高级设置中可视化编辑 | Phase 16 MODL-04 |
| max_tokens → 直接透传 | Responses API 使用 max_output_tokens | 需在 responses_request.rs 中处理 |

**已废弃/需注意：**
- `open_ai_compatible`：后端已通过 serde alias 保持兼容，前端保存时应输出 `open_ai_chat_completions`

---

## Open Questions

1. **工具调用历史对话（multi-turn 时的 tool_result 转换格式）**
   - 已知：Responses API 工具结果用 `{type:"function_call_output", call_id:"...", output:"..."}` 作为独立 input 项
   - 已知：Anthropic 用 `{role:"user", content:[{type:"tool_result", tool_use_id:"..."}]}`
   - 不确定：Claude Code 实际发送的多轮对话中 tool_result 格式的变体（数组 content 或字符串 content）
   - 建议：responses_request.rs 中，处理 tool_result 时与 Chat Completions 的 request.rs 行为一致（字符串或数组均序列化为字符串）

2. **响应 usage 字段名称一致性**
   - Responses API 的 `usage` 字段已使用 `input_tokens` / `output_tokens`（与 Anthropic 命名相同），无需重命名
   - 但 Chat Completions 版本需要 `prompt_tokens` → `input_tokens` 的映射
   - 建议：responses_response.rs 中，直接将 `usage.input_tokens` / `usage.output_tokens` 透传到 Anthropic 响应，无需重命名

3. **Responses API stop_reason 推断**
   - Responses API 无 `finish_reason` 字段，需从 `output` 中推断
   - 建议：若最后一个 output 项为 `function_call`，`stop_reason = "tool_use"`；否则 `"end_turn"`（status=completed）；若 status=incomplete，则 `"max_tokens"`

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `#[test]` / `#[tokio::test]` + cargo test |
| Config file | `src-tauri/Cargo.toml`（已配置） |
| Quick run command | `cd src-tauri && cargo test -p cli-manager-lib -- translate::responses 2>&1` |
| Full suite command | `cd src-tauri && cargo test -p cli-manager-lib 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RAPI-02 | anthropic_to_responses() 请求字段映射 | unit | `cargo test translate::responses_request -- --nocapture` | ❌ Wave 0 |
| RAPI-02 | max_tokens → max_output_tokens 转换 | unit | `cargo test responses_request::test_max_tokens_mapping` | ❌ Wave 0 |
| RAPI-02 | instructions 字段从 system 提取 | unit | `cargo test responses_request::test_system_to_instructions` | ❌ Wave 0 |
| RAPI-02 | tool 定义无 function 包装层 | unit | `cargo test responses_request::test_tools_format` | ❌ Wave 0 |
| RAPI-03 | output 数组文本响应转换 | unit | `cargo test responses_response::test_text_response` | ❌ Wave 0 |
| RAPI-03 | output function_call 转换为 tool_use | unit | `cargo test responses_response::test_function_call_response` | ❌ Wave 0 |
| RAPI-03 | usage 字段透传 | unit | `cargo test responses_response::test_usage_passthrough` | ❌ Wave 0 |
| RAPI-04 | 文本流式事件序列 | unit (async) | `cargo test responses_stream::test_text_stream` | ❌ Wave 0 |
| RAPI-04 | 函数调用流式事件序列 | unit (async) | `cargo test responses_stream::test_function_call_stream` | ❌ Wave 0 |
| RAPI-01 | handler.rs 路由分支覆盖 | unit | `cargo test handler::tests` | ✅（现有测试扩展） |
| MODL-04 | 前端 ProviderDialog UI 集成 | manual | 手动 UI 验证 | N/A |

### Sampling Rate

- **Per task commit:** `cd src-tauri && cargo test -p cli-manager-lib -- translate::responses 2>&1 | tail -5`
- **Per wave merge:** `cd src-tauri && cargo test -p cli-manager-lib 2>&1`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src-tauri/src/proxy/translate/responses_request.rs` — 覆盖 RAPI-02（纯函数单元测试）
- [ ] `src-tauri/src/proxy/translate/responses_response.rs` — 覆盖 RAPI-03（纯函数单元测试）
- [ ] `src-tauri/src/proxy/translate/responses_stream.rs` — 覆盖 RAPI-04（tokio::test 异步测试）

---

## Sources

### Primary (HIGH confidence)
- OpenAI Responses API 官方文档（streaming events reference）— [platform.openai.com/docs/api-reference/responses-streaming](https://platform.openai.com/docs/api-reference/responses-streaming) — SSE 事件类型和字段
- 现有代码库：`src-tauri/src/proxy/translate/request.rs`、`response.rs`、`stream.rs`、`handler.rs` — 现有模式和可复用函数
- 现有代码库：`src/components/provider/ProviderDialog.tsx`、`src/types/provider.ts` — 前端扩展基础

### Secondary (MEDIUM confidence)
- OpenAI community 详细事件指南 — [community.openai.com/t/responses-api-streaming-the-simple-guide-to-events/1363122](https://community.openai.com/t/responses-api-streaming-the-simple-guide-to-events/1363122) — 流式事件完整列表（社区文档，经多处交叉验证）
- OpenAI function calling 文档（WebSearch 抓取摘要）— tools 格式差异（无 function 包装层）
- OpenAI input message roles 文档（WebSearch 摘要）— role 映射和 content type 规范

### Tertiary (LOW confidence)
- WebSearch 摘要中的 Responses API 请求体格式描述 — 字段名和结构细节（已从多个来源交叉验证，提升为 MEDIUM）

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 全部在现有依赖范围内，无新 crate
- Responses API 请求格式: MEDIUM-HIGH — 关键字段（input, instructions, max_output_tokens, tools 无包装层）已从多个来源交叉验证
- Responses API 响应格式: MEDIUM-HIGH — output 数组、output_text、function_call、call_id 字段已验证
- Responses API 流式 SSE 事件: HIGH — 事件名称序列（response.output_text.delta, response.function_call_arguments.delta, response.completed）已从官方文档和社区文档交叉验证
- 前端架构: HIGH — 直接基于现有代码分析，无推测
- Pitfalls: HIGH — 基于 API 差异文档和现有代码模式

**Research date:** 2026-03-14
**Valid until:** 2026-04-13（30 天，OpenAI Responses API 规范相对稳定）
