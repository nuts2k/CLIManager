# Phase 28: 流式 SSE Token 提取 - Research

**Researched:** 2026-03-18
**Domain:** Rust 异步流 + SSE 协议解析 + SQLite 更新 + Tauri emit
**Confidence:** HIGH（所有核心结论均来自直接阅读已存在的代码）

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- Stream EOF 后统一解析 token，不在中途提取
- Phase 27 已为流式请求写入基础元数据（token/duration/ttfb 留 null），Phase 28 在 EOF 后 UPDATE 同一行填充
- UPDATE 完成后通过 `traffic-log` 事件 emit type="update" 更新前端
- **Anthropic 原始流**：`message_delta` 事件的 `usage` 字段包含 input_tokens/output_tokens/cache_creation_input_tokens/cache_read_input_tokens；`message_delta.delta.stop_reason` 含停止原因
- **OpenAI Chat Completions 流**：含 `finish_reason` 的最后一个 chunk 中 `usage` 字段包含 prompt_tokens/completion_tokens；缓存在 `usage.prompt_tokens_details.cached_tokens`；stream.rs 中已有 `Usage` 结构体和 `extract_cache_read_tokens()` 函数
- **OpenAI Responses API 流**：`response.completed` 事件的 `response.usage` 包含 input_tokens/output_tokens；缓存字段在 `response.usage.input_token_details.cached_tokens`
- TTFB：从 handler 中 `upstream_resp` 返回的时间点（`send().await` 完成即为 TTFB）
- Duration：handler 全生命周期（含流式 stream 全部传输）
- stop_reason 存原始值，不做跨协议映射
- Phase 26 已预留 cache_creation_tokens 和 cache_read_tokens 列，Phase 28 对三协议流式响应均提取缓存 token

### Claude's Discretion

- 流内 token 数据累积的具体实现方式（channel、闭包回调、Arc<Mutex> 等）
- UPDATE SQL 的行定位标识（rowid、created_at 组合等）
- TTFB 时间点如何从 handler 传递到流结束时的 UPDATE 逻辑
- Responses API 缓存字段是否实际存在于流式 response.completed 事件中（不存在时优雅降级为 None）

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| COLLECT-03 | 流式 SSE 响应在 stream 结束后提取 token 用量（支持 Anthropic、OpenAI Chat Completions、OpenAI Responses 三种格式） | 三协议的 token 字段位置已在 CONTEXT.md 和代码中确认；UPDATE 路径需新增 `update_streaming_log()` 方法；回传机制选型在 Claude's Discretion |
</phase_requirements>

---

## Summary

Phase 28 的核心工作是为三种流式处理路径增加"流结束后回传 token → UPDATE DB → emit 前端"的机制。当前代码（handler.rs 第 709-738 行）在流式分支提前发送 LogEntry 时，token 字段全为 None，duration 也为 None。Phase 28 需要在 stream 消费完成后补齐这些字段。

三种流处理函数（`create_anthropic_sse_stream`、`create_responses_anthropic_sse_stream`、`create_anthropic_reverse_model_stream`）均已内部解析了 token 数据——其中前两个已在 message_delta/response.completed 中提取并输出为 SSE 事件，第三个（纯 Anthropic 透传）尚未解析 usage。所有三个函数当前只作字节流转换，没有办法向 handler 回传 token 数值。

核心挑战是：流函数内部的 token 数据如何传递到 handler 层（流启动之后、stream 消费完成之前 handler 已经把 `Body` 返回给客户端了）。解决方案是在流函数签名中引入一个回传机制，在 stream EOF 时写入值，handler 在 `Body::from_stream(...)` 之后通过后台任务监听该值并执行 UPDATE。

**Primary recommendation:** 使用 `tokio::sync::oneshot` channel 作为 token 回传机制——流函数在 EOF 时 `send(TokenData)`，handler 启动后台 task 通过 `rx.await` 等待并执行 UPDATE+emit。这是最简洁且不引入 Arc<Mutex> 复杂度的方案。

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio::sync::oneshot | （tokio 已有） | 流 EOF 时单次回传 token 数据 | 恰好语义匹配：单个发送方、单个接收方、只发一次 |
| rusqlite | （已有） | UPDATE SQL 执行 | 项目已有，TrafficDb 已封装 |
| tauri::Emitter | （已有） | emit type="update" 到前端 | log_worker 中已有相同用法 |
| async_stream | （已有） | 三个流函数均已使用 | 项目已有 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio::task::spawn | （tokio 已有） | 在 handler 返回后继续等待流结束 | 流式分支必须用后台 task，否则 handler 无法先返回响应 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| oneshot channel | Arc<Mutex<Option<TokenData>>> | Mutex 方案需轮询或额外通知机制，oneshot 天然阻塞到 EOF |
| oneshot channel | 闭包回调 | 闭包方案需 Send + 'static 约束，在 async_stream 宏内更难满足 |
| oneshot channel | mpsc channel（已有 log_tx） | mpsc 需要定义新的消息类型并修改 log_worker；oneshot 可独立工作 |

**Installation:** 无需新增依赖，tokio/rusqlite/tauri 均已在项目中。

---

## Architecture Patterns

### Recommended Project Structure

修改涉及以下已有文件（无需新建模块）：

```
src-tauri/src/
├── traffic/
│   └── log.rs              # 新增 update_streaming_log() 方法
├── proxy/
│   ├── handler.rs          # 流式分支：记录 row_id + ttfb，启动后台 task
│   └── translate/
│       ├── stream.rs       # create_anthropic_sse_stream：增加 oneshot tx 参数
│       ├── responses_stream.rs  # create_responses_anthropic_sse_stream：增加 oneshot tx 参数
│       └── handler.rs      # create_anthropic_reverse_model_stream：增加 Anthropic token 解析 + oneshot tx 参数
```

### Pattern 1: oneshot 回传 + 后台 task

**What:** 流函数接受一个 `oneshot::Sender<StreamTokenData>` 参数，在 EOF（message_stop/response.completed/[DONE]）时 `send(data)`。handler 在启动流后 `tokio::spawn` 一个 task 等待 `rx.await`，收到后执行 UPDATE + emit。

**When to use:** 适用于"流函数在另一个 axum 任务中消费，但 handler 需要在流结束后做额外操作"的场景。

**Example:**

```rust
// 在 translate/stream.rs 中（示意，非精确代码）
pub struct StreamTokenData {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub stop_reason: Option<String>,
}

pub fn create_anthropic_sse_stream(
    upstream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
    model: String,
    token_tx: tokio::sync::oneshot::Sender<StreamTokenData>,  // 新增
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send {
    async_stream::stream! {
        // ... 现有逻辑 ...
        // finish_reason chunk 中：
        // let data = StreamTokenData { ... };
        // let _ = token_tx.send(data);  // 忽略 send 失败（客户端已断开时正常）
    }
}

// 在 handler.rs 流式分支中（示意）
let (token_tx, token_rx) = tokio::sync::oneshot::channel::<StreamTokenData>();
let stream = translate::stream::create_anthropic_sse_stream(
    upstream_resp.bytes_stream(),
    request_model,
    token_tx,
);
let body = Body::from_stream(stream);

// 记录 TTFB（upstream_resp.send().await 完成时即为 TTFB，在步骤 H 之后）
let ttfb_ms = start_time.elapsed().as_millis() as i64;

// 发送基础 LogEntry（token=null）并获取 row_id
// ... existing log send code, but capture row_id ...

// 启动后台 task 等待 stream EOF
let app_handle_clone = state.app_handle.clone();
let row_id_for_update = row_id;
let start_time_clone = start_time;
tokio::spawn(async move {
    if let Ok(token_data) = token_rx.await {
        let duration_ms = start_time_clone.elapsed().as_millis() as i64;
        // UPDATE DB
        // emit type="update"
    }
});
```

### Pattern 2: TrafficDb.update_streaming_log()

**What:** 在 `traffic/log.rs` 中为 `TrafficDb` 新增 `update_streaming_log()` 方法，按 rowid 更新 token/ttfb/duration/stop_reason 字段。

**When to use:** 流式请求 stream 结束后调用，与 `insert_request_log()` 配套。

**Example:**

```rust
// 在 traffic/log.rs 中新增（示意）
impl TrafficDb {
    pub fn update_streaming_log(
        &self,
        id: i64,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
        cache_creation_tokens: Option<i64>,
        cache_read_tokens: Option<i64>,
        stop_reason: Option<&str>,
        ttfb_ms: Option<i64>,
        duration_ms: Option<i64>,
    ) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE request_logs SET
                input_tokens = ?1,
                output_tokens = ?2,
                cache_creation_tokens = ?3,
                cache_read_tokens = ?4,
                stop_reason = ?5,
                ttfb_ms = ?6,
                duration_ms = ?7
             WHERE id = ?8",
            rusqlite::params![
                input_tokens, output_tokens,
                cache_creation_tokens, cache_read_tokens,
                stop_reason, ttfb_ms, duration_ms, id
            ],
        )?;
        Ok(())
    }
}
```

### Anti-Patterns to Avoid

- **在流函数内直接访问 TrafficDb：** 流函数是 translate 模块的职责，不应知道 traffic 层的存在。token 数据只负责"回传"，更新由 handler 或后台 task 执行。
- **在 Body::from_stream 之后同步等待：** handler 必须先返回 `Response`（body = stream），不能阻塞等待 stream 消费完。必须用 `tokio::spawn`。
- **UPDATE 行定位用 created_at：** created_at 是 epoch ms，理论上可能碰撞。应使用 `insert_request_log` 返回的 rowid（`last_insert_rowid()`）定位，唯一且无碰撞风险。
- **忽略 oneshot send 失败：** 客户端主动断开连接时 stream 提前终止，`token_tx.send()` 可能失败（receiver 已 drop）。应用 `let _ = token_tx.send(data)` 忽略，不 panic。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 流结束通知 | 自定义 flag + 轮询 | `tokio::sync::oneshot` | 语义精确，无需轮询，天然 Send + 'static |
| 后台 DB 操作 | 在流函数内直接调用 rusqlite | `tokio::spawn` + `TrafficDb.update_streaming_log()` | 保持 translate 层不依赖 traffic 层 |
| UPDATE 行定位 | 按 created_at/provider_name 组合查询 | rowid（`last_insert_rowid()`） | 唯一性有保证 |
| TTFB 计算 | 在流函数内计时 | `start_time.elapsed()` 在 `upstream_resp = send().await` 之后立即采样 | `send().await` 返回即意味着第一字节到达，已有 `start_time` |

---

## Common Pitfalls

### Pitfall 1：handler 已 move upstream_resp，但需要在 Body 之后访问 TrafficDb

**What goes wrong:** `Body::from_stream(stream)` 之后，handler 到 `builder.body(body)` 就 return 了。后台 task 需要访问 `app_handle`（`TrafficDb` 的持有者），但 `state` 在 return 后可能不可用。

**Why it happens:** `ProxyState` 是 axum State 提取器，其 clone 成本低（内部是 Arc）。

**How to avoid:** 在启动后台 task 之前 `let app_handle_clone = state.app_handle.clone()`（如果 ProxyState 持有 app_handle）；或者直接传入 `app_handle` 引用的 clone。需确认 `ProxyState` 是否已持有 `tauri::AppHandle`，若没有则需要在 `ProxyState` 中增加该字段。

**Warning signs:** 编译时 `move` 冲突；运行时 `try_state` 返回 None。

### Pitfall 2：create_anthropic_reverse_model_stream 当前不解析 Anthropic usage

**What goes wrong:** `AnthropicPassthrough` 流式分支的处理函数 `create_anthropic_reverse_model_stream` 只做逐行 model 字段替换，不解析 `message_delta` 事件中的 `usage` 字段。如果直接加 oneshot 但不增加解析，回传的 token 将全为 None。

**Why it happens:** 该函数的原始职责只是 model 名反向映射，没有 token 解析需求（Phase 27 之前）。

**How to avoid:** 在该函数的 `data: {...}` 行处理逻辑中，检测 `"type": "message_delta"` 事件，提取 `usage.input_tokens`、`usage.output_tokens`、`usage.cache_creation_input_tokens`、`usage.cache_read_input_tokens` 和 `delta.stop_reason`。可参考已有的 `extract_anthropic_tokens()` 的字段路径。

**Warning signs:** Anthropic 协议流式日志 token 字段始终为 null。

### Pitfall 3：log_worker 中需要同时处理 INSERT（新请求）和 UPDATE（流完成）

**What goes wrong:** 当前 `log_worker` 只处理 `LogEntry`（对应 INSERT）。若要复用同一 channel 传递 UPDATE 消息，需修改 channel 类型或增加新枚举。

**Why it happens:** Phase 27 设计时只考虑了 INSERT 场景。

**How to avoid:** 两个选项：
1. **不复用 log_worker**：后台 task 直接调用 `app_handle.try_state::<TrafficDb>()` 并执行 UPDATE（类似非 worker 模式，但只有一个连接且是 Mutex，直接调用即可）。
2. **扩展 log_worker**：将 channel 类型改为枚举 `LogWorkerMsg { Insert(LogEntry), Update { id, tokens... } }`。

**Recommendation:** 方案 1 更简单，后台 task 直接拿 TrafficDb state 执行。std::sync::Mutex<rusqlite::Connection> 在 tokio 任务中可以短暂持有（不长时间持有即可）。

### Pitfall 4：TTFB 采样时机

**What goes wrong:** TTFB 应是"上游第一字节到达时间"，即 `upstream_resp = req_builder.send().await` 完成的时刻（reqwest 的 `send()` 返回时上游已响应了 HTTP 头，第一字节准备就绪）。若在 stream 消费开始时才采样则偏晚。

**Why it happens:** `send().await` 完成意味着上游已开始响应。

**How to avoid:** 在 handler.rs 步骤 H（`let upstream_resp = req_builder.send().await`）完成后立即：
```rust
let ttfb_ms = start_time.elapsed().as_millis() as i64;
```
然后在 INSERT 时填入 ttfb_ms（流式和非流式都可以在这里计算）。注意当前代码注释 `ttfb_ms: None, // Phase 27 非流式不计 TTFB，Phase 28 处理` 确认了这个时机。

### Pitfall 5：Responses API 缓存字段可能不在流式响应中出现

**What goes wrong:** CONTEXT.md 指出 `response.usage.input_token_details.cached_tokens` 可能不存在于实际流式 response.completed 事件。

**Why it happens:** OpenAI Responses API 规范可能未保证流式响应携带该字段。

**How to avoid:** 用 `Option` chain 提取，不存在时返回 `None`（而非 0）。当前 `responses_stream.rs` 第 322-333 行的提取代码已经用 `.unwrap_or(0)` 处理缺失情况——Phase 28 需改为 `Option` 保留 None 而非 0（0 和 None 语义不同：0 表示"有数据且为零"，None 表示"未提取到"）。

---

## Code Examples

从已有代码中确认的正确模式：

### OpenAI Chat Completions 流式 token 提取位置

```rust
// src-tauri/src/proxy/translate/stream.rs 第 534-546 行
// message_delta 事件中提取 usage（在 finish_reason chunk 处）
let usage_val: Value = chunk_val.usage.as_ref().map(|u| {
    let mut uj = json!({
        "input_tokens": u.prompt_tokens,       // 对应 log 的 input_tokens
        "output_tokens": u.completion_tokens   // 对应 log 的 output_tokens
    });
    if let Some(cached) = extract_cache_read_tokens(u) {
        uj["cache_read_input_tokens"] = json!(cached);  // cache_read_tokens
    }
    if let Some(created) = u.cache_creation_input_tokens {
        uj["cache_creation_input_tokens"] = json!(created);  // cache_creation_tokens
    }
    uj
}).unwrap_or(Value::Null);
```

**关键**：`chunk_val.usage` 仅在含 `finish_reason` 的 chunk 中出现，与 `finish_reason` 在同一 JSON 对象的顶层（不在 choices 内）。

### Responses API 流式 token 提取位置

```rust
// src-tauri/src/proxy/translate/responses_stream.rs 第 322-333 行
// response.completed 事件中提取
let input_tokens = data
    .pointer("/response/usage/input_tokens")
    .and_then(|v| v.as_u64())
    .unwrap_or(0);  // Phase 28 改为 and_then(|v| v.as_i64()) 返回 Option<i64>
let output_tokens = data
    .pointer("/response/usage/output_tokens")
    .and_then(|v| v.as_u64())
    .unwrap_or(0);
// 缓存字段路径（待验证是否实际出现）：
// data.pointer("/response/usage/input_token_details/cached_tokens")
```

### Anthropic 原始流 token 字段路径

```rust
// message_delta 事件 JSON 结构（参考 handler.rs 的 extract_anthropic_tokens）
// {
//   "type": "message_delta",
//   "delta": { "stop_reason": "end_turn", ... },
//   "usage": {
//     "input_tokens": 100,
//     "output_tokens": 50,
//     "cache_creation_input_tokens": 10,  // 可能不存在
//     "cache_read_input_tokens": 5        // 可能不存在
//   }
// }
//
// 注意：create_anthropic_reverse_model_stream 在逐行扫描中需要解析这个事件
// 可直接复用 serde_json::from_str 后的 pointer() 访问路径
```

### log_worker 中的 emit 模式（type="new"，Phase 28 需 type="update"）

```rust
// src-tauri/src/traffic/log.rs 第 163-178 行
pub async fn log_worker(mut rx: mpsc::Receiver<LogEntry>, app_handle: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    while let Some(entry) = rx.recv().await {
        if let Some(db) = app_handle.try_state::<super::TrafficDb>() {
            match db.insert_request_log(&entry) {
                Ok(id) => {
                    let payload = TrafficLogPayload::from_entry(id, &entry, "new");
                    if let Err(e) = app_handle.emit("traffic-log", &payload) {
                        log::warn!("emit traffic-log 失败: {}", e);
                    }
                }
                Err(e) => log::warn!("写入 request_logs 失败: {}", e),
            }
        }
    }
}
// Phase 28 后台 task 中需要类似：
// db.update_streaming_log(id, ...)?;
// app_handle.emit("traffic-log", &payload_with_type_update)?;
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 流式请求 token=null 永久 | stream EOF 后 UPDATE 填充 | Phase 28 | 日志从"进行中"变为"完成"，前端可正确显示 token |
| 只有 INSERT 日志操作 | INSERT + UPDATE 双操作 | Phase 28 | TrafficDb 新增 update 方法，log_worker 不改动 |
| 无 TTFB 记录 | TTFB 在 send().await 后立即采样 | Phase 28 | 流式和非流式请求均填充 ttfb_ms |

**Phase 27 遗留注释（Phase 28 需处理）:**
- `handler.rs:726` — `ttfb_ms: None, // Phase 27 非流式不计 TTFB，Phase 28 处理`
- `handler.rs:727-729` — `duration_ms: if upstream_is_sse { None } else { ... }` 流式留 None

---

## Open Questions

1. **ProxyState 不含 tauri::AppHandle——后台 task 的 emit 途径需设计**（已确认）
   - What we know：已读 `proxy/state.rs`（第 24-31 行）——`ProxyState` 只有 `upstream`、`http_client`、`log_tx`、`cli_id` 四个字段，无 `app_handle`。`log_worker` 中的 emit 通过传入参数实现。
   - What's unclear：后台 task（`tokio::spawn`）如何拿到 `app_handle` 执行 emit。
   - Recommendation：最简方案是在 `ProxyState` 增加 `app_handle: Option<tauri::AppHandle>` 字段（cost：clone 成本低，AppHandle 内部是 Arc）。替代方案：将 UPDATE+emit 的消息通过扩展后的 log channel 发送给 log_worker（需改 channel 类型为枚举）。推荐方案一（在 ProxyState 加 app_handle），与 Phase 27 的 try_state 模式保持一致。

2. **Responses API 缓存字段实际格式**
   - What we know：CONTEXT.md 说字段在 `response.usage.input_token_details.cached_tokens`；当前代码未提取缓存（responses_stream.rs 第 321-334 行只提取 input/output）。
   - What's unclear：该字段在流式响应实际是否出现（需要实际 API 调用验证，或查阅 OpenAI Responses API 文档）。
   - Recommendation：实现时用 Option chain 提取，字段不存在时 None，不阻塞实现。

3. **create_anthropic_reverse_model_stream 当前是按行处理还是按 SSE block？**
   - What we know：从 handler.rs 第 172-223 行看，该函数按 `\n` 分割逐行处理，每行独立替换 model 字段。Anthropic SSE 的 `message_delta` 事件是 `data: {...}\n\n` 格式，data 行是完整单行 JSON。
   - What's unclear：在行缓冲模式下是否能可靠捕获完整的 `data: {...}` 行（理论上可以，buffer 保证了完整性）。
   - Recommendation：在 `type == "message_delta"` 时提取 usage，逻辑与现有 model 替换并列即可。

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust 内置 `#[cfg(test)]` + tokio::test |
| Config file | Cargo.toml（无独立测试配置文件） |
| Quick run command | `cargo test -p cli-manager-lib 2>&1` |
| Full suite command | `cargo test --workspace 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| COLLECT-03 | OpenAI Chat Completions 流式 token 回传（Usage 在 finish_reason chunk） | unit | `cargo test -p cli-manager-lib stream::tests 2>&1` | ✅（stream.rs 含测试模块） |
| COLLECT-03 | Responses API 流式 token 回传（response.completed 提取） | unit | `cargo test -p cli-manager-lib responses_stream::tests 2>&1` | ✅（responses_stream.rs 含测试模块） |
| COLLECT-03 | Anthropic 透传流式 token 回传（message_delta 解析） | unit | `cargo test -p cli-manager-lib handler::tests 2>&1` | ✅（handler.rs 含测试模块） |
| COLLECT-03 | update_streaming_log UPDATE 字段正确性 | unit | `cargo test -p cli-manager-lib traffic::log::tests 2>&1` | ✅（log.rs 含测试模块） |
| COLLECT-03 | oneshot tx.send / rx.await 端到端 token 传递 | unit | `cargo test -p cli-manager-lib -- streaming_token 2>&1` | ❌ Wave 0 需新增 |

### Sampling Rate

- **Per task commit:** `cargo test -p cli-manager-lib 2>&1`
- **Per wave merge:** `cargo test --workspace 2>&1`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `stream.rs` — 新增测试：`test_token_callback_on_eof`（验证 oneshot tx 在 finish_reason 时被调用，携带正确 token 值）
- [ ] `responses_stream.rs` — 新增测试：`test_token_callback_on_response_completed`（验证 response.completed 时 tx 被调用）
- [ ] `handler.rs` / 新模块 — `create_anthropic_reverse_model_stream` 扩展后的 token 解析测试
- [ ] `traffic/log.rs` — 新增测试：`test_update_streaming_log`（INSERT 后 UPDATE，验证 token 字段正确填充）

---

## Sources

### Primary (HIGH confidence)
- `/Users/kelin/Work/CLIManager/src-tauri/src/proxy/handler.rs` — 直接阅读：流式分支代码结构（第 603-707 行）、日志发送模式（第 709-738 行）、TTFB 注释、三协议路由
- `/Users/kelin/Work/CLIManager/src-tauri/src/proxy/translate/stream.rs` — 直接阅读：Usage 结构体（第 59-77 行）、extract_cache_read_tokens（第 145-156 行）、finish_reason chunk 中 usage 提取位置（第 534-555 行）
- `/Users/kelin/Work/CLIManager/src-tauri/src/proxy/translate/responses_stream.rs` — 直接阅读：response.completed 中 usage 提取（第 307-351 行）、缓存字段未提取现状
- `/Users/kelin/Work/CLIManager/src-tauri/src/traffic/log.rs` — 直接阅读：TrafficDb.insert_request_log、log_worker emit 模式、TrafficLogPayload 结构
- `.planning/phases/28-sse-token/28-CONTEXT.md` — 用户决策，token 字段位置、UPDATE 策略

### Secondary (MEDIUM confidence)
- `.planning/STATE.md` — 积累的决策：std::sync::Mutex<Connection>、log_tx 持有方式

### Tertiary (LOW confidence)
- Responses API 缓存字段（`input_token_details.cached_tokens`）在流式响应中是否实际出现——未通过实际 API 调用验证，基于 CONTEXT.md 用户调研结论

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — 所有库均已在项目中，tokio::sync::oneshot 是 tokio 标准组件
- Architecture: HIGH — 直接从代码阅读得出，关键路径（handler 流式分支、流函数签名）均已确认
- Pitfalls: HIGH — 大多来自代码直接观察（Phase 27 遗留 None、逐行流函数无 usage 解析、TTFB 时机注释）

**Research date:** 2026-03-18
**Valid until:** 2026-04-18（项目内部代码变化时失效）
