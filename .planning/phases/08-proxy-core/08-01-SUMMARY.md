---
phase: 08-proxy-core
plan: 01
subsystem: proxy
tags: [axum, reqwest, sse, http-proxy, tokio, streaming]

# Dependency graph
requires: []
provides:
  - "ProxyServer: 单端口 HTTP 代理服务器，支持启停生命周期管理"
  - "proxy_handler: 全路径透传 + 凭据替换 + SSE 流式透传"
  - "health_handler: GET /health 健康检查端点"
  - "ProxyState: 动态上游切换共享状态（Arc<RwLock>）"
  - "ProxyError: 9 变体错误枚举 + IntoResponse + Serialize"
  - "UpstreamTarget: 上游目标结构体（api_key, base_url, protocol_type）"
affects: [08-02, 09-mode-switch, 10-realtime-ui]

# Tech tracking
tech-stack:
  added: [axum 0.8, tower-http 0.6, tokio (显式声明), futures 0.3 (dev)]
  patterns: [axum fallback handler 全路径透传, bytes_stream + Body::from_stream SSE 零缓冲管道, oneshot channel 优雅停机, Arc<RwLock> 动态状态共享]

key-files:
  created:
    - src-tauri/src/proxy/error.rs
    - src-tauri/src/proxy/state.rs
    - src-tauri/src/proxy/handler.rs
    - src-tauri/src/proxy/server.rs
    - src-tauri/src/proxy/mod.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs

key-decisions:
  - "健康自检使用 no_proxy 客户端，避免 Surge 等系统代理拦截本地请求"
  - "凭据注入仅在检测到 PROXY_MANAGED 占位值时触发，非占位值原样保留"
  - "reqwest Client 由外部传入 ProxyServer，便于测试时注入 no_proxy 配置"

patterns-established:
  - "axum fallback handler: 全路径透传代理模式，/health 路由优先，其余 fallback"
  - "SSE 流式透传: reqwest bytes_stream() -> axum Body::from_stream() 零缓冲管道"
  - "代理状态共享: ProxyState(Arc<RwLock<Option<UpstreamTarget>>>) + Clone"
  - "集成测试: mock 上游 axum 服务器 + 动态端口 + oneshot 停机"

requirements-completed: [PROXY-01, PROXY-02, PROXY-03, PROXY-05, UX-03]

# Metrics
duration: 10min
completed: 2026-03-13
---

# Phase 8 Plan 1: 代理核心引擎 Summary

**axum 0.8 单端口 HTTP 代理服务器，支持全路径透传、SSE 流式透传、占位凭据替换、优雅停机与健康自检**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-13T11:13:11Z
- **Completed:** 2026-03-13T11:24:04Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments
- proxy/ 模块完整实现：error.rs, state.rs, handler.rs, server.rs, mod.rs 五个文件
- 全路径透传 + SSE 流式响应（bytes_stream + Body::from_stream 零缓冲）
- Anthropic/OpenAI 双协议凭据替换（仅替换 PROXY_MANAGED 占位值）
- ProxyServer 完整启停生命周期：启动后健康自检、oneshot channel 优雅停机（5s 超时）
- 23 个测试全部通过（5 错误映射 + 5 状态操作 + 6 handler 逻辑 + 7 集成测试）

## Task Commits

Each task was committed atomically:

1. **Task 1: 添加依赖 + 创建 proxy 模块基础类型** - `6430354` (feat)
2. **Task 2: 实现请求转发 handler + 健康检查端点** - `ff8dee4` (feat)
3. **Task 3: 实现 ProxyServer 启停生命周期 + 集成测试** - `0f06175` (feat)

## Files Created/Modified
- `src-tauri/Cargo.toml` - 新增 axum 0.8, tower-http 0.6, tokio 显式声明, reqwest stream feature, futures dev-dep
- `src-tauri/src/lib.rs` - 注册 proxy 模块
- `src-tauri/src/proxy/error.rs` - ProxyError 枚举（9 变体）+ IntoResponse（502/503/500）+ Serialize
- `src-tauri/src/proxy/state.rs` - UpstreamTarget + ProxyState（Arc<RwLock> 动态上游切换）
- `src-tauri/src/proxy/handler.rs` - proxy_handler（全路径透传 + 凭据替换 + SSE）+ health_handler
- `src-tauri/src/proxy/server.rs` - ProxyServer（start/stop/is_running）+ build_router + health_check + 7 个集成测试
- `src-tauri/src/proxy/mod.rs` - 模块声明 + 公开导出

## Decisions Made
- 健康自检和集成测试使用 `reqwest::Client::builder().no_proxy().build()` 避免系统代理（Surge）拦截本地 127.0.0.1 请求
- reqwest Client 由外部注入 ProxyServer（构造函数参数），而非内部创建，方便测试控制和 Client 复用
- 凭据注入使用 `needs_credential_injection` 标志，仅当检测到占位值时触发，确保非代理管理的请求头原样保留

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 添加 futures dev-dependency**
- **Found during:** Task 3（SSE 流式测试）
- **Issue:** SSE 测试需要 `futures::stream::iter` 构造流式数据，但 futures crate 不在 dev-dependencies 中
- **Fix:** 在 Cargo.toml dev-dependencies 添加 `futures = "0.3"`
- **Files modified:** src-tauri/Cargo.toml
- **Committed in:** 0f06175

**2. [Rule 3 - Blocking] 使用 no_proxy 客户端绕过系统代理**
- **Found during:** Task 3（test_upstream_unreachable 测试）
- **Issue:** 系统安装的 Surge 代理软件拦截了本地 HTTP 请求，导致上游不可达测试收到 Surge 的 HTML 错误页面（503）而非预期的 502
- **Fix:** 所有测试客户端和 health_check 函数使用 `reqwest::Client::builder().no_proxy().build()` 绕过系统代理
- **Files modified:** src-tauri/src/proxy/server.rs
- **Committed in:** 0f06175

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** 两个修复都是环境兼容性问题，不影响功能设计。no_proxy 实际上是更健壮的实现。

## Issues Encountered
- Surge 系统代理拦截本地 127.0.0.1 请求 -- 通过 no_proxy 客户端解决，对生产代码也是更好的实践

## User Setup Required
None - 无需外部服务配置。

## Next Phase Readiness
- proxy/ 模块核心引擎完成，提供 ProxyServer::start/stop/state API
- Plan 02 将把 ProxyServer 包装为 Tauri 托管 ProxyService，注册 Tauri 命令
- ProxyState::update_upstream() 已支持运行时动态切换上游，供 Phase 10 使用

---
*Phase: 08-proxy-core*
*Completed: 2026-03-13*

## Self-Check: PASSED

All 5 proxy module files created. All 3 task commits verified. SUMMARY.md exists.
