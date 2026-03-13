# Phase 8: 代理核心 - Context

**Gathered:** 2026-03-13
**Status:** Ready for planning

<domain>
## Phase Boundary

每个 CLI 拥有独立端口的本地 HTTP 代理服务器，能将请求转发到上游 Provider 并支持 SSE 流式响应。代理自动将占位 API key 替换为当前活跃 Provider 的真实凭据。Phase 8 只做代理服务器本身，不包含模式切换、CLI 配置 patch、崩溃恢复（Phase 9）或实时切换 UI（Phase 10）。

</domain>

<decisions>
## Implementation Decisions

### API 路径路由策略
- 全路径透传：代理不理解路径语义，任何 HTTP 请求原样转发到上游 base_url
- 端口即身份：15800=Claude Code, 15801=Codex，代理通过监听端口判断请求属于哪个 CLI
- 上游 URL 拼接：Provider.base_url + 原始请求路径（如 `https://api.anthropic.com` + `/v1/messages`）
- 请求头最小化替换：保留原始请求头，只替换 Authorization/x-api-key 为真实凭据

### 占位凭据格式
- 占位值为统一字符串 "PROXY_MANAGED"（与 cc-switch 一致）
- 匹配占位才替换：代理检查请求中的 auth 头/key 是否为 "PROXY_MANAGED"，是则替换为真实 key；非占位值的请求正常转发不修改

### 代理复杂度边界
- 纯透传代理：接收请求 → 替换 key → 转发上游 → 透传响应（包括 SSE 流式）
- 不做请求体解析/修改（不包含 thinking rectifier、body filter、model mapping 等）
- 架构预留中间件插槽：方便未来添加请求/响应处理中间件而无需重构核心转发逻辑
- 复用 reqwest 作为上游 HTTP 客户端（已在 Cargo.toml 中，需启用 stream feature）

### 错误与健康检查
- 统一代理错误格式：代理自身错误（如无法连接上游）返回 502 + JSON（`{"error": {"type": "proxy_error", "message": "..."}}`）
- 上游错误原样透传：上游返回的非 2xx 响应（429/400 等）保留原始状态码和响应体直接转发给 CLI
- GET /health 只返回 `{"status": "ok"}`，用于启动后健康自检
- 不设代理端超时，完全依赖 CLI 自身的请求超时设置

### 代理生命周期
- Phase 8 提供 start()/stop()/status() API，由上层控制启停时机（Phase 9 决定何时调用）
- 代理服务作为 Tauri 托管状态（app.manage(ProxyService)），通过 Tauri 命令暴露
- 停止代理只停 axum 服务器监听，不触及 CLI 配置（配置还原是 Phase 9 职责）
- 启动时注入当前活跃 Provider 信息，运行时通过 update_upstream() 方法动态切换上游目标（Phase 10 利用此接口）

### Claude's Discretion
- SSE 流式透传的具体实现方式（逐 chunk vs buffered）
- axum Router 和 Handler 的代码组织方式
- 多端口架构：每端口一个独立 axum Server 实例 vs 共享状态的多 listener
- reqwest Client 配置（连接池、keep-alive 等）
- 代理内部错误日志级别和格式

</decisions>

<specifics>
## Specific Ideas

- cc-switch 用 axum 0.7 做代理（参考 `cc-switch/src-tauri/src/proxy/`），我们用 axum 0.8，路径语法从 `/:param` 变为 `/{param}`
- cc-switch 的 ProxyServer 使用 oneshot channel 做优雅停机，可以参考
- cc-switch 单端口 + 路径前缀区分 CLI，但我们用双端口方案更简洁（每端口绑定固定 CLI，无需路径解析）

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Provider` struct（provider.rs）：包含 api_key, base_url, model, protocol_type, cli_id 等所有代理转发需要的信息
- `storage::local` 模块：LocalSettings 读写，代理设置可复用此存储模式
- `reqwest` crate（Cargo.toml 已有）：用于上游 HTTP 请求转发
- `SelfWriteTracker`（watcher 模块）：防循环写入模式，代理修改 Provider 时可能需要

### Established Patterns
- Tauri 托管状态模式（`app.manage(T)` + `State<T>`）：代理服务应遵循此模式
- 命令层（commands/）+ 业务层分离：代理应有 commands/proxy.rs + 独立 proxy 模块
- 事件发射（`providers-changed`）：代理状态变更可复用事件机制通知前端

### Integration Points
- `lib.rs` 的 setup 闭包：注册 ProxyService 为 Tauri State
- `lib.rs` 的 invoke_handler：注册代理相关 Tauri 命令（start/stop/status）
- `Cargo.toml`：需添加 axum 0.8 依赖，reqwest 需启用 stream feature
- tokio runtime：Tauri 2 内置 tokio，axum 服务器直接 spawn 在同一 runtime 上

</code_context>

<deferred>
## Deferred Ideas

- Thinking Rectifier / Body Filter / Model Mapping — 作为中间件插槽的未来实现，如果在多 Provider 切换中遇到兼容性问题再考虑
- 代理流量统计与可视化 — v2.x+ 里程碑（TRAF-01, TRAF-02）
- 自动 Failover / 熔断器 — v2.x+ 里程碑（ADV-01, ADV-02）
- 自定义代理端口 — v2.x+ 里程碑（ADV-05）
- 协议转换（Anthropic↔OpenAI）— v2.x+ 里程碑（PROTO-01, PROTO-02）

</deferred>

---

*Phase: 08-proxy-core*
*Context gathered: 2026-03-13*
