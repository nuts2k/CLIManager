# Phase 15: Handler 集成与协议路由 - Research

**Researched:** 2026-03-14
**Domain:** Rust proxy handler 集成，协议路由，模型名映射，E2E SSE 链路验证
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions (from Phase 14)
- 映射优先级：精确匹配 → upstream_model → 保留原模型名
- 4xx/5xx 错误响应直接透传，不经转换处理（RESP-05）
- 转换失败返回 400 BAD_REQUEST（ProxyError::TranslateError）
- 端点重写使用 build_proxy_endpoint_url()（已实现）
- 所有转换函数是纯函数，handler 层负责调用和组装

### Claude's Discretion
- 协议路由分支在 handler.rs 中的插入位置和代码结构设计
- UpstreamTarget 是否扩展以携带 upstream_model/upstream_model_map，或在 handler 中另行获取映射数据
- 流式/非流式响应的检测方式（请求 body stream 字段 vs 响应 content-type）
- E2E 验证方式（mock server 集成测试 vs 手动真实 Provider 测试）
- handler.rs 内部函数拆分方式
- 错误处理细节（转换函数内部错误已由 TranslateError/400 处理，handler 层如何包装）

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| ROUT-01 | 代理模式下，Provider 协议类型为 OpenAiCompatible 时自动启用协议转换路径 | handler.rs 步骤 H 之前插入 `match upstream.protocol_type { OpenAiChatCompletions => 转换路径 }` |
| ROUT-02 | Provider 协议类型为 Anthropic 时请求直接透传，零回归 | handler.rs 现有 Anthropic 路径完全不修改，仅在 OpenAiChatCompletions 分支中介入 |
| MODL-03 | 代理转换时按映射表自动替换请求中的模型名（精确匹配优先，无匹配时用默认模型） | handler 层读取 `UpstreamTarget` 携带的 `upstream_model` + `upstream_model_map`，在调用 `anthropic_to_openai()` 之前执行映射 |
</phase_requirements>

---

## Summary

Phase 15 的核心工作是将 Phase 14 实现的三个纯函数转换模块（request.rs、response.rs、stream.rs）集成到 proxy_handler，并实现 MODL-03 的模型名映射逻辑。当前 handler.rs 是一个 133 行的线性透传 handler，步骤 A-J 依次处理请求。Phase 15 要在两个点位插入新逻辑：步骤 D（URL 拼接）改为 `build_proxy_endpoint_url()`，以及步骤 H 之前（转换请求体 + 模型映射），步骤 J 之后（转换响应体）。

`UpstreamTarget` 结构体当前只有三个字段（api_key、base_url、protocol_type）。Phase 15 需要扩展它以携带 `upstream_model` 和 `upstream_model_map`，并更新 `build_upstream_target_from_provider()` 以及 mod.rs 中的若干测试辅助函数，同时将 `base_url` 的处理从 `extract_origin_base_url`（strip path）改为直接保留完整 base_url（含路径，供 `build_proxy_endpoint_url` 使用）。

流式/非流式分支的检测应从**请求 body** 的 `"stream": true` 字段读取，而不依赖响应 content-type，因为 handler 已经读取了 body_bytes（步骤 C），这是最可靠的判断时机。

**Primary recommendation:** 一次性完成 UpstreamTarget 扩展 → handler 路由分支 → 集成测试三步，无内部并行依赖，按单一线性路径推进。

---

## Standard Stack

### Core（已有，无需新增）

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde_json` | 1.x | `Value` 动态操作，请求体解析 | 转换函数输入输出类型 |
| `axum::body` | 0.7.x | `Body::from_stream()`，SSE 响应流构建 | 现有透传路径已使用 |
| `reqwest` | 0.12.x | `bytes_stream()` 获取上游响应流 | 现有 http_client 使用 |
| `bytes::Bytes` | 1.11.1 | SSE 流块类型 | `create_anthropic_sse_stream` 的返回类型 |
| `futures::stream::Stream` | 0.3.32 | 流式适配器类型 | `create_anthropic_sse_stream` 的参数类型 |

### 新依赖（无）

Phase 15 不需要引入任何新 crate，所有依赖在 Phase 14 已完整添加到 Cargo.toml。

---

## Architecture Patterns

### 推荐改动范围

```
src-tauri/src/
├── proxy/
│   ├── state.rs          ← 扩展 UpstreamTarget（+2 字段）
│   ├── handler.rs        ← 核心改动：路由分支 + 模型映射 + 响应转换
│   └── mod.rs            ← 更新 make_upstream 辅助函数（测试用）
└── commands/
    └── proxy.rs          ← 更新 build_upstream_target_from_provider() + base_url 处理策略
```

### Pattern 1: UpstreamTarget 扩展

**What:** 为 UpstreamTarget 添加模型映射字段，使 handler 无需回溯查询 Provider 数据。
**When to use:** UpstreamTarget 是 handler 的唯一信息来源；映射数据应当和凭据一起传入。

```rust
// src-tauri/src/proxy/state.rs — 扩展后
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct UpstreamTarget {
    pub api_key: String,
    pub base_url: String,              // 保留完整路径（含 /v1 等）
    pub protocol_type: ProtocolType,
    pub upstream_model: Option<String>,
    pub upstream_model_map: Option<HashMap<String, String>>,
}
```

> **注意：** `base_url` 字段的语义改变：从"仅 origin"变为"完整 base_url（含路径前缀）"。`extract_origin_base_url` 已不适用于 OpenAiChatCompletions 场景，应在 `build_upstream_target_from_provider` 中直接保留原始 base_url（或至多清理 credentials/fragment，不 strip path）。Anthropic Provider 的 base_url 本来就不含路径，所以两者行为一致。

### Pattern 2: build_upstream_target_from_provider 更新

**What:** 从 Provider struct 读取 upstream_model 和 upstream_model_map，填充扩展后的 UpstreamTarget。

```rust
// src-tauri/src/commands/proxy.rs
fn build_upstream_target_from_provider(provider: &Provider) -> Result<UpstreamTarget, AppError> {
    // OpenAiChatCompletions 保留含路径的 base_url，Anthropic 行为不变（base_url 本就不含路径）
    // 统一使用 extract_origin_base_url 兼容旧格式，或直接保留 provider.base_url
    // 关键：不应 strip /v1 路径，build_proxy_endpoint_url 需要它
    Ok(UpstreamTarget {
        api_key: provider.api_key.clone(),
        base_url: provider.base_url.clone(),  // 直接保留，不 strip path
        protocol_type: provider.protocol_type.clone(),
        upstream_model: provider.upstream_model.clone(),
        upstream_model_map: provider.upstream_model_map.clone(),
    })
}
```

> **重要：** 当前 `build_upstream_target_from_provider` 调用 `extract_origin_base_url` 会 strip 所有路径（如 `https://openrouter.ai/api/v1` → `https://openrouter.ai`）。这对 Anthropic Provider 无影响（base_url 不含路径），但对 OpenAiChatCompletions Provider 会破坏 `build_proxy_endpoint_url` 的正确行为。Phase 15 应**直接保留 `provider.base_url`**（不经过任何 URL 操作），让 `build_proxy_endpoint_url` 在 handler 中处理所有路径逻辑。现有测试 `test_build_upstream_target_from_provider_strips_legacy_path` 需要更新以反映新行为。

### Pattern 3: handler.rs 协议路由分支（核心集成点）

**What:** 在 handler.rs 中增加两个关键插入点：请求转换（步骤 C 之后、步骤 H 之前）和响应转换（步骤 J 处）。

```rust
// handler.rs 整体结构（伪代码，展示关键修改点）

// 步骤 C：读取 body（已有）
let body_bytes = axum::body::to_bytes(req.into_body(), 200 * 1024 * 1024).await?;

// ===================== 新增：协议路由分支 =====================
let (upstream_url, translated_body_bytes, is_streaming) = match upstream.protocol_type {
    ProtocolType::OpenAiChatCompletions => {
        // 步骤 1：解析请求体
        let body_value: Value = serde_json::from_slice(&body_bytes)
            .map_err(|e| ProxyError::TranslateError(format!("无法解析请求体: {}", e)))?;

        // 步骤 2：模型名映射（MODL-03）
        let body_value = apply_upstream_model_mapping(body_value, &upstream);

        // 步骤 3：是否流式（从请求体读取）
        let is_streaming = body_value.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);

        // 步骤 4：请求转换 + 端点重写
        let openai_body = translate::request::anthropic_to_openai(body_value)?;
        let url = translate::request::build_proxy_endpoint_url(&upstream.base_url, "/chat/completions");
        let new_bytes = serde_json::to_vec(&openai_body)
            .map_err(|e| ProxyError::Internal(e.to_string()))?;

        (url, Bytes::from(new_bytes), is_streaming)
    }
    ProtocolType::Anthropic | ProtocolType::OpenAiResponses => {
        // 透传路径：URL 拼接与现有逻辑一致
        let url = format!("{}{}{}", upstream.base_url.trim_end_matches('/'), path, query);
        (url, body_bytes, false)  // Anthropic 不走转换，is_streaming 不影响后续
    }
};
// ===================== 新增结束 =====================

// 步骤 E-H：构建 reqwest 请求（使用 translated_body_bytes）
// ... 现有逻辑基本不变 ...
let upstream_resp = req_builder
    .body(translated_body_bytes.to_vec())
    .send()
    .await?;

// 步骤 I：构建响应（透传 status + headers）
// ... 现有逻辑不变 ...

// 步骤 J：响应体处理
// ===================== 新增：响应转换 =====================
let body = match upstream.protocol_type {
    ProtocolType::OpenAiChatCompletions => {
        if status.is_success() {
            if is_streaming {
                // 流式：直接 wrap 为 SSE 转换流
                let model = "unknown".to_string();  // 或从 body_value 提取
                Body::from_stream(
                    translate::stream::create_anthropic_sse_stream(upstream_resp.bytes_stream())
                )
            } else {
                // 非流式：读完整响应，转换后返回
                let resp_bytes = upstream_resp.bytes().await
                    .map_err(|e| ProxyError::Internal(e.to_string()))?;
                let resp_value: Value = serde_json::from_slice(&resp_bytes)
                    .map_err(|e| ProxyError::TranslateError(format!("响应解析失败: {}", e)))?;
                let anthropic_resp = translate::response::openai_to_anthropic(resp_value)?;
                let resp_bytes = serde_json::to_vec(&anthropic_resp)
                    .map_err(|e| ProxyError::Internal(e.to_string()))?;
                Body::from(resp_bytes)
            }
        } else {
            // 4xx/5xx 直接透传（RESP-05）
            Body::from_stream(upstream_resp.bytes_stream())
        }
    }
    _ => {
        // Anthropic/OpenAiResponses：透传（现有行为）
        Body::from_stream(upstream_resp.bytes_stream())
    }
};
// ===================== 新增结束 =====================
```

### Pattern 4: 模型名映射纯函数

**What:** 将 MODL-03 的映射逻辑提取为独立辅助函数，便于单元测试。
**Where:** 可放在 `handler.rs` 内部（私有），或放在新文件 `proxy/model_mapping.rs`（如代码量增长）。

```rust
/// 根据 UpstreamTarget 的映射配置替换请求体中的模型名
///
/// 优先级：upstream_model_map 精确匹配 > upstream_model（默认） > 保留原模型名
fn apply_upstream_model_mapping(mut body: Value, upstream: &UpstreamTarget) -> Value {
    let original_model = match body.get("model").and_then(|m| m.as_str()) {
        Some(m) => m.to_string(),
        None => return body,
    };

    // 1. 精确匹配 upstream_model_map
    if let Some(ref map) = upstream.upstream_model_map {
        if let Some(mapped) = map.get(&original_model) {
            body["model"] = serde_json::json!(mapped);
            return body;
        }
    }

    // 2. 无精确匹配 → 使用 upstream_model（默认模型）
    if let Some(ref default_model) = upstream.upstream_model {
        body["model"] = serde_json::json!(default_model);
        return body;
    }

    // 3. 无任何映射 → 保留原模型名
    body
}
```

### Pattern 5: 非流式响应的 Content-Length 头处理

**What:** 非流式响应经过 `openai_to_anthropic()` 转换后，响应体长度改变。上游返回的 `content-length` 头已失效，必须移除（防止客户端截断读取）。
**When to use:** OpenAiChatCompletions 非流式成功响应的响应头过滤时。

```rust
// 在步骤 I（构建响应头）中，非流式转换路径需要额外过滤 content-length
for (key, value) in resp_headers.iter() {
    let k = key.as_str().to_lowercase();
    if matches!(k.as_str(), "transfer-encoding" | "content-length" | "connection") {
        continue;  // content-length 在此处已被过滤——转换后字节数不同
    }
    builder = builder.header(key, value);
}
```

> **注意：** 现有的响应头过滤逻辑（handler.rs 步骤 I）已经过滤了 `content-length`，因此非流式转换场景**无需额外修改**。这是一个已处理好的陷阱。

### Anti-Patterns to Avoid

- **从响应 content-type 判断是否流式：** 此时 handler 已经在等待响应，无法再修改请求。应从**请求体**的 `"stream"` 字段判断。
- **在 Anthropic 透传路径中调用任何转换函数：** ROUT-02 要求零回归，Anthropic 路径完全不触碰转换模块。
- **UpstreamTarget 扩展后忘记更新测试辅助函数 `make_upstream`：** proxy/mod.rs 中的集成测试使用 `make_upstream()` 辅助函数构造 `UpstreamTarget`，增加字段后需更新（新字段填 `None`）。
- **`body_bytes` 既被读取又被重用于透传：** 步骤 C 读取 body_bytes 后，转换路径生成新的 `translated_body_bytes`；透传路径重用 `body_bytes`。两条路径共享一个 `Bytes` clone 即可，不需要重新读取。
- **流式路径下读取整个响应体：** `upstream_resp.bytes().await` 会等待整个响应完成，绝对不能在流式路径中使用。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 请求体 Anthropic→OpenAI 转换 | 手写字段复制 | `translate::request::anthropic_to_openai()` | Phase 14 已有完整实现，29 个测试覆盖所有边缘情况 |
| 端点 URL 重写 | 字符串拼接 | `translate::request::build_proxy_endpoint_url()` | 已处理 /v1 重复问题，4 个测试验证 |
| 非流式响应转换 | 手写 JSON 字段重命名 | `translate::response::openai_to_anthropic()` | Phase 14 已有完整实现，28 个测试 |
| 流式 SSE 转换 | 手写 SSE 事件生成 | `translate::stream::create_anthropic_sse_stream()` | Deferred Start 状态机极复杂，Phase 14 已验证 |
| 模型名映射查找 | 内联 if-else | `apply_upstream_model_mapping()` 纯函数 | 可独立单元测试，逻辑清晰 |

---

## Common Pitfalls

### Pitfall 1: base_url 含路径被 extract_origin_base_url strip 掉

**What goes wrong:** `build_upstream_target_from_provider` 当前调用 `extract_origin_base_url`，将 `https://openrouter.ai/api/v1` 变为 `https://openrouter.ai`，导致后续 `build_proxy_endpoint_url` 生成 `https://openrouter.ai/v1/chat/completions`（缺少 `/api`，404）。
**Why it happens:** `extract_origin_base_url` 专门 strip path，设计用于获取 origin，不适用于代理转发。
**How to avoid:** 更新 `build_upstream_target_from_provider` 直接使用 `provider.base_url`（不做 URL 操作）。同步更新 `test_build_upstream_target_from_provider_strips_legacy_path` 测试用例，将其改为验证新行为（保留路径）。
**Warning signs:** 集成测试中 openrouter.ai 类 Provider 请求返回 404。

### Pitfall 2: UpstreamTarget 有多处构造点

**What goes wrong:** `UpstreamTarget` 扩展后新增两个字段，但代码中有多处构造点（proxy/mod.rs 测试的 `make_upstream`、commands/proxy.rs 的 `build_upstream_target` 和 `build_upstream_target_from_provider`、以及 commands/proxy.rs 中若干测试的内联构造），遗漏任一处会导致编译错误。
**Why it happens:** Rust 结构体初始化必须填写所有字段（无 `Default` derive 时）。
**How to avoid:** 搜索所有 `UpstreamTarget {` 构造点，统一更新。建议为新字段添加 `..UpstreamTarget::default()` 或明确填写 `None`。或者为 `UpstreamTarget` 实现 `Default` + 建造者模式以减少搜索遗漏。
**Warning signs:** 编译错误 `missing field 'upstream_model' in initializer of 'UpstreamTarget'`。当前已知构造点（来自 grep）：
- `proxy/state.rs` 测试 `make_target()`
- `proxy/mod.rs` 测试 `make_upstream()`
- `commands/proxy.rs` 的 `build_upstream_target()`（基于字符串参数，新字段填 None）
- `commands/proxy.rs` 的 `build_upstream_target_from_provider()`（新字段读 Provider）
- `commands/proxy.rs` 若干测试内联构造（约 5 处，需统一填 None）

### Pitfall 3: 流式路径 Content-Type 头丢失

**What goes wrong:** 流式 SSE 响应经转换后，`content-type: text/event-stream` 头如果在 Anthropic 侧不存在，Claude Code 无法识别为 SSE 流（可能整体等待响应完成后才处理）。
**Why it happens:** OpenAI 上游会设置正确的 content-type，但如果 content-type 头被过滤，下游就收不到流式标记。
**How to avoid:** 确认 `content-type: text/event-stream` 在步骤 I 的头部过滤中不被误删（当前过滤只针对 `transfer-encoding`/`content-length`/`connection`，`content-type` 已经会被透传）。这个陷阱在当前实现下**已被自动处理**，但应在集成测试中显式验证。

### Pitfall 4: 非流式转换后构建响应缺少 Content-Type

**What goes wrong:** 非流式路径读取 body → 转换 → 重新序列化为 JSON，但响应头中的 `content-type: application/json` 来自上游。若上游返回的 content-type 与 Anthropic 格式不同（如 `charset=utf-8` 差异），Claude Code 可能拒绝解析。
**Why it happens:** 透传上游 content-type，但重新序列化后编码格式一致（serde_json 始终输出 UTF-8）。
**How to avoid:** 对非流式转换路径，可以明确将 content-type 覆盖为 `application/json`，而不透传上游的值。这确保 Claude Code 始终收到标准格式。

### Pitfall 5: `is_streaming` 变量在透传路径下影响响应体处理

**What goes wrong:** `is_streaming` 变量在透传路径（Anthropic）下未被设置（或设为 false），但响应体处理分支（步骤 J）依赖它。若结构不清晰，可能导致 Anthropic 流式响应被错误当成非流式处理。
**Why it happens:** 逻辑结构不清晰时，两路径共享变量。
**How to avoid:** 响应体处理分支的外层 `match upstream.protocol_type` 优先——Anthropic 直接走透传，`is_streaming` 变量只在 `OpenAiChatCompletions` 分支内有语义。

### Pitfall 6: 模型映射在请求转换之前还是之后执行

**What goes wrong:** 若模型映射在 `anthropic_to_openai()` **之后**执行，则操作的是 OpenAI 格式的 body（`model` 字段仍存在），不影响正确性。但若在之前执行，则操作的是 Anthropic 格式（更符合语义）。
**Why it happens:** 两种顺序都可以工作，但先映射后转换更清晰：`anthropic_to_openai` 在 request.rs 第 17-19 行原样透传 `model` 字段，因此只要在转换前或转换后修改 body["model"] 都有效。
**How to avoid:** 按 CONTEXT.md 规定，在调用 `anthropic_to_openai()` **之前**执行模型映射，语义最清晰。

---

## Code Examples

### 完整 apply_upstream_model_mapping 函数（含测试）

```rust
// 放置位置：handler.rs 私有辅助函数，或 proxy/model_mapping.rs
use std::collections::HashMap;
use serde_json::Value;
use crate::proxy::state::UpstreamTarget;

fn apply_upstream_model_mapping(mut body: Value, upstream: &UpstreamTarget) -> Value {
    let original_model = match body.get("model").and_then(|m| m.as_str()) {
        Some(m) => m.to_string(),
        None => return body,
    };

    // 1. 精确匹配
    if let Some(ref map) = upstream.upstream_model_map {
        if let Some(mapped) = map.get(&original_model) {
            body["model"] = serde_json::json!(mapped);
            return body;
        }
    }

    // 2. 默认模型（无精确匹配时）
    if let Some(ref default_model) = upstream.upstream_model {
        body["model"] = serde_json::json!(default_model);
        return body;
    }

    // 3. 无映射，保留原模型名
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use crate::provider::ProtocolType;

    fn make_target_with_mapping() -> UpstreamTarget {
        let mut map = HashMap::new();
        map.insert("claude-3-5-sonnet-20241022".to_string(), "gpt-4o".to_string());
        UpstreamTarget {
            api_key: "sk-test".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            protocol_type: ProtocolType::OpenAiChatCompletions,
            upstream_model: Some("gpt-4o-mini".to_string()),
            upstream_model_map: Some(map),
        }
    }

    #[test]
    fn test_model_exact_match_wins_over_default() {
        let target = make_target_with_mapping();
        let body = json!({"model": "claude-3-5-sonnet-20241022", "messages": []});
        let result = apply_upstream_model_mapping(body, &target);
        assert_eq!(result["model"], "gpt-4o");  // 精确匹配
    }

    #[test]
    fn test_model_fallback_to_upstream_model() {
        let target = make_target_with_mapping();
        let body = json!({"model": "claude-3-opus-20240229", "messages": []});  // 无精确匹配
        let result = apply_upstream_model_mapping(body, &target);
        assert_eq!(result["model"], "gpt-4o-mini");  // 退回默认
    }

    #[test]
    fn test_model_preserved_when_no_mapping() {
        let target = UpstreamTarget {
            api_key: "sk-test".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            protocol_type: ProtocolType::OpenAiChatCompletions,
            upstream_model: None,
            upstream_model_map: None,
        };
        let body = json!({"model": "claude-3-5-sonnet-20241022"});
        let result = apply_upstream_model_mapping(body, &target);
        assert_eq!(result["model"], "claude-3-5-sonnet-20241022");  // 保留原名
    }
}
```

### 集成测试结构（mock server 方式）

```rust
// proxy/mod.rs 测试区 — 新增 OpenAiChatCompletions 集成测试
#[tokio::test]
async fn test_openai_compatible_non_streaming_roundtrip() {
    // mock 上游：接收 OpenAI 格式请求，返回 OpenAI 非流式响应
    let mock_openai_resp = json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Hello!"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
    });

    let (upstream_port, shutdown) = start_mock_upstream(mock_openai_resp).await;
    let service = ProxyService::new();

    // 构造 OpenAiChatCompletions 上游
    let upstream = UpstreamTarget {
        api_key: "sk-test".to_string(),
        base_url: format!("http://127.0.0.1:{}", upstream_port),
        protocol_type: ProtocolType::OpenAiChatCompletions,
        upstream_model: None,
        upstream_model_map: None,
    };
    service.start("claude", 0, upstream).await.unwrap();

    let proxy_port = service.status().await.servers[0].port;

    // 发送 Anthropic 格式请求
    let anthropic_req = json!({
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 1024,
        "messages": [{"role": "user", "content": "Hello"}]
    });

    let resp: Value = test_client()
        .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
        .header("x-api-key", "PROXY_MANAGED")
        .header("content-type", "application/json")
        .json(&anthropic_req)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // 验证响应为 Anthropic 格式
    assert_eq!(resp["content"][0]["type"], "text");
    assert_eq!(resp["content"][0]["text"], "Hello!");
    assert_eq!(resp["stop_reason"], "end_turn");

    service.stop("claude").await.unwrap();
    let _ = shutdown.send(());
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| handler.rs 完全透传，无转换 | handler.rs 按 ProtocolType 路由，OpenAiChatCompletions 走转换路径 | Phase 15 | 实现 ROUT-01/02，Anthropic 路径零回归 |
| UpstreamTarget 无模型映射字段 | UpstreamTarget 携带 upstream_model + upstream_model_map | Phase 15 | 实现 MODL-03，handler 无需查询 Provider 存储 |
| base_url 在 build_upstream_target 中 strip path | base_url 直接保留（含路径前缀） | Phase 15 | 使 build_proxy_endpoint_url 正确工作于 openrouter.ai 等 Provider |

---

## Open Questions

1. **`is_streaming` 的检测：请求 body 字段 vs 响应 Content-Type**
   - What we know: Claude Code 发出的 Anthropic 请求中，流式请求包含 `"stream": true` 字段。`body_bytes` 在步骤 C 已读取，可直接解析。响应 Content-Type 方案需等到 `upstream_resp` 返回。
   - What's unclear: Claude Code 是否总是设置 `"stream"` 字段（即使为 false 时是否省略该字段）？
   - Recommendation: 从请求 body `"stream"` 字段读取，缺省视为 `false`（非流式）。这是最早可判断的时机，且 `anthropic_to_openai` 已正确透传 `stream` 字段到 OpenAI 请求（request.rs 第 66 行）。

2. **模型映射辅助函数的放置位置**
   - What we know: CONTEXT.md 给 Claude 裁量权决定 handler.rs 内部函数拆分方式。函数本身约 15 行，加上测试约 60 行。
   - Recommendation: 放置在 `handler.rs` 内部私有函数（`fn apply_upstream_model_mapping`），保持文件内聚。若后续 handler.rs 超过 300 行，可提取到 `proxy/model_mapping.rs`。

3. **`UpstreamTarget.base_url` 语义变更的测试兼容性**
   - What we know: `proxy/state.rs` 和 `proxy/mod.rs` 的测试用 `make_target()` / `make_upstream()` 构造 UpstreamTarget，使用 `https://api.anthropic.com`（无路径）。这些测试不受语义变更影响（Anthropic base_url 本就无路径）。
   - What's unclear: `commands/proxy.rs` 的 `test_build_upstream_target_from_provider_strips_legacy_path` 测试明确断言"legacy path 被 strip"。Phase 15 改为不 strip 后，此测试需要更新（断言变为"base_url 原样保留"）。
   - Recommendation: 更新该测试，将断言从 `assert_eq!(upstream.base_url, "https://api.openai.com")` 改为 `assert_eq!(upstream.base_url, "https://api.openai.com/v1")` 或直接删除该测试并新增两个更精确的测试。

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust 内置 `cargo test`（rustc test harness） |
| Config file | `src-tauri/Cargo.toml` |
| Quick run command | `/Users/kelin/.cargo/bin/cargo test --manifest-path /Users/kelin/Workspace/CLIManager/src-tauri/Cargo.toml --package cli-manager proxy::handler` |
| Full suite command | `/Users/kelin/.cargo/bin/cargo test --manifest-path /Users/kelin/Workspace/CLIManager/src-tauri/Cargo.toml --package cli-manager` |

**基线状态（Phase 14 验证后）:** 295 个测试，294 passed，1 failed（`test_proxy_enable_patches_cli_and_starts_proxy`，UX-01 遗留端口冲突，与 Phase 15 代码无关）。

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ROUT-01 | OpenAiChatCompletions 请求经转换路径，响应返回 Anthropic 格式 | integration | `cargo test --package cli-manager proxy::mod::tests::test_openai_compatible_non_streaming_roundtrip` | ❌ Wave 0 |
| ROUT-01 | OpenAiChatCompletions 流式请求，响应返回 Anthropic SSE 流 | integration | `cargo test --package cli-manager proxy::mod::tests::test_openai_compatible_streaming_roundtrip` | ❌ Wave 0 |
| ROUT-02 | Anthropic 请求零回归（现有透传测试继续通过） | integration | `cargo test --package cli-manager proxy::mod::tests` | ✅ 已有透传测试 |
| MODL-03 | 精确匹配优先：upstream_model_map 中有对应条目时替换模型名 | unit | `cargo test --package cli-manager proxy::handler::tests::test_model_exact_match_wins_over_default` | ❌ Wave 0 |
| MODL-03 | 无精确匹配时退回 upstream_model（默认模型） | unit | `cargo test --package cli-manager proxy::handler::tests::test_model_fallback_to_upstream_model` | ❌ Wave 0 |
| MODL-03 | upstream_model 和 upstream_model_map 均为 None 时保留原模型名 | unit | `cargo test --package cli-manager proxy::handler::tests::test_model_preserved_when_no_mapping` | ❌ Wave 0 |
| MODL-03 | 模型映射在 anthropic_to_openai() 调用前执行 | unit（通过集成测试间接验证） | `cargo test --package cli-manager proxy::mod::tests::test_model_mapping_applied_before_translate` | ❌ Wave 0 |
| ROUT-01+ROUT-02 | Anthropic 现有集成测试零回归（`test_proxy_service_*` 系列全部通过） | integration | `cargo test --package cli-manager proxy::mod::tests` | ✅ 已有 |

### Sampling Rate

- **Per task commit:** `/Users/kelin/.cargo/bin/cargo test --manifest-path /Users/kelin/Workspace/CLIManager/src-tauri/Cargo.toml --package cli-manager proxy::handler proxy::mod proxy::state`
- **Per wave merge:** `/Users/kelin/.cargo/bin/cargo test --manifest-path /Users/kelin/Workspace/CLIManager/src-tauri/Cargo.toml --package cli-manager`（全套 295+ 测试，期望 294 passed 或更多）
- **Phase gate:** Full suite green（含新增集成测试）before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src-tauri/src/proxy/handler.rs` — 新增 `apply_upstream_model_mapping()` 私有函数 + 3 个 unit tests
- [ ] `src-tauri/src/proxy/state.rs` — `UpstreamTarget` 新增 2 个字段 + 更新 `make_target()` 辅助函数
- [ ] `src-tauri/src/proxy/mod.rs` — 新增 2 个 OpenAiChatCompletions 集成测试（非流式 + 流式），更新 `make_upstream()` 辅助函数
- [ ] `src-tauri/src/commands/proxy.rs` — 更新 `build_upstream_target_from_provider()` + 所有 `UpstreamTarget {}` 构造点（约 5 处） + 更新 `test_build_upstream_target_from_provider_strips_legacy_path` 测试

*(更新已有文件为主；无需创建新文件，translate/ 模块已在 Phase 14 完整建立)*

---

## Sources

### Primary (HIGH confidence)

- `src-tauri/src/proxy/handler.rs`（133 行）— 当前 handler 完整实现，步骤 A-J 确认
- `src-tauri/src/proxy/state.rs`（117 行）— UpstreamTarget 当前字段确认，构造点
- `src-tauri/src/proxy/translate/request.rs`（765 行）— `anthropic_to_openai()`、`build_proxy_endpoint_url()` 签名确认
- `src-tauri/src/proxy/translate/response.rs`（第 35 行）— `openai_to_anthropic()` 签名确认
- `src-tauri/src/proxy/translate/stream.rs`（第 170 行）— `create_anthropic_sse_stream()` 签名确认
- `src-tauri/src/proxy/error.rs`（第 35 行）— `ProxyError::TranslateError` 确认返回 400
- `src-tauri/src/provider.rs`（L114-117）— `Provider.upstream_model` / `upstream_model_map` 字段确认
- `src-tauri/src/commands/proxy.rs`（L43-49）— `build_upstream_target_from_provider` 当前实现确认
- Phase 14 VERIFICATION.md — 20/20 验证通过，转换模块就绪状态确认
- 实际测试运行 — 295 tests, 294 passed（基线确认）

### Secondary (MEDIUM confidence)

- `cc-switch/src-tauri/src/proxy/model_mapper.rs` — 参考实现（cc-switch 的映射逻辑复杂，CLIManager 方案更简洁，仅参考）
- Phase 15 CONTEXT.md — 集成点、决策锁定

### Tertiary (LOW confidence)

- 无

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 所有所需 crate 在 Phase 14 已添加，API 从源码直接确认
- Architecture: HIGH — handler.rs 步骤结构从源码直接分析，插入点明确
- Pitfalls: HIGH — UpstreamTarget 多构造点从 grep 直接统计，base_url 语义问题从 extract_origin_base_url 实现直接分析

**Research date:** 2026-03-14
**Valid until:** 2026-04-14（依赖稳定，代码在同一仓库，30 天有效）
