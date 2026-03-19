# Phase 27: 日志写入管道 - Research

**Researched:** 2026-03-18
**Domain:** Rust/Tauri mpsc channel + SQLite 写入 + Tauri 事件推送
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**流式请求日志策略**
- Phase 27 为所有请求（含流式）写入基础元数据，流式请求的 token 字段（input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens）和耗时字段（duration_ms, ttfb_ms）留 null
- Phase 28 在 stream EOF 后 UPDATE 同一行，填充 token 和耗时字段
- 流式请求写入基础日志后立即 emit 到前端（token/duration 为 null），Phase 28 补充后再 emit 一次更新事件
- 统一使用 `traffic-log` 事件名，payload 含 `type` 字段区分 `"new"` 和 `"update"`，前端用同一个 listener 根据 type 决定 append 还是 update row

**非流式 Token + 缓存 Token 提取**
- 非流式响应在 handler 中直接解析 usage 字段，提取 input_tokens、output_tokens、cache_creation_tokens、cache_read_tokens（不通过 mpsc 传输 body）
- 三种协议的非流式 token 提取全部在 Phase 27 实现：
  - Anthropic: usage.input_tokens, usage.output_tokens, usage.cache_creation_input_tokens, usage.cache_read_input_tokens
  - OpenAI Chat Completions: usage.prompt_tokens, usage.completion_tokens, usage.prompt_tokens_details.cached_tokens
  - OpenAI Responses: usage.input_tokens, usage.output_tokens（缓存字段位置由研究阶段确认）
- stop_reason/finish_reason 存原始值，不做跨协议映射（Anthropic 存 end_turn/max_tokens 等，OpenAI 存 stop/length 等），保留协议差异便于调试

**事件 Payload 设计**
- traffic-log 事件 payload 携带 request_logs 表完整 19 列 + type 字段（"new"/"update"）
- 时间戳保持 epoch 毫秒（与 DB 一致），前端用 new Date(ts) 转换显示
- 前端初始加载通过 Tauri command 查询最近 100 条日志（双轨策略：command 初始拉取 + event 增量追加）
- Phase 27 提供基础查询 command（按 created_at 降序，LIMIT 参数）

### Claude's Discretion
- mpsc channel buffer 大小
- handler 中计时逻辑的具体实现方式（Instant::now 位置）
- TrafficDb/AppHandle/mpsc sender 在 ProxyState 中的注入方式
- 错误日志的 error_message 格式化方式
- 查询 command 的具体函数签名和过滤参数

### Deferred Ideas (OUT OF SCOPE)
- 流式 SSE 的 token + 缓存字段提取 -- Phase 28
- 流式请求的 duration_ms 和 ttfb_ms 计算 -- Phase 28
- 前端流量监控页面 -- Phase 29
- 统计聚合与数据保留 -- Phase 30
- 费用估算 (cost_usd) -- v2.7+ (ADV-01)
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| STORE-03 | 代理请求完成后通过 mpsc channel 非阻塞发送日志，后台 task 写入 SQLite 并 emit 到前端 | mpsc channel 设计、AppHandle 注入、后台 spawn 模式已研究完毕 |
| COLLECT-01 | 记录每个代理请求的基础元数据（时间戳、CLI、Provider、方法、路径、状态码、总耗时、TTFB、是否流式、请求模型名） | ProxyState 扩展方案（需加 provider_name 字段）、Instant 计时已确认 |
| COLLECT-02 | 非流式响应直接从 body 提取 input/output token 用量 | 三种协议的 usage 字段位置已逐一确认；handler.rs 已在非流式分支读取完整 body |
| COLLECT-04 | 请求失败时记录错误信息，成功时记录 stop_reason | ProxyError 枚举路径已确认；stop_reason 从原始响应/转换后响应两个来源提取 |
| LOG-01 | 后台写入 SQLite 后通过 Tauri emit 实时推送日志条目到前端 | AppHandle 注入方式、emit 模式参照 watcher/mod.rs 已确认 |
</phase_requirements>

---

## Summary

Phase 27 的核心工作是在现有代理 handler 中埋点采集请求元数据，通过 mpsc channel 非阻塞地将日志条目传递给后台写入 task，task 写入 SQLite 后立即通过 Tauri emit 推送 `traffic-log` 事件到前端。Phase 26 已完成 SQLite 基础设施（TrafficDb、schema、migration），Phase 27 在此基础上做三件事：（1）扩展 ProxyState 携带 provider_name 和 mpsc Sender；（2）在 handler 各分支埋点计时、token 提取和发送；（3）在 traffic 模块新增 `insert_request_log` 方法和查询 Tauri command。

**关键设计约束：** `UpstreamTarget` 当前不携带 `provider_name`，但 `request_logs` 表的 `provider_name` 列是 NOT NULL。必须在 `UpstreamTarget` 中新增 `provider_name` 字段，同时修改所有构造点（`build_upstream_target`、`build_upstream_target_from_provider`、watcher 中的构造代码）。这是 Phase 27 最重要的接口变更点。

**Primary recommendation:** 在 `UpstreamTarget` 中加 `provider_name: String`，在 `ProxyState` 中加 `log_tx: Option<mpsc::Sender<LogEntry>>`（克隆 sender），handler 中按 `ResponseTranslationMode` 分支提取 token，所有数据打包成 `LogEntry` 通过 sender 发送，后台 task 单独 `loop { recv + insert + emit }`。

---

## Standard Stack

### Core（已在 Cargo.toml，无需新增）

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.x | mpsc channel、async_runtime::spawn | 项目已用，Tauri 内置 |
| rusqlite | Phase 26 已引入 | SQLite 写入 | 单连接 + std::sync::Mutex 模式已确立 |
| serde / serde_json | 1.x | LogEntry 序列化供 Tauri emit | 项目标准，TrafficLogPayload 需 derive Serialize |
| tauri Emitter | 2.x | app_handle.emit("traffic-log", &payload) | watcher/mod.rs 中已有使用范例 |

### 无需新增依赖

Phase 27 所有功能均可用项目现有依赖实现：
- `tokio::sync::mpsc` — 已在 tokio features 中
- `std::time::Instant` — 标准库，用于 duration_ms 计时
- `std::time::SystemTime` — 标准库，用于 epoch ms 时间戳

---

## Architecture Patterns

### 推荐项目结构变更

```
src-tauri/src/
├── proxy/
│   ├── state.rs       # 扩展 ProxyState：加 log_tx, 扩展 UpstreamTarget 加 provider_name
│   └── handler.rs     # 埋点：start_time, 发 LogEntry
├── traffic/
│   ├── mod.rs         # 扩展：新增 log_worker 后台 task 函数
│   ├── db.rs          # 现有（不变）
│   ├── schema.rs      # 现有（不变）
│   └── log.rs         # 新建：LogEntry struct、insert_request_log()
└── commands/
    └── traffic.rs     # 新建：get_recent_logs Tauri command
lib.rs                 # 扩展 setup：创建 mpsc channel、spawn log_worker、inject sender
```

### Pattern 1: mpsc channel 写入管道

**What:** handler 通过 `sender.try_send(entry)` 发送（fire-and-forget），后台 task 通过 `receiver.recv().await` 消费并写入 SQLite + emit。

**When to use:** 任何需要非阻塞 IO 操作的代理 handler 场景。

**buffer 大小建议（Claude's Discretion）:** 1024。理由：`< 10 req/s` 的场景下 buffer 不会堆满；`try_send` 满 buffer 时直接 log warn 并丢弃（可接受），不阻塞 handler。

```rust
// lib.rs setup 闭包中
let (log_tx, mut log_rx) = tokio::sync::mpsc::channel::<LogEntry>(1024);

// 注入 sender 到 proxy 状态（在 proxy 恢复前完成）
let proxy_service = app.state::<proxy::ProxyService>();
proxy_service.set_log_sender(log_tx.clone());

// 启动后台写入 task（需要 AppHandle 用于 emit）
let app_handle_for_log = app.handle().clone();
tauri::async_runtime::spawn(async move {
    traffic::log_worker(log_rx, app_handle_for_log).await;
});
```

### Pattern 2: UpstreamTarget 扩展（provider_name）

**What:** `provider_name` 字段加入 `UpstreamTarget`，所有构造点一并填充。

**关键约束：** `request_logs.provider_name` 是 NOT NULL。handler 没有其他途径获取 provider 名称，只能从 `UpstreamTarget` 读取。

```rust
// proxy/state.rs
pub struct UpstreamTarget {
    pub api_key: String,
    pub base_url: String,
    pub protocol_type: ProtocolType,
    pub upstream_model: Option<String>,
    pub upstream_model_map: Option<HashMap<String, String>>,
    pub provider_name: String,   // 新增 Phase 27
}
```

**需同步修改的构造点：**
1. `commands/proxy.rs::build_upstream_target_from_provider` — 加 `provider_name: provider.name.clone()`
2. `commands/proxy.rs::build_upstream_target` — 加 `provider_name: String`（来自调用方传入）
3. `watcher/mod.rs::update_proxy_upstream_if_needed` — 加 `provider_name: provider.name.clone()`
4. 所有测试中手工构造的 `UpstreamTarget` — 加 `provider_name: "test".to_string()`

### Pattern 3: ProxyState 中注入 log_tx

**What:** `ProxyState` 持有 `Option<mpsc::Sender<LogEntry>>`，创建时为 None，setup 闭包注入后为 Some。

**注入方式：**

```rust
// proxy/state.rs
pub struct ProxyState {
    upstream: Arc<RwLock<Option<UpstreamTarget>>>,
    pub http_client: reqwest::Client,
    pub log_tx: Arc<RwLock<Option<tokio::sync::mpsc::Sender<LogEntry>>>>,
}

impl ProxyState {
    pub async fn set_log_sender(&self, tx: tokio::sync::mpsc::Sender<LogEntry>) {
        *self.log_tx.write().await = Some(tx);
    }
    pub async fn log_sender(&self) -> Option<tokio::sync::mpsc::Sender<LogEntry>> {
        self.log_tx.read().await.clone()
    }
}
```

**替代方案（更简单）：** 在 `ProxyService` 层持有 sender，通过启动 server 时注入到每个 `ProxyServer`/`ProxyState`。选哪种由 planner 决定（Claude's Discretion）。

### Pattern 4: handler 中采集元数据

**What:** `proxy_handler` 函数中添加 Instant 计时，请求结束时收集元数据，通过 `log_tx.try_send(entry)` 发送。

```rust
// proxy/handler.rs 中 proxy_handler 顶部
let start_time = std::time::Instant::now();
let request_start_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as i64;
```

**handler 结束处（响应体构建完毕后，return 前）：**

```rust
// 非流式分支内（取到 resp_bytes 后）：提取 token
let (input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, stop_reason) =
    extract_tokens_from_response(&resp_value, &response_mode);

// 发送日志（fire-and-forget）
if let Some(tx) = state.log_sender().await {
    let entry = LogEntry {
        created_at: request_start_ms,
        provider_name: upstream.provider_name.clone(),
        cli_id: /* 从 ProxyState 获取 */ ...,
        method: method.to_string(),
        path: path.clone(),
        status_code: Some(status.as_u16() as i64),
        is_streaming: 0,
        request_model: /* 从 body 提取 */ ...,
        upstream_model: upstream.upstream_model.clone(),
        protocol_type: upstream.protocol_type.to_string(),
        input_tokens: Some(input_tokens),
        output_tokens: Some(output_tokens),
        cache_creation_tokens,
        cache_read_tokens,
        ttfb_ms: None,
        duration_ms: Some(start_time.elapsed().as_millis() as i64),
        stop_reason,
        error_message: None,
    };
    let _ = tx.try_send(entry); // 满 buffer 时丢弃，不阻塞
}
```

### Pattern 5: log_worker 后台 task

**What:** 持续监听 mpsc receiver，收到 LogEntry 后写入 SQLite，写入成功后 emit `traffic-log` 事件。

```rust
// traffic/mod.rs 或 traffic/log.rs
pub async fn log_worker(
    mut rx: tokio::sync::mpsc::Receiver<LogEntry>,
    app_handle: tauri::AppHandle,
) {
    while let Some(entry) = rx.recv().await {
        // 从 Tauri 托管状态获取 TrafficDb
        if let Some(db) = app_handle.try_state::<TrafficDb>() {
            match db.insert_request_log(&entry) {
                Ok(id) => {
                    let payload = TrafficLogPayload::new_from_entry(id, &entry, "new");
                    if let Err(e) = app_handle.emit("traffic-log", &payload) {
                        log::warn!("emit traffic-log 失败: {}", e);
                    }
                }
                Err(e) => {
                    log::warn!("写入 request_logs 失败: {}", e);
                }
            }
        }
    }
}
```

### Pattern 6: Tauri emit（已建立，参照 watcher/mod.rs）

```rust
// 现有 emit 模式（watcher/mod.rs 第 199 行）
if let Err(e) = app_handle.emit("providers-changed", &payload) {
    log::error!("Failed to emit providers-changed event: {:?}", e);
}
```

**traffic-log 事件 payload 结构：**

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrafficLogPayload {
    // type 字段区分 "new" / "update"（Phase 28 用 update）
    #[serde(rename = "type")]
    pub event_type: String,
    // request_logs 全部 19 列
    pub id: i64,
    pub created_at: i64,
    pub provider_name: String,
    pub cli_id: String,
    pub method: String,
    pub path: String,
    pub status_code: Option<i64>,
    pub is_streaming: i64,
    pub request_model: Option<String>,
    pub upstream_model: Option<String>,
    pub protocol_type: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub ttfb_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub stop_reason: Option<String>,
    pub error_message: Option<String>,
}
```

### Anti-Patterns to Avoid

- **await 写入 DB 直接在 handler 中：** 会增加代理请求延迟。用 mpsc send 代替。
- **传递完整响应 body 到 mpsc：** body 可能很大，只传 token 数值（i64）。
- **在 log_worker 中 panic：** 写入失败应 `log::warn` 后继续，不能让整个日志 task 崩溃。
- **忘记处理 cli_id 注入：** handler 需要知道当前是哪个 cli（claude/codex）用于 `request_logs.cli_id`。ProxyState 中需要加 cli_id 字段或通过 provider_name 反查。
- **UpstreamTarget 构造测试遗漏 provider_name：** handler.rs 和 mod.rs 中共有 10+ 个测试构造 UpstreamTarget，全部需要加 `provider_name` 字段。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| epoch 毫秒时间戳 | 自己算毫秒 | `SystemTime::now().duration_since(UNIX_EPOCH).as_millis()` | 标准库直接支持 |
| 后台 spawn | 自建线程池 | `tauri::async_runtime::spawn` | 与 Tauri event loop 集成，watcher 已用此模式 |
| SQLite 写入并发 | 自建队列 | mpsc channel + 单 writer task | 已有 Mutex<Connection>，单写符合 rusqlite 最佳实践 |
| try_state 获取 TrafficDb | 直接 .state() | `app_handle.try_state::<TrafficDb>()` | TrafficDb 可能为 None（降级运行），try_state 返回 Option |

**Key insight:** log_worker 通过 `app_handle.try_state::<TrafficDb>()` 动态获取 DB，完全不需要在 channel 中传递 DB 引用。Tauri 的 `manage()` 系统是全局注册表。

---

## Common Pitfalls

### Pitfall 1: provider_name 字段缺失导致编译错误

**What goes wrong:** `request_logs.provider_name NOT NULL`，UpstreamTarget 无此字段，handler 无法填充。
**Why it happens:** UpstreamTarget 是 Phase 26 前设计的，未预留日志字段。
**How to avoid:** 第一个 task 必须扩展 UpstreamTarget + 修改所有构造点（含测试中约 10+ 处）。
**Warning signs:** 编译器报 "missing field `provider_name`" 错误。

### Pitfall 2: cli_id 在 handler 中不可见

**What goes wrong:** `request_logs.cli_id NOT NULL`，但 `proxy_handler` 的 `State<ProxyState>` 不携带 cli_id 信息。
**Why it happens:** ProxyServer 按 cli_id 管理，但 ProxyState 是每个 server 独立的——目前不存储 cli_id。
**How to avoid:** 在 `ProxyState` 中加 `cli_id: String` 字段，在 `ProxyServer::new` 时（或 `ProxyService::start` 时）传入。
**Warning signs:** 测试中 cli_id 为空字符串或 "unknown"。

### Pitfall 3: 非流式 body 被消费后无法重用

**What goes wrong:** handler 读取非流式 body 后调用 `.bytes().await`，但这个 body 已经在转换分支中被消费。
**Why it happens:** `upstream_resp.bytes()` 消费了流，只能调用一次。
**How to avoid:** 在已有的非流式分支（OpenAiChatCompletions/OpenAiResponses/AnthropicPassthrough）中，`resp_bytes` 变量已经是 `Bytes`。直接从 `resp_bytes` 解析 JSON Value 提取 token，不需要重新读取流。只需在这些分支构建 Body 返回之前（resp_bytes 已有值时）提取 token。

### Pitfall 4: 流式请求的 token 提取属 Phase 28

**What goes wrong:** 在流式分支（SSE）尝试提取 token，导致流被消费或阻塞。
**Why it happens:** 流式响应是 Body::from_stream，不能提前读取。
**How to avoid:** Phase 27 对流式请求只写基础元数据（token 字段留 null），不尝试提取 token。流式 body 完全按现有方式透传。

### Pitfall 5: Anthropic 协议直接透传时 token 字段缺失

**What goes wrong:** `ResponseTranslationMode::Passthrough` 分支（Anthropic 协议非 /v1/messages 请求）不解析 body，导致 token = null。
**Why it happens:** Passthrough 分支直接 `Body::from_stream(upstream_resp.bytes_stream())`，不读取完整 body。
**How to avoid:** Passthrough 分支的非流式场景同样需要先 `.bytes().await` 读取，判断是否为成功非 SSE 响应后尝试提取 token，token 提取失败则留 null。但注意：`Passthrough` 分支当前为 "所有非 /v1/messages 请求"（如 /v1/token_count），这些请求的响应格式各异，token 提取失败是正常的，直接留 null 可接受。

### Pitfall 6: try_send 满 buffer 时日志丢失

**What goes wrong:** 高并发时 channel buffer 满，`try_send` 返回 Err，日志静默丢失。
**Why it happens:** `< 10 req/s` 前提下 1024 buffer 不会满，但意外场景下可能丢日志。
**How to avoid:** `try_send` 返回 Err 时 `log::warn!` 记录 "日志 channel 已满，丢弃本条日志"。这是有意为之的 trade-off（不阻塞代理 > 日志完整性）。

---

## Code Examples

### Anthropic 协议非流式 token 提取

现有 response body（Anthropic 直接透传，非流式）字段：
```json
{
  "usage": {
    "input_tokens": 10,
    "output_tokens": 5,
    "cache_creation_input_tokens": 2,
    "cache_read_input_tokens": 8
  },
  "stop_reason": "end_turn"
}
```

提取代码：
```rust
fn extract_anthropic_tokens(resp_value: &serde_json::Value) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>) {
    let usage = resp_value.get("usage");
    let input_tokens = usage.and_then(|u| u.get("input_tokens")).and_then(|v| v.as_i64());
    let output_tokens = usage.and_then(|u| u.get("output_tokens")).and_then(|v| v.as_i64());
    let cache_creation = usage.and_then(|u| u.get("cache_creation_input_tokens")).and_then(|v| v.as_i64());
    let cache_read = usage.and_then(|u| u.get("cache_read_input_tokens")).and_then(|v| v.as_i64());
    let stop_reason = resp_value.get("stop_reason").and_then(|v| v.as_str()).map(|s| s.to_string());
    (input_tokens, output_tokens, cache_creation, cache_read, stop_reason)
}
```

### OpenAI Chat Completions 非流式 token 提取

OpenAI Chat Completions 原始响应（在 `translate::response::openai_to_anthropic` 之前提取）：
```json
{
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 5,
    "prompt_tokens_details": { "cached_tokens": 3 }
  },
  "choices": [{ "finish_reason": "stop" }]
}
```

**注意：** 对于 OpenAiChatCompletions 分支，非流式 token 提取有两个选择：
1. 从上游原始 body (`resp_bytes` / `resp_value` 在 `openai_to_anthropic` 转换之前) 提取原始 OpenAI 字段
2. 从 `openai_to_anthropic(resp_value)` 转换后的 Anthropic 格式中提取（已标准化为 input_tokens/output_tokens）

**推荐：** 从 `openai_to_anthropic` 转换后的结果提取。转换函数已正确处理 `prompt_tokens → input_tokens`、`completion_tokens → output_tokens`、`prompt_tokens_details.cached_tokens → cache_read_input_tokens`。stop_reason 已从 `finish_reason` 映射为 Anthropic 格式（end_turn/max_tokens/tool_use）。

**但 CONTEXT.md 要求 stop_reason 存原始值（不做映射）**，因此 stop_reason 应从原始 `resp_value`（转换前）提取 `choices[0].finish_reason`。

### OpenAI Responses API 非流式 token 提取

Responses API 原始响应：
```json
{
  "usage": {
    "input_tokens": 10,
    "output_tokens": 5
  },
  "output": [{ ... }]
}
```

**CONTEXT.md 说明：** 缓存字段位置由研究阶段确认。经查 `responses_response.rs` 代码，`responses_to_anthropic` 当前只透传 `input_tokens` 和 `output_tokens`，未处理缓存字段。

**实际 Responses API 缓存字段位置（基于 OpenAI 官方文档模式）：** `usage.input_token_details.cached_tokens`。但目前代码中无此字段的处理，Phase 27 对 Responses API 的 cache token 可留 null（Phase 28 可补充）。

### LogEntry 结构设计

```rust
// traffic/log.rs
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub created_at: i64,       // epoch ms
    pub provider_name: String,
    pub cli_id: String,
    pub method: String,
    pub path: String,
    pub status_code: Option<i64>,
    pub is_streaming: i64,     // 0 或 1
    pub request_model: Option<String>,
    pub upstream_model: Option<String>,
    pub protocol_type: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub ttfb_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub stop_reason: Option<String>,
    pub error_message: Option<String>,
}
```

### insert_request_log 方法

```rust
// traffic/log.rs 中 impl TrafficDb
pub fn insert_request_log(&self, entry: &LogEntry) -> rusqlite::Result<i64> {
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO request_logs
         (created_at, provider_name, cli_id, method, path, status_code, is_streaming,
          request_model, upstream_model, protocol_type,
          input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens,
          ttfb_ms, duration_ms, stop_reason, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        rusqlite::params![
            entry.created_at, entry.provider_name, entry.cli_id,
            entry.method, entry.path, entry.status_code, entry.is_streaming,
            entry.request_model, entry.upstream_model, entry.protocol_type,
            entry.input_tokens, entry.output_tokens, entry.cache_creation_tokens, entry.cache_read_tokens,
            entry.ttfb_ms, entry.duration_ms, entry.stop_reason, entry.error_message,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}
```

### get_recent_logs Tauri command

```rust
// commands/traffic.rs
#[tauri::command]
pub async fn get_recent_logs(
    traffic_db: tauri::State<'_, crate::traffic::TrafficDb>,
    limit: Option<i64>,
) -> Result<Vec<crate::traffic::log::TrafficLogPayload>, String> {
    let limit = limit.unwrap_or(100).min(1000);
    traffic_db.query_recent_logs(limit).map_err(|e| e.to_string())
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 直接在 handler await 写 DB | mpsc channel fire-and-forget | Phase 26 设计决策 | handler 延迟零增加 |
| Tauri 1.x 的 emit API | Tauri 2.x `Emitter` trait + `app_handle.emit()` | Phase 26 之前项目升级 | 已在 watcher 中使用，直接复用 |
| rusqlite blocking 连接 | Mutex<Connection> + 后台 task 中同步调用 | Phase 26 确立 | 单写 task 避免 mutex contention |

---

## Open Questions

1. **cli_id 如何传入 handler**
   - 当前状态：`ProxyState` 不携带 cli_id；`proxy_handler` 的 `State<ProxyState>` 无法读取 cli_id
   - 建议方案：在 `ProxyState` 中加 `cli_id: String`，在 `ProxyServer::new(port, client)` 时初始化为空字符串，在 `ProxyService::start(cli_id, port, upstream)` 中通过 `server.state().set_cli_id(cli_id)` 写入
   - 替代方案：通过 `proxy_port_for_cli` 反查（不优雅）

2. **ProxyError 到 error_message 的转换**
   - 当前状态：handler 返回 `Result<Response, ProxyError>`，错误通过 `IntoResponse` 转为 HTTP 502 返回
   - 建议方案：在 `ProxyError::into_response` 触发前，或在 handler 顶层 `match` 中捕获错误并发送日志
   - 注意：handler 函数签名是 `Result<Response, ProxyError>`，如果要记录错误日志，需要在返回 Err 之前或通过 middleware 拦截

3. **Passthrough 分支的 token 提取**
   - 当前状态：`Passthrough` 直接 `Body::from_stream(upstream_resp.bytes_stream())`，不读取完整 body
   - 建议方案：Passthrough 非流式场景不尝试提取 token（留 null），因为这些端点格式各异；仅对 is_streaming=0 的基础元数据写入即可

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (内置) |
| Config file | src-tauri/Cargo.toml（`[dev-dependencies]` 含 `tempfile`） |
| Quick run command | `cd src-tauri && cargo test traffic` |
| Full suite command | `cd src-tauri && cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STORE-03 | mpsc channel 发送后后台 task 写入 DB | unit | `cargo test traffic::log::tests` | ❌ Wave 0 |
| COLLECT-01 | LogEntry 包含所有基础元数据字段 | unit | `cargo test traffic::log::tests::test_log_entry_fields` | ❌ Wave 0 |
| COLLECT-02 | 非流式 Anthropic/OpenAI/Responses 响应 token 提取正确 | unit | `cargo test traffic::log::tests::test_token_extraction` | ❌ Wave 0 |
| COLLECT-04 | 失败请求 error_message 非 null，成功请求 stop_reason 非 null | unit | `cargo test traffic::log::tests::test_error_log` | ❌ Wave 0 |
| LOG-01 | 写入 DB 后 emit traffic-log 事件（integration） | integration | `cargo test proxy::handler::tests::test_log_pipeline_e2e` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cd /Users/kelin/Work/CLIManager/src-tauri && cargo test traffic 2>&1 | tail -5`
- **Per wave merge:** `cd /Users/kelin/Work/CLIManager/src-tauri && cargo test 2>&1 | tail -20`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src-tauri/src/traffic/log.rs` — 新文件，含 `LogEntry`、`TrafficLogPayload`、`insert_request_log`、`log_worker`、单元测试（covers STORE-03, COLLECT-01, COLLECT-02, COLLECT-04）
- [ ] `src-tauri/src/commands/traffic.rs` — 新文件，含 `get_recent_logs` command
- [ ] 现有测试中 `UpstreamTarget` 构造需同步加 `provider_name` 字段（约 15 处，属修复编译错误范畴，不是新增测试文件）

---

## Sources

### Primary (HIGH confidence)

- 直接读取项目源码：
  - `src-tauri/src/traffic/mod.rs` — TrafficDb 结构，init_traffic_db 函数
  - `src-tauri/src/traffic/schema.rs` — request_logs 表 19 列完整定义
  - `src-tauri/src/traffic/db.rs` — WAL 配置，open_traffic_db 模式
  - `src-tauri/src/proxy/state.rs` — ProxyState 当前字段（确认无 provider_name/cli_id/log_tx）
  - `src-tauri/src/proxy/handler.rs` — 各协议分支中 resp_bytes/resp_value 的实际可用位置
  - `src-tauri/src/proxy/translate/response.rs` — OpenAI Chat Completions token 字段映射
  - `src-tauri/src/proxy/translate/responses_response.rs` — Responses API token 字段（仅 input_tokens/output_tokens）
  - `src-tauri/src/watcher/mod.rs` — app_handle.emit 使用范例，tauri::async_runtime::spawn 模式
  - `src-tauri/src/lib.rs` — setup 闭包结构，TrafficDb manage 注入点
  - `src-tauri/src/commands/proxy.rs` — UpstreamTarget 构造点（build_upstream_target_from_provider）
  - `src-tauri/Cargo.toml` — 已有依赖（tokio, rusqlite, serde, tauri）

### Secondary (MEDIUM confidence)

- tokio mpsc channel buffer 大小建议（1024）基于项目吞吐量 `< 10 req/s` 的工程判断，符合 Rust 社区惯例

### Tertiary (LOW confidence)

- OpenAI Responses API 缓存 token 字段位置（`usage.input_token_details.cached_tokens`）：基于知识截止日期内的 OpenAI 文档记忆，未在本次研究中通过官方文档直接验证。Phase 27 决定对 Responses API 缓存 token 留 null，此不确定性不影响实现。

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — 完全基于项目已有 Cargo.toml 和源码，无新依赖
- Architecture: HIGH — 基于直接代码阅读，ProxyState/handler/watcher 各点均已确认
- Token 字段位置: HIGH（Anthropic/OpenAI Chat） / MEDIUM（Responses API 缓存）

**Research date:** 2026-03-18
**Valid until:** 2026-04-17（项目依赖稳定，30 天有效）
