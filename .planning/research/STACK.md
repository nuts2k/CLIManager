# Technology Stack

**Project:** CLIManager v2.0 Local Proxy
**Researched:** 2026-03-13

## Scope

本文档只覆盖 v2.0 本地代理功能所需的**增量**技术栈。现有 v1.1 技术栈（Tauri 2.10, React 19, Vite 7, shadcn/ui, Tailwind CSS v4, i18next, Rust 后端 serde/toml_edit/notify 等）已验证并上线，不在此重新评估。

---

## 核心发现：依赖复用率极高

Tauri 2 内部已经携带了构建 HTTP 代理所需的绝大部分底层依赖。当前 `Cargo.lock` 中已存在：

| 依赖 | 当前版本 | 来源 |
|------|---------|------|
| tokio | 1.50.0 | Tauri 2 async runtime |
| hyper | 1.8.1 | Tauri/reqwest 传递依赖 |
| hyper-util | 0.1.20 | Tauri/reqwest 传递依赖 |
| tower | 0.5.3 | Tauri 传递依赖 |
| tower-http | 0.6.8 | Tauri 传递依赖 |
| bytes | 1.11.1 | Tauri/reqwest 传递依赖 |
| http | 1.x | Tauri/reqwest 传递依赖 |
| reqwest | 0.12.28 | 已在 Cargo.toml（用于 test_provider） |

这意味着新增的 crate 不会引入大量新的传递依赖，编译时间和产物体积影响极小。

**置信度:** HIGH -- 通过直接检查 `Cargo.lock` 验证。

---

## 推荐新增依赖

### 核心 HTTP 服务器框架

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| axum | 0.8 | HTTP 路由和请求处理 | axum 是 tokio 官方生态的 HTTP 框架，基于 hyper 1.x + tower 0.5 构建。**与 Tauri 2 的 tokio runtime 完全兼容**（共享同一 tokio 实例），无需创建独立 runtime。cc-switch 已验证此方案可行（使用 axum 0.7 + Tauri 2）。axum 0.8 是当前稳定版（0.8.8，2025-12-20 发布），相比 cc-switch 的 0.7 更新了路径语法但核心 API 稳定。 |

**置信度:** HIGH

**为什么选 axum 而不是其他：**
- **vs hyper 直接使用：** axum 在 hyper 之上提供路由、提取器、中间件组合，代码量减少 60%+。axum 本身开销极小，性能与直接使用 hyper 相当。
- **vs actix-web：** actix 有自己的 async runtime，与 Tauri 的 tokio runtime 不兼容，需要开独立线程运行，增加复杂度和通信开销。
- **vs warp：** warp 维护活跃度下降，且 filter 组合模式对于代理场景不如 axum 的 handler + State 模式直观。
- **vs axum-reverse-proxy 第三方 crate：** 该 crate 带来了负载均衡、DNS 服务发现、WebSocket 转发等 v2.0 不需要的功能。我们的需求是动态切换单一上游目标，用 `Arc<RwLock<>>` + 自定义 handler 更简洁可控。
- **vs Pingora/Rama：** 生产级代理框架，为 Cloudflare 规模设计，对桌面应用本地代理严重过度工程化。

### reqwest 功能扩展（非新 crate）

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| reqwest `stream` feature | 0.12 (已有) | 启用 `Response::bytes_stream()` 方法 | SSE 流式转发的关键：上游返回 `text/event-stream` 响应时，用 `bytes_stream()` 逐 chunk 读取，再通过 `axum::body::Body::from_stream()` 零缓冲转发给客户端。不启用此 feature 则无法流式代理，必须缓冲完整响应才能返回——对 AI API 的长流式响应不可接受。cc-switch 也使用了此 feature。 |

**置信度:** HIGH -- cc-switch 的 `reqwest` 配置验证了 `stream` feature 用于 SSE 代理。

### tower-http CORS 中间件

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| tower-http `cors` feature | 0.6 (已在 lock) | 代理服务器 CORS 支持 | CLI 工具直接 HTTP 连接不需要 CORS，但预留此中间件成本为零（tower-http 已在依赖树中）。cc-switch 在代理服务器上也配置了 `CorsLayer`。 |

**置信度:** HIGH

---

## 完整 Cargo.toml 变更

```toml
# src-tauri/Cargo.toml

[dependencies]
# ... 现有依赖保持不变 ...

# 修改：reqwest 增加 stream feature
reqwest = { version = "0.12", features = ["json", "stream"] }

# 新增：HTTP 服务器框架（用于本地代理）
axum = "0.8"

# 新增：CORS 中间件（tower-http 已在依赖树中，只需显式声明使用 cors feature）
tower-http = { version = "0.6", features = ["cors"] }

# 新增：tokio 显式声明（Tauri 已携带 tokio 1.50，显式声明确保编译器可见 net/sync/time API）
tokio = { version = "1", features = ["net", "sync", "time"] }
```

### 为什么需要显式声明 tokio

Tauri 2 内部依赖 tokio 但不对外暴露所有 feature flags。代理服务器需要：
- `net`：`TcpListener::bind()` 绑定端口
- `sync`：`RwLock`, `oneshot` 通道
- `time`：`timeout()` 用于优雅停机

通过在 `Cargo.toml` 中显式声明 tokio，Cargo 的 feature unification 机制会合并所有 feature，不会引入新的 tokio 实例。

**置信度:** HIGH -- cc-switch 和 Tauri 社区文档均显式声明 tokio。

---

## 不需要新增的依赖

| 类别 | 不需要的 | 原因 |
|------|---------|------|
| HTTP 框架 | actix-web, warp, rocket | axum 与 Tauri tokio runtime 原生兼容，无需额外 runtime |
| 代理库 | axum-reverse-proxy, hyper-reverse-proxy | 自定义 handler 更简洁；这些库带来不需要的负载均衡/DNS 发现功能 |
| 生产级代理 | pingora, rama, sozu | 为 CDN/云规模设计，桌面应用不需要 |
| SSE 解析 | reqwest-eventsource, eventsource-stream | v2.0 做透明转发不解析 SSE 事件内容，`bytes_stream()` 逐 chunk 透传即可 |
| 额外序列化 | serde_yaml, json5, regex | v2.0 代理不需要新的格式解析 |
| 数据库 | rusqlite, sqlx | v2.0 代理设置存 `local.json`（现有存储层），不需要数据库 |
| 异步流工具 | futures, async-stream | `bytes_stream()` 返回的 `impl Stream<Item=Result<Bytes>>` 可直接传给 `Body::from_stream()`，无需额外的 stream 组合器 |
| 连接池 | deadpool, bb8 | reqwest 内置连接池，桌面应用单用户不需要外部池 |

---

## 架构集成：Tauri tokio Runtime 内启动 axum

### 启动模式

在 `lib.rs` 的 `.setup()` 钩子中使用 `tauri::async_runtime::spawn()` 启动 axum 服务器：

```rust
// lib.rs setup() 中
let proxy_state = Arc::new(ProxyManager::new());
app.manage(proxy_state.clone());

tauri::async_runtime::spawn(async move {
    if let Err(e) = proxy_manager.start_if_enabled().await {
        log::error!("代理服务器启动失败: {}", e);
    }
});
```

**关键点：**
1. `tauri::async_runtime::spawn()` 在 Tauri 管理的 tokio runtime 中调度任务。
2. **不需要** `#[tokio::main]` 或独立 `tokio::runtime::Runtime::new()`。
3. **不需要** `std::thread::spawn` + 独立 runtime（那是 actix 等不兼容 runtime 的 workaround）。
4. axum 的 `serve()` 是一个异步函数，天然运行在 tokio executor 上。

**置信度:** HIGH -- [Tauri 官方文档](https://docs.rs/tauri/latest/tauri/async_runtime/index.html)和 cc-switch 代码均验证此模式。

### SSE 流式转发模式

```rust
// 代理 handler 核心逻辑（伪代码）
async fn proxy_handler(
    State(state): State<Arc<ProxyState>>,
    req: axum::extract::Request,
) -> axum::response::Response {
    // 1. 从共享状态读取当前上游目标
    let upstream = state.get_current_upstream(&cli_type).await;

    // 2. 构造 reqwest 请求，注入上游 API key 和 base URL
    let client = &state.http_client;
    let upstream_resp = client
        .post(&format!("{}{}", upstream.base_url, path))
        .headers(forward_headers(&req))
        .header("Authorization", format!("Bearer {}", upstream.api_key))
        .body(reqwest::Body::from(body_bytes))
        .send()
        .await?;

    // 3. 流式转发响应（SSE 和非 SSE 统一处理）
    let mut builder = axum::response::Response::builder()
        .status(upstream_resp.status());
    for (k, v) in upstream_resp.headers() {
        builder = builder.header(k, v);
    }
    let body = axum::body::Body::from_stream(
        upstream_resp.bytes_stream()
    );
    builder.body(body).unwrap()
}
```

**关键设计决策：透明转发而非 SSE 解析**

v2.0 代理不需要理解 SSE 事件的语义内容（那是 2.x 流量监控的事）。只需要：
1. 保持 `Content-Type: text/event-stream` header 原样传递
2. 用 `bytes_stream()` 逐 chunk 流式转发，不缓冲
3. 不加压缩层（或排除 `text/event-stream` MIME type）

这使得实现极其简单，且对所有 API 协议（Anthropic、OpenAI）通用。

**置信度:** HIGH -- 这正是 cc-switch `response_processor.rs` 的核心模式。

### 动态上游切换模式

```rust
struct ProxyState {
    // 每个 CLI 类型的当前上游目标
    upstreams: RwLock<HashMap<CliType, UpstreamTarget>>,
    // 共享 HTTP 客户端（连接池复用）
    http_client: reqwest::Client,
}

struct UpstreamTarget {
    base_url: String,
    api_key: String,
    // 其他需要注入的字段
}

impl ProxyState {
    /// Provider 切换时调用，立即生效
    async fn switch_upstream(&self, cli_type: CliType, target: UpstreamTarget) {
        let mut upstreams = self.upstreams.write().await;
        upstreams.insert(cli_type, target);
        // 下一个请求自动使用新目标，无需重启服务器
    }
}
```

**为什么用 `tokio::sync::RwLock` 而不是 `std::sync::RwLock`：**
- 读操作（每个代理请求都会读）远多于写操作（只在 Provider 切换时写）
- `tokio::sync::RwLock` 的 `.read().await` 不阻塞 tokio worker thread
- `std::sync::RwLock` 在异步上下文中可能导致 thread 阻塞，影响其他任务

**置信度:** HIGH -- 标准 Rust 异步模式，cc-switch 广泛使用此模式。

---

## 端口分配策略

| CLI | 默认端口 | 说明 |
|-----|---------|------|
| Claude Code | 9960 | 固定端口，代理模式下 CLI 配置 patch 为 `http://127.0.0.1:9960` |
| Codex | 9961 | 固定端口，代理模式下 CLI 配置 patch 为 `http://127.0.0.1:9961` |

**实现方式：** 每个 CLI 类型一个独立的 `TcpListener` + axum `Router`，各自 `tokio::spawn` 运行。不使用单一服务器 + 路径前缀区分的原因是：
1. CLI 工具配置的 base URL 格式固定（如 Claude Code 期望 `/v1/messages` 路径），加前缀需要 CLI 修改配置
2. 独立端口使每个 CLI 可以独立启停，互不影响
3. 与 cc-switch 的单端口 + 路径前缀方案相比，独立端口更简单——不需要处理 `/claude/v1/messages` vs `/codex/v1/chat/completions` 的路由歧义

**置信度:** MEDIUM -- 端口号为参考值，需确认不与常见服务冲突。独立端口 vs 单端口 + 前缀的取舍需在实现阶段确认。

---

## Alternatives Considered

| 类别 | 推荐 | 备选 | 不选原因 |
|------|-----|------|---------|
| HTTP 框架 | axum 0.8 | actix-web 4 | actix 有独立 runtime，与 Tauri tokio 不兼容 |
| HTTP 框架 | axum 0.8 | warp 0.3 | warp 维护活跃度下降，filter 模式对代理场景不直观 |
| 代理实现 | 自定义 handler | axum-reverse-proxy 1.0 | 第三方 crate 引入不需要的负载均衡/DNS 发现；我们需要动态 upstream 切换，自己写更可控 |
| SSE 转发 | bytes_stream() 透传 | SSE 事件解析再发射 | v2.0 不需要理解事件内容，透传更简单更高效 |
| 共享状态 | `Arc<RwLock<HashMap>>` | 每次从文件重读 | RwLock 读取纳秒级，文件 I/O 毫秒级；代理路径不应有磁盘 I/O |
| 端口模式 | 每 CLI 独立端口 | 单端口 + 路径前缀 | 独立端口避免路径重写，CLI 配置更简单 |
| 上游 HTTP 客户端 | reqwest 0.12 (已有) | hyper client 直接使用 | reqwest 是 hyper 的高层封装，已在项目中，API 更友好 |

---

## 安装总结

```toml
# src-tauri/Cargo.toml 变更

# 修改（增加 stream feature）:
reqwest = { version = "0.12", features = ["json", "stream"] }

# 新增:
axum = "0.8"
tower-http = { version = "0.6", features = ["cors"] }
tokio = { version = "1", features = ["net", "sync", "time"] }
```

**新增 crate 数量：** 1 个直接新增（axum），2 个显式声明已有传递依赖（tower-http, tokio）。实际新增传递依赖极少，因为 axum 依赖的 hyper/tower/tokio/bytes/http 全部已在 Cargo.lock 中。

**无新增 npm 包。** 前端只需增加代理开关的 UI 组件和对应的 Tauri 命令调用。

---

## 与 cc-switch 的对比

| 维度 | cc-switch | CLIManager v2.0 | 原因 |
|------|----------|-----------------|------|
| axum 版本 | 0.7 | 0.8 | 0.8 是当前稳定版，路径语法更新但核心 API 同 |
| 端口模式 | 单端口 + 路径前缀 | 每 CLI 独立端口 | 避免路径重写复杂度 |
| 数据库 | rusqlite | 无（local.json） | v2.0 只存开关状态和端口，JSON 足够 |
| 协议转换 | Anthropic ↔ OpenAI 双向 | 无（透传） | 明确标记为 Out of Scope（2.x 里程碑） |
| 熔断器 | 有（CircuitBreaker） | 无 | v2.0 不做 failover |
| Usage 统计 | 有（解析流并统计 token） | 无 | v2.0 不做流量监控 |
| SSE 处理 | 解析事件、统计 token、协议转换 | 透明转发 | v2.0 代理只做路由，不做内容处理 |

**cc-switch 代理模块约 2000+ 行 Rust 代码，CLIManager v2.0 代理核心预计 300-500 行**——因为我们只做透传，不做协议转换/熔断/统计。

---

## Sources

- [Tauri async_runtime 文档](https://docs.rs/tauri/latest/tauri/async_runtime/index.html) -- 在 Tauri 内 spawn 异步任务的官方 API (HIGH)
- [axum GitHub](https://github.com/tokio-rs/axum) -- HTTP 框架官方仓库 (HIGH)
- [axum 0.8.0 公告](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) -- 版本发布说明 (HIGH)
- [axum 官方 reverse-proxy 示例](https://github.com/tokio-rs/axum/blob/main/examples/reverse-proxy/src/main.rs) -- reqwest + axum 代理模式 (HIGH)
- [axum 官方 SSE 示例](https://github.com/tokio-rs/axum/blob/main/examples/sse/src/main.rs) -- SSE 响应支持 (HIGH)
- [Tauri + Async Rust Process](https://rfdonnelly.github.io/posts/tauri-async-rust-process/) -- 在 Tauri 内运行异步任务的模式 (MEDIUM)
- [Tauri GitHub Discussion #2942](https://github.com/tauri-apps/tauri/discussions/2942) -- 在 Tauri 内运行 HTTP 服务器 (MEDIUM)
- [Static streams for faster async proxies](https://blog.adamchalmers.com/streaming-proxy/) -- 流式代理架构决策 (MEDIUM)
- [axum + reqwest 代理讨论](https://github.com/tokio-rs/axum/discussions/1821) -- 大文件/流式代理最佳实践 (MEDIUM)
- cc-switch `src-tauri/src/proxy/server.rs` -- 工作参考：axum + Tauri 代理服务器实现 (HIGH)
- cc-switch `src-tauri/src/proxy/response_processor.rs` -- 工作参考：`bytes_stream()` SSE 透传 (HIGH)
- cc-switch `src-tauri/src/proxy/forwarder.rs` -- 工作参考：reqwest 请求转发 (HIGH)
- cc-switch `src-tauri/Cargo.toml` -- 验证依赖：axum 0.7, reqwest stream, tokio features (HIGH)
- CLIManager `src-tauri/Cargo.lock` -- 直接验证已有传递依赖版本 (HIGH)
