# Phase 8: 代理核心 - Research

**Researched:** 2026-03-13
**Domain:** Rust HTTP 反向代理 (axum 0.8 + reqwest + Tauri 2 tokio runtime)
**Confidence:** HIGH

## Summary

Phase 8 的目标是实现代理核心基础设施：每个 CLI 拥有独立端口的本地 HTTP 代理服务器，能将请求转发到上游 Provider 并支持 SSE 流式响应。本阶段只构建代理服务器本身（start/stop/status API、请求转发、SSE 透传、凭据替换、健康检查），不包含模式切换 UI、CLI 配置 patch、崩溃恢复（Phase 9）或实时切换联动（Phase 10）。

技术栈选型极为有利：Tauri 2 内置 tokio 1.50、hyper 1.8.1、tower 0.5.3，新增 axum 0.8 几乎不引入新传递依赖。代理核心逻辑（请求转发 + SSE 流式透传 + 动态上游切换）在 cc-switch 中有成熟参考实现，且 Phase 8 范围远比 cc-switch 精简——不做协议转换、熔断器、Usage 统计，预计核心代码量 300-500 行。关键风险点在于 SSE 流式转发的正确实现（不能缓冲）和 Tauri 生命周期中的优雅停机。

**核心建议:** 使用 axum 0.8 的 `fallback` handler 实现全路径透传代理，reqwest `bytes_stream()` + axum `Body::from_stream()` 实现零缓冲 SSE 流式转发，`tokio::sync::RwLock` 存储动态上游路由表，`oneshot` channel 实现优雅停机。

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- 全路径透传：代理不理解路径语义，任何 HTTP 请求原样转发到上游 base_url
- 端口即身份：15800=Claude Code, 15801=Codex，代理通过监听端口判断请求属于哪个 CLI
- 上游 URL 拼接：Provider.base_url + 原始请求路径
- 请求头最小化替换：保留原始请求头，只替换 Authorization/x-api-key 为真实凭据
- 占位值为统一字符串 "PROXY_MANAGED"（与 cc-switch 一致）
- 匹配占位才替换：检查请求中的 auth 头/key 是否为 "PROXY_MANAGED"，是则替换；非占位值正常转发不修改
- 纯透传代理：接收请求 -> 替换 key -> 转发上游 -> 透传响应（包括 SSE 流式）
- 不做请求体解析/修改
- 架构预留中间件插槽
- 复用 reqwest 作为上游 HTTP 客户端（需启用 stream feature）
- 统一代理错误格式：代理自身错误返回 502 + JSON（`{"error": {"type": "proxy_error", "message": "..."}}`）
- 上游错误原样透传：非 2xx 响应保留原始状态码和响应体
- GET /health 只返回 `{"status": "ok"}`
- 不设代理端超时，完全依赖 CLI 自身的请求超时设置
- Phase 8 提供 start()/stop()/status() API，由上层控制启停时机
- 代理服务作为 Tauri 托管状态（app.manage(ProxyService)），通过 Tauri 命令暴露
- 停止代理只停 axum 服务器监听，不触及 CLI 配置
- 启动时注入当前活跃 Provider 信息，运行时通过 update_upstream() 方法动态切换上游目标

### Claude's Discretion
- SSE 流式透传的具体实现方式（逐 chunk vs buffered）
- axum Router 和 Handler 的代码组织方式
- 多端口架构：每端口一个独立 axum Server 实例 vs 共享状态的多 listener
- reqwest Client 配置（连接池、keep-alive 等）
- 代理内部错误日志级别和格式

### Deferred Ideas (OUT OF SCOPE)
- Thinking Rectifier / Body Filter / Model Mapping
- 代理流量统计与可视化（TRAF-01, TRAF-02）
- 自动 Failover / 熔断器（ADV-01, ADV-02）
- 自定义代理端口（ADV-05）
- 协议转换 Anthropic<->OpenAI（PROTO-01, PROTO-02）
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PROXY-01 | 用户开启代理后，CLI 的 API 请求经本地 HTTP 代理转发到上游 Provider | axum 0.8 fallback handler + reqwest 转发；全路径透传模式 |
| PROXY-02 | 代理支持 SSE 流式响应逐 chunk 透传 | `reqwest::Response::bytes_stream()` + `axum::body::Body::from_stream()` 零缓冲管道 |
| PROXY-03 | 代理拦截请求中的占位 API key，替换为当前活跃 Provider 的真实 key 后转发上游 | 检查 `x-api-key` / `Authorization` header 是否为 "PROXY_MANAGED"，是则替换 |
| PROXY-04 | 每个 CLI 监听独立固定端口（Claude Code: 15800, Codex: 15801） | 每端口一个独立 axum Server 实例 + 共享 ProxyState |
| PROXY-05 | 上游不可达时代理返回 502 + JSON 结构化错误 | ProxyError 枚举 + IntoResponse 实现，reqwest 连接错误映射为 502 |
| UX-03 | 代理启动后执行健康自检（GET /health），确认监听正常 | GET /health 端点 + 启动后自动发 reqwest GET 验证 |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.8 | HTTP 路由和请求处理 | tokio 官方生态 HTTP 框架，基于 hyper 1.x + tower 0.5，与 Tauri 2 的 tokio runtime 完全兼容 |
| reqwest | 0.12 (已有) | 上游 HTTP 请求转发 | 已在 Cargo.toml 中，启用 `stream` feature 后可用 `bytes_stream()` 逐 chunk 读取 |
| tokio | 1 (已有) | async runtime、net、sync、time | Tauri 2 内置，显式声明确保 net/sync/time feature 可见 |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tower-http | 0.6 (已在 Cargo.lock) | CORS 中间件 | 预留 CorsLayer，CLI 直连不需 CORS 但成本为零 |
| serde/serde_json | 1 (已有) | JSON 序列化 | 错误响应体构造、状态序列化 |
| thiserror | 2.0 (已有) | 错误类型 derive | ProxyError 枚举定义 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| axum 0.8 | actix-web 4 | actix 有独立 runtime，与 Tauri tokio 不兼容，需开独立线程 |
| 自定义 handler | axum-reverse-proxy crate | 引入不需要的负载均衡/DNS 发现功能；我们需要动态认证注入，自己写更可控 |
| reqwest bytes_stream() | SSE 事件解析库 (eventsource-stream) | v2.0 不需理解 SSE 事件内容，透传更简单高效 |
| tokio::sync::RwLock | ArcSwap | RwLock 对本地单用户场景完全足够，ArcSwap 是可选优化 |

**Installation:**
```toml
# src-tauri/Cargo.toml 变更

# 修改（增加 stream feature）:
reqwest = { version = "0.12", features = ["json", "stream"] }

# 新增:
axum = "0.8"
tower-http = { version = "0.6", features = ["cors"] }
tokio = { version = "1", features = ["net", "sync", "time"] }
```

**新增 crate 数量:** 1 个真正新增（axum），2 个显式声明已有传递依赖（tower-http, tokio）。实际新增传递依赖极少——axum 依赖的 hyper/tower/tokio/bytes/http 全部已在 Cargo.lock 中。置信度 HIGH（直接检查 Cargo.lock 验证）。

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
  proxy/                     # 新增顶层模块
    mod.rs                   # ProxyService 定义 + 模块公开导出
    server.rs                # 单个 axum 代理服务器的创建/启停（ProxyServer）
    handler.rs               # HTTP 请求转发处理器（全路径透传 + 凭据替换）
    error.rs                 # ProxyError 枚举 + IntoResponse 实现
    state.rs                 # UpstreamTarget + 共享路由表
  commands/
    proxy.rs                 # 新增：代理相关 Tauri 命令（start/stop/status）
    mod.rs                   # 修改：注册 proxy 命令模块
  lib.rs                     # 修改：注册 ProxyService 为 Tauri State + 注册命令
  error.rs                   # 修改：新增 Proxy 错误变体
```

### Pattern 1: 全路径透传代理（Fallback Handler）
**What:** 使用 axum 的 `fallback` handler 捕获所有未匹配路由，将请求原样转发到上游 Provider.base_url + 原始路径。
**When to use:** Phase 8 的核心转发逻辑。
**Why fallback not catch-all route:** axum 0.8 的 wildcard route `/{*path}` 不匹配根路径 `/`，而 `fallback` 捕获所有未匹配请求（含根路径），更适合全路径透传场景。

**Example:**
```rust
// proxy/server.rs
use axum::{routing::get, Router};

fn build_router(state: ProxyState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .fallback(proxy_handler)  // 所有未匹配路由走代理转发
        .with_state(state)
}

// proxy/handler.rs
async fn proxy_handler(
    State(state): State<ProxyState>,
    req: axum::extract::Request,  // axum 0.8: 直接用 Request
) -> Result<axum::response::Response, ProxyError> {
    // 1. 从 state 读取当前上游目标
    let upstream = state.get_upstream().await
        .ok_or(ProxyError::NoUpstreamConfigured)?;

    // 2. 提取原始请求信息
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let headers = req.headers().clone();
    let body_bytes = axum::body::to_bytes(req.into_body(), usize::MAX).await?;

    // 3. 构建上游 URL
    let upstream_url = format!("{}{}{}", upstream.base_url, path, query);

    // 4. 构建 reqwest 请求（替换凭据）
    let mut req_builder = state.http_client
        .request(method, &upstream_url);

    // 5. 透传 headers，替换认证头
    for (key, value) in headers.iter() {
        let key_str = key.as_str().to_lowercase();
        // 跳过 hop-by-hop headers 和 host
        if matches!(key_str.as_str(), "host" | "content-length" | "transfer-encoding") {
            continue;
        }
        // 检查是否需要替换认证头
        if key_str == "x-api-key" || key_str == "authorization" {
            let val_str = value.to_str().unwrap_or("");
            if val_str == "PROXY_MANAGED" || val_str == "Bearer PROXY_MANAGED" {
                // 替换为真实凭据（在下方注入）
                continue;
            }
        }
        req_builder = req_builder.header(key, value);
    }

    // 6. 注入真实认证头
    match upstream.protocol_type {
        ProtocolType::Anthropic => {
            req_builder = req_builder.header("x-api-key", &upstream.api_key);
        }
        ProtocolType::OpenAiCompatible => {
            req_builder = req_builder
                .header("Authorization", format!("Bearer {}", upstream.api_key));
        }
    }

    // 7. 发送请求
    let upstream_resp = req_builder
        .body(body_bytes.to_vec())
        .send()
        .await
        .map_err(|e| ProxyError::UpstreamUnreachable(e.to_string()))?;

    // 8. 构建响应（流式透传）
    let status = upstream_resp.status();
    let resp_headers = upstream_resp.headers().clone();

    let mut builder = axum::response::Response::builder()
        .status(status.as_u16());

    for (key, value) in resp_headers.iter() {
        let k = key.as_str().to_lowercase();
        if !matches!(k.as_str(), "transfer-encoding" | "content-length") {
            builder = builder.header(key, value);
        }
    }

    // 9. SSE 流式透传：bytes_stream() -> Body::from_stream()
    let body = axum::body::Body::from_stream(upstream_resp.bytes_stream());
    builder.body(body).map_err(|e| ProxyError::Internal(e.to_string()))
}
```

### Pattern 2: 共享状态 + 动态上游切换
**What:** 使用 `tokio::sync::RwLock` 持有每个 CLI 的当前上游目标，切换 Provider 时更新路由表，在途请求不受影响。
**When to use:** ProxyState 的核心状态管理。

**Example:**
```rust
// proxy/state.rs
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct UpstreamTarget {
    pub api_key: String,
    pub base_url: String,
    pub protocol_type: ProtocolType,
}

#[derive(Clone)]
pub struct ProxyState {
    upstream: Arc<RwLock<Option<UpstreamTarget>>>,
    pub http_client: reqwest::Client,
}

impl ProxyState {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            upstream: Arc::new(RwLock::new(None)),
            http_client: client,
        }
    }

    pub async fn get_upstream(&self) -> Option<UpstreamTarget> {
        self.upstream.read().await.clone()
    }

    pub async fn update_upstream(&self, target: UpstreamTarget) {
        *self.upstream.write().await = Some(target);
    }

    pub async fn clear_upstream(&self) {
        *self.upstream.write().await = None;
    }
}
```

### Pattern 3: 优雅停机 via oneshot channel
**What:** 每个 ProxyServer 启动时保留 `oneshot::Sender<()>`，停止时发送信号触发 `axum::serve().with_graceful_shutdown()`。
**When to use:** ProxyServer start/stop 生命周期。

**Example:**
```rust
// proxy/server.rs
pub struct ProxyServer {
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    state: ProxyState,
    port: u16,
}

impl ProxyServer {
    pub async fn start(&mut self) -> Result<(), ProxyError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| ProxyError::BindFailed(format!("{}: {}", addr, e)))?;

        let app = build_router(self.state.clone());

        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async { rx.await.ok(); })
                .await
                .ok();
        });

        self.shutdown_tx = Some(tx);
        self.server_handle = Some(handle);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), ProxyError> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.server_handle.take() {
            match tokio::time::timeout(
                std::time::Duration::from_secs(5), handle
            ).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(ProxyError::StopFailed(e.to_string())),
                Err(_) => Err(ProxyError::StopTimeout),
            }
        } else {
            Ok(())
        }
    }
}
```

### Pattern 4: ProxyService 管理多端口服务器
**What:** ProxyService 作为 Tauri 托管状态，管理 Claude Code + Codex 两个独立 ProxyServer 实例，对外暴露 start/stop/status API。
**When to use:** 应用层集成点。

**Example:**
```rust
// proxy/mod.rs
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct ProxyService {
    servers: Mutex<HashMap<String, ProxyServer>>,  // cli_id -> server
    http_client: reqwest::Client,  // 共享 HTTP 客户端
}

impl ProxyService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .expect("创建 HTTP 客户端失败");
        Self {
            servers: Mutex::new(HashMap::new()),
            http_client: client,
        }
    }

    /// 启动指定 CLI 的代理
    pub async fn start(
        &self, cli_id: &str, port: u16, upstream: UpstreamTarget
    ) -> Result<(), ProxyError> { ... }

    /// 停止指定 CLI 的代理
    pub async fn stop(&self, cli_id: &str) -> Result<(), ProxyError> { ... }

    /// 查询所有代理状态
    pub async fn status(&self) -> ProxyStatusInfo { ... }

    /// 动态更新某 CLI 的上游目标（Provider 切换时调用）
    pub async fn update_upstream(
        &self, cli_id: &str, upstream: UpstreamTarget
    ) -> Result<(), ProxyError> { ... }
}
```

### Pattern 5: Tauri 命令层集成
**What:** 代理命令遵循现有 commands/ 分层模式，使用 `State<ProxyService>` 访问代理服务。
**When to use:** 前端/上层调用代理功能。

**Example:**
```rust
// commands/proxy.rs
use tauri::State;

#[tauri::command]
pub async fn proxy_start(
    cli_id: String,
    port: u16,
    api_key: String,
    base_url: String,
    protocol_type: String,
    proxy_service: State<'_, ProxyService>,
) -> Result<(), String> {
    let target = UpstreamTarget { api_key, base_url, protocol_type: ... };
    proxy_service.start(&cli_id, port, target).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn proxy_stop(
    cli_id: String,
    proxy_service: State<'_, ProxyService>,
) -> Result<(), String> {
    proxy_service.stop(&cli_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn proxy_status(
    proxy_service: State<'_, ProxyService>,
) -> Result<ProxyStatusInfo, String> {
    Ok(proxy_service.status().await)
}

#[tauri::command]
pub async fn proxy_update_upstream(
    cli_id: String,
    api_key: String,
    base_url: String,
    protocol_type: String,
    proxy_service: State<'_, ProxyService>,
) -> Result<(), String> {
    let target = UpstreamTarget { api_key, base_url, protocol_type: ... };
    proxy_service.update_upstream(&cli_id, target).await
        .map_err(|e| e.to_string())
}
```

### Anti-Patterns to Avoid
- **请求体解析/修改:** Phase 8 不解析请求 body，不做 model mapping、thinking rectifier 等——这些是未来中间件插槽的事
- **每请求创建 reqwest Client:** 必须预创建并复用，否则无法复用 TCP 连接池
- **缓冲完整响应再发送:** SSE 流式代理必须逐 chunk 透传，否则 CLI 的"打字机效果"消失
- **绑定 0.0.0.0:** 必须绑定 127.0.0.1，避免 macOS 防火墙弹窗和 API key 泄露
- **使用 `tokio::spawn` 而非 `tauri::async_runtime::spawn`:** tokio::spawn 在 Tauri handler 中可能静默失败（但在 server.rs 内部的 spawn 可以用 tokio::spawn，因为已在 Tauri runtime 内）

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP 路由 + 提取器 | 自己解析 HTTP 请求 | axum Router + extractors | axum 在 hyper 之上提供路由、提取器、中间件组合，代码量减少 60%+ |
| HTTP 客户端 | 直接用 hyper Client | reqwest 0.12 (已有) | reqwest 已在项目中，API 更友好，内置连接池、TLS、重定向 |
| 优雅停机 | 自己管理 signal + abort | axum::serve().with_graceful_shutdown() | 官方支持的停机模式，正确处理进行中的连接 |
| 流式响应 Body | 手动实现 Stream trait | Body::from_stream(bytes_stream()) | axum + reqwest 原生支持，一行代码完成流式管道 |
| 错误序列化 | 手写 match + JSON 构造 | thiserror + IntoResponse impl | axum 的 IntoResponse trait 自动处理错误到 HTTP 响应的转换 |

**关键洞察:** cc-switch 的 proxy 模块约 2000+ 行代码，因为包含了协议转换、熔断器、Usage 统计等。CLIManager Phase 8 只做纯透传代理，核心代码预计 300-500 行。

## Common Pitfalls

### Pitfall 1: SSE 流式响应缓冲（而非逐 chunk 透传）
**What goes wrong:** CLI 收到的 SSE 事件延迟堆积，"打字机效果"消失，变成长时间等待后一次性输出全部内容。
**Why it happens:** (1) 中间件压缩层（CompressionLayer）缓冲数据；(2) 未使用 `bytes_stream()` 而是 `text()` 或 `bytes()` 一次性读取；(3) 响应中无意间添加 Content-Length header。
**How to avoid:** 使用 `reqwest::Response::bytes_stream()` + `axum::body::Body::from_stream()` 零缓冲管道；不加 CompressionLayer；不添加 Content-Length header；透传上游的 `Content-Type: text/event-stream` 和 `Transfer-Encoding: chunked`。
**Warning signs:** 代理模式下 CLI 输出是"一坨一坨"出现而非逐字出现；直连模式正常。

### Pitfall 2: macOS 防火墙弹窗
**What goes wrong:** 用户启动代理后 macOS 弹出"允许传入连接"提示框。开发期间每次重编译都触发。
**Why it happens:** 程序绑定到 `0.0.0.0` 时 macOS 防火墙拦截。
**How to avoid:** 强制绑定 `127.0.0.1`，在代码中硬编码。
**Warning signs:** 开发期间每次 `cargo tauri dev` 弹防火墙提示。

### Pitfall 3: 端口泄漏 / 无法优雅停机
**What goes wrong:** 应用退出后端口仍被占用，下次启动 bind 失败报 "Address already in use"。
**Why it happens:** Tauri 在 macOS 上的退出事件不可靠——`app.exit()` 调用 `std::process::exit()` 不触发 drop。
**How to avoid:** 使用 oneshot channel + graceful shutdown；启动时先检测端口可用性；提供清晰的错误信息。Phase 8 提供 stop() API，Phase 9 负责在退出时调用它。
**Warning signs:** 第二次启动时报 "Address already in use"。

### Pitfall 4: 上游请求认证头注入不正确
**What goes wrong:** Anthropic API 返回 401（x-api-key 未设置）或 OpenAI API 返回 401（Authorization header 格式错误）。
**Why it happens:** Anthropic 使用 `x-api-key` header，OpenAI 使用 `Authorization: Bearer {key}`。注入时需根据 `protocol_type` 区分。
**How to avoid:** `UpstreamTarget` 包含 `protocol_type` 字段，handler 根据 protocol_type 注入正确的认证 header。
**Warning signs:** 代理模式下所有请求返回 401。

### Pitfall 5: hop-by-hop headers 透传错误
**What goes wrong:** 代理透传了 `Transfer-Encoding`, `Connection` 等 hop-by-hop headers，导致下游 HTTP 解析异常。
**Why it happens:** HTTP/1.1 规范要求代理不应透传 hop-by-hop headers。
**How to avoid:** 转发响应头时过滤 `transfer-encoding`, `content-length`, `connection` 等 hop-by-hop headers。axum 和 reqwest 会各自管理这些 headers。
**Warning signs:** CLI 收到 malformed response 错误。

### Pitfall 6: reqwest 连接错误未正确映射
**What goes wrong:** 上游不可达时 CLI 收到 500 而非 502，或者错误体非 JSON 格式。
**Why it happens:** reqwest 连接失败抛出的 error 未被正确捕获和映射。
**How to avoid:** ProxyError 实现 IntoResponse trait，`UpstreamUnreachable` 变体映射为 502 + JSON body `{"error": {"type": "proxy_error", "message": "..."}}`.
**Warning signs:** 上游关闭时 CLI 收到非结构化错误。

## Code Examples

### 健康检查端点
```rust
// proxy/handler.rs
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

pub async fn health_handler() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}
```

### 启动后健康自检（UX-03）
```rust
// proxy/server.rs — start() 方法末尾
async fn health_check(port: u16) -> Result<(), ProxyError> {
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = reqwest::Client::new();
    let resp = client.get(&url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map_err(|e| ProxyError::HealthCheckFailed(format!("无法连接: {}", e)))?;

    if resp.status() != 200 {
        return Err(ProxyError::HealthCheckFailed(
            format!("状态码: {}", resp.status())
        ));
    }
    Ok(())
}
```

### ProxyError 枚举 + IntoResponse
```rust
// proxy/error.rs
use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("服务器已在运行")]
    AlreadyRunning,

    #[error("服务器未运行")]
    NotRunning,

    #[error("地址绑定失败: {0}")]
    BindFailed(String),

    #[error("停止超时")]
    StopTimeout,

    #[error("停止失败: {0}")]
    StopFailed(String),

    #[error("上游不可达: {0}")]
    UpstreamUnreachable(String),

    #[error("未配置上游目标")]
    NoUpstreamConfigured,

    #[error("健康检查失败: {0}")]
    HealthCheckFailed(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ProxyError::UpstreamUnreachable(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            ProxyError::NoUpstreamConfigured => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = json!({
            "error": {
                "type": "proxy_error",
                "message": message,
            }
        });

        (status, Json(body)).into_response()
    }
}
```

### lib.rs 集成点
```rust
// lib.rs — 新增注册
mod proxy;

// setup() 中：
let proxy_service = proxy::ProxyService::new();
app.manage(proxy_service);

// invoke_handler 中新增：
commands::proxy::proxy_start,
commands::proxy::proxy_stop,
commands::proxy::proxy_status,
commands::proxy::proxy_update_upstream,
```

## Discretion Recommendations

以下是 Claude's Discretion 区域的研究建议：

### SSE 流式透传方式
**建议: 逐 chunk 透传（而非 buffered）。** 使用 `reqwest::Response::bytes_stream()` + `axum::body::Body::from_stream()` 构建零缓冲管道。这是 cc-switch 验证过的模式，且是 AI API 代理的标准做法。不需要任何 SSE 事件解析。

### axum Router 和 Handler 代码组织
**建议: 4 文件模块结构。** `mod.rs`（ProxyService 定义 + 公开导出）、`server.rs`（单个 axum 服务器启停）、`handler.rs`（请求转发逻辑 + 健康检查）、`error.rs`（ProxyError 枚举）、`state.rs`（UpstreamTarget + ProxyState）。保持与现有 commands/adapter/storage 的模块粒度一致。

### 多端口架构
**建议: 每端口一个独立 axum Server 实例 + 共享 reqwest::Client。** 每个 CLI 独立的 ProxyServer 实例，各自拥有 `ProxyState`（含独立的 UpstreamTarget），但共享同一个 `reqwest::Client`（连接池复用）。这比共享 Router + 多 listener 更简单，且允许每个 CLI 独立启停。

### reqwest Client 配置
**建议: 预创建共享 Client，开启 TCP keepalive。** 配置 `tcp_keepalive(30s)` 防止长 SSE 连接被中间网络设备切断。不设全局 timeout（用户决策：不设代理端超时），但保留 `connect_timeout(10s)` 防止 DNS 解析和连接建立挂住。

```rust
let client = reqwest::Client::builder()
    .tcp_keepalive(std::time::Duration::from_secs(30))
    .connect_timeout(std::time::Duration::from_secs(10))
    // 不设 timeout()：依赖 CLI 自身的请求超时
    .build()
    .expect("创建 HTTP 客户端失败");
```

### 代理内部日志
**建议: 使用 `log` crate（已有），关键路径 info 级别，错误 error 级别。** 启动/停止/bind 失败用 `log::info!` / `log::error!`。每个请求不记录日志（v2.0 不做流量监控）。连接错误用 `log::warn!`。

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| axum 0.7 路径语法 `/:param` | axum 0.8 `/{param}` | 2025-01 (axum 0.8.0) | cc-switch 参考代码需调整路径语法 |
| axum 0.7 wildcard `/*path` | axum 0.8 `/{*path}` | 2025-01 | catch-all 路由语法变更 |
| StreamBody 包装流式响应 | Body::from_stream() | axum 0.7+ | 直接用 Body::from_stream 更简洁 |
| hyper 0.14 Client | hyper 1.x + reqwest 0.12 | 2024 | reqwest 基于 hyper 1.x，API 更友好 |

**Deprecated/outdated:**
- `axum::body::StreamBody`: 已被 `Body::from_stream()` 替代
- `hyper::Client`: hyper 1.x 移除了 Client，推荐用 `hyper-util` 或 `reqwest`
- axum 0.7 `/:param` 和 `/*path` 路径语法: 在 0.8 中会 panic

## Open Questions

1. **connect_timeout 是否符合"不设代理端超时"决策?**
   - What we know: 用户决策明确"不设代理端超时，完全依赖 CLI 自身的请求超时设置"
   - What's unclear: `connect_timeout` 是建立连接的超时（非请求超时），与 CLI 的请求超时是不同层级
   - Recommendation: 保留 `connect_timeout(10s)` 作为防御性措施，它不影响 SSE 流式传输时间。如果用户反馈，可以去掉。

2. **ProxyService 中 servers HashMap 用 `tokio::sync::Mutex` 还是 `std::sync::Mutex`?**
   - What we know: servers map 只在 start/stop 时写入，频率极低
   - What's unclear: start/stop 是 async 操作（需要 await bind），但 HashMap 本身的读写是同步的
   - Recommendation: 使用 `tokio::sync::Mutex`，因为 start/stop 方法内部是 async（涉及 TcpListener::bind().await）。

3. **reqwest 请求体如何从 axum Request 高效传递到 reqwest?**
   - What we know: axum 0.8 的 `Request` body 是 `axum::body::Body`，需要转换为 reqwest 能接受的格式
   - What's unclear: 是先 `to_bytes()` 再传，还是可以流式传递请求体
   - Recommendation: 先用 `axum::body::to_bytes(body, max_size).await` 一次性读取请求体，因为 AI API 请求体通常 < 1MB（对话历史 + prompt），不值得流式传递的复杂度。设 `max_size` 为 200MB（与 cc-switch 一致）作为安全边界。

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust 内置 `#[cfg(test)]` + `cargo test` |
| Config file | 无独立配置——Cargo.toml `[dev-dependencies]` |
| Quick run command | `cargo test --lib -p cli-manager -- proxy` |
| Full suite command | `cargo test --lib -p cli-manager` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PROXY-01 | 请求经代理转发到上游 | integration | `cargo test --lib -p cli-manager -- proxy::tests::test_proxy_forward` | Wave 0 |
| PROXY-02 | SSE 流式响应逐 chunk 透传 | integration | `cargo test --lib -p cli-manager -- proxy::tests::test_sse_streaming` | Wave 0 |
| PROXY-03 | 占位 key 替换为真实 key | unit | `cargo test --lib -p cli-manager -- proxy::handler::tests::test_credential_replacement` | Wave 0 |
| PROXY-04 | 双端口独立监听 | integration | `cargo test --lib -p cli-manager -- proxy::tests::test_dual_port` | Wave 0 |
| PROXY-05 | 上游不可达返回 502 + JSON | unit | `cargo test --lib -p cli-manager -- proxy::error::tests::test_error_response_format` | Wave 0 |
| UX-03 | 健康自检 | integration | `cargo test --lib -p cli-manager -- proxy::tests::test_health_check` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib -p cli-manager -- proxy`
- **Per wave merge:** `cargo test --lib -p cli-manager`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src-tauri/src/proxy/mod.rs` — proxy 模块入口 + ProxyService
- [ ] `src-tauri/src/proxy/server.rs` — ProxyServer 启停逻辑 + tests
- [ ] `src-tauri/src/proxy/handler.rs` — 请求转发 + 凭据替换 + tests
- [ ] `src-tauri/src/proxy/error.rs` — ProxyError + IntoResponse + tests
- [ ] `src-tauri/src/proxy/state.rs` — UpstreamTarget + ProxyState + tests
- [ ] `Cargo.toml` — 新增 axum 0.8, reqwest stream feature, tokio features, tower-http

**注意:** 集成测试（PROXY-01, PROXY-02, PROXY-04, UX-03）需要启动真实的 axum 服务器绑定端口，测试中应使用动态端口（`TcpListener::bind("127.0.0.1:0")`）避免端口冲突。单元测试（PROXY-03, PROXY-05）不需要网络。

## Sources

### Primary (HIGH confidence)
- [axum 0.8.0 公告](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) — 版本发布说明、路径语法变更
- [axum Router 文档](https://docs.rs/axum/latest/axum/routing/struct.Router.html) — fallback handler、wildcard 路由
- [axum 0.8 wildcard 讨论](https://github.com/tokio-rs/axum/discussions/3204) — `/{*path}` 新语法
- [axum fallback 文档](https://github.com/tokio-rs/axum/blob/main/axum/src/docs/routing/fallback.md) — fallback vs catch-all 区别
- [axum + reqwest 代理讨论](https://github.com/tokio-rs/axum/discussions/1821) — bytes_stream() + Body::from_stream() 模式
- cc-switch `src-tauri/src/proxy/server.rs` — axum + Tauri 代理服务器实现 (工作参考)
- cc-switch `src-tauri/src/proxy/handlers.rs` — 健康检查、请求处理器模式 (工作参考)
- cc-switch `src-tauri/src/proxy/error.rs` — ProxyError + IntoResponse 实现 (工作参考)
- CLIManager `Cargo.lock` — 直接验证 tokio 1.50、hyper 1.8.1、tower 0.5.3 已存在
- CLIManager `src-tauri/src/lib.rs` — 现有 Tauri setup 和 State 管理模式
- CLIManager `src-tauri/src/provider.rs` — Provider struct 和 ProtocolType 定义
- `.planning/research/STACK.md` — v2.0 域级技术栈调研 (已验证)
- `.planning/research/ARCHITECTURE.md` — 代理集成架构设计 (已验证)
- `.planning/research/PITFALLS.md` — 7 个关键陷阱和预防策略 (已验证)

### Secondary (MEDIUM confidence)
- [Static streams for faster async proxies](https://blog.adamchalmers.com/streaming-proxy/) — 流式代理 vs 缓冲代理性能分析
- [axum + reqwest body 转换讨论](https://github.com/tokio-rs/axum/discussions/2603) — Request body -> reqwest Body 转换
- [Tauri + Async Rust Process](https://rfdonnelly.github.io/posts/tauri-async-rust-process/) — Tauri 内运行异步任务模式

### Tertiary (LOW confidence)
- 无——本阶段所有关键技术点均有 PRIMARY 或 SECONDARY 级别来源支持

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — Cargo.lock 验证依赖兼容，cc-switch 验证可行性，axum 0.8 官方文档完整
- Architecture: HIGH — 基于 v2.0 域级调研（ARCHITECTURE.md），cc-switch 参考实现验证，Tauri State 模式已在项目中使用
- Pitfalls: HIGH — 基于 v2.0 域级调研（PITFALLS.md），Tauri GitHub Issues 验证生命周期问题，SSE 代理是已知难点但方案明确

**Research date:** 2026-03-13
**Valid until:** 2026-04-13（axum 0.8 稳定版，30 天内不太可能有破坏性变更）

---
*Phase: 08-proxy-core*
*Research completed: 2026-03-13*
