# Project Research Summary

**Project:** CLIManager v2.0 Local Proxy
**Domain:** 桌面应用内嵌 HTTP 反向代理（AI CLI 配置切换加速）
**Researched:** 2026-03-13
**Confidence:** HIGH

## Executive Summary

CLIManager v2.0 的核心功能是在 Tauri 桌面应用内运行本地 HTTP 代理服务器，使 AI CLI 工具（Claude Code、Codex）通过代理访问上游 API Provider。这解决了 v1.x 直连模式下切换 Provider 需要修改配置文件并重启 CLI 的痛点——代理模式下切换 Provider 只需更新内存中的上游目标，对 CLI 完全透明，下一个请求立即生效。

技术栈选型极为有利：Tauri 2 内部已携带 tokio 1.50、hyper 1.8.1、tower 0.5.3、tower-http 0.6.8 等关键依赖，新增 axum 0.8 作为 HTTP 框架几乎不引入新的传递依赖。代理核心逻辑（请求转发 + SSE 流式透传 + 动态上游切换）在 cc-switch 中已有成熟的参考实现，且 CLIManager v2.0 的范围远比 cc-switch 精简——不做协议转换、熔断器、Usage 统计，预计核心代码量 300-500 行。

主要风险点在于：(1) Tauri tokio runtime 与 axum 服务器的生命周期管理（启动、停机、端口占用），(2) 代理模式与直连模式的双模式切换需要精确协调 CLI 配置 patch，(3) SSE 流式转发必须逐 chunk 透传不能缓冲。这些都是工程实现层面的问题，无技术不可行性。

## Key Findings

**Stack:** 仅需新增 axum 0.8（1 个真正新增 crate），显式声明 tokio 和 tower-http（均为已有传递依赖），reqwest 增加 `stream` feature。axum 与 Tauri 的 tokio runtime 完全兼容，通过 `tauri::async_runtime::spawn()` 启动。不需要 axum-reverse-proxy、pingora、actix-web 等第三方方案。

**Architecture:** 每个 CLI 类型独立端口运行 axum 服务器，共享 `Arc<RwLock<>>` 状态实现动态上游切换。SSE 流式转发使用 `reqwest::Response::bytes_stream()` + `axum::body::Body::from_stream()` 逐 chunk 透传。

**Critical pitfall:** 代理模式和直连模式的切换必须原子性地协调——关闭代理前必须先将 CLI 配置 patch 回直连地址，否则 CLI 指向已关闭的 localhost 端口会完全不可用。另外 macOS 防火墙弹窗可通过绑定 `127.0.0.1`（而非 `0.0.0.0`）完全避免。

## Implications for Roadmap

Based on research, suggested phase structure:

1. **Phase 1: 代理核心（最小可用）** - 先让代理能跑起来
   - Addresses: axum 服务器启动/停机、请求转发（含 SSE 流式）、动态上游切换、健康检查端点
   - Avoids: 过早做 UI 或双模式切换逻辑，先验证核心代理功能
   - Stack: axum 0.8 + reqwest (stream) + tokio (net, sync, time) + tower-http (cors)

2. **Phase 2: 代理设置持久化 + 模式切换** - 让代理状态可管理
   - Addresses: local.json 存储代理开关/端口配置、直连 vs 代理模式切换、CLI 配置 patch 联动、watcher 模式感知
   - Avoids: 在核心未验证前做 UI

3. **Phase 3: UI 集成 + 完善** - 让用户能操作
   - Addresses: 设置页全局开关、Tab 内独立开关、托盘菜单联动、错误提示、启动时状态恢复
   - Avoids: 在模式切换逻辑未完善前做 UI

**Phase ordering rationale:**
- Phase 1 必须先行，因为代理服务器是后续所有功能的基础
- Phase 2 在核心稳定后加入持久化和模式切换，这是最复杂的业务逻辑层
- Phase 3 最后做 UI，因为 UI 只是现有逻辑的展示层，且可以在 Phase 1-2 期间用 Tauri 命令手动测试

**Research flags for phases:**
- Phase 1: 标准模式，不需要额外调研（axum + reqwest 文档充足，cc-switch 有完整参考）
- Phase 2: 需要关注模式切换的边界情况（CLI 正在使用代理时切换模式的处理）
- Phase 3: 标准 React UI 开发，不需要调研

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | axum + tokio + reqwest 组合成熟，cc-switch 已验证，Cargo.lock 确认依赖兼容。仅 1 个真正新增 crate。 |
| Features | HIGH | 功能需求明确（PROJECT.md Active 列表），cc-switch 有完整参考实现。明确的 Anti-Features 边界（不做协议转换/熔断/统计）。 |
| Architecture | HIGH | Tauri 内 spawn axum 服务器的模式有官方文档和社区验证。`Arc<RwLock<>>` 动态上游切换是标准模式。 |
| Pitfalls | HIGH | 7 个已识别的关键陷阱均有明确的预防策略和阶段分配。Tauri 生命周期管理的坑已通过 GitHub Issues 验证。 |

## Gaps to Address

- **端口冲突检测策略：** 如何处理用户系统上默认端口已被占用的情况（建议：启动失败时返回明确错误，不自动寻找端口）
- **macOS 防火墙权限：** 即使绑定 127.0.0.1，首次运行是否仍需网络权限 entitlement（需在打包时测试）
- **代理模式下 CLI 配置 patch 的具体字段映射：** Claude Code 的 `ANTHROPIC_BASE_URL` 通过 `settings.json` 的 `env` 块设置，Codex 的 `base_url` 通过 `config.toml` 设置——需确认占位 API key 的具体格式要求
- **应用退出时的顺序：** 先 patch 回直连配置 -> 再停代理 -> 最后退出。但 Tauri 的 `app.exit()` 调用 `std::process::exit()` 不触发 drop——需要在退出逻辑中显式执行
- **cc-switch 使用 axum 0.7 而我们使用 0.8：** 路径语法从 `/:param` 变为 `/{param}`，需注意迁移，但核心 API 不变

## Sources

### Primary (HIGH confidence)
- [Tauri async_runtime 文档](https://docs.rs/tauri/latest/tauri/async_runtime/index.html)
- [axum GitHub](https://github.com/tokio-rs/axum) + [0.8.0 公告](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0)
- [axum 官方 reverse-proxy 示例](https://github.com/tokio-rs/axum/blob/main/examples/reverse-proxy/src/main.rs)
- cc-switch 代理实现：`proxy/server.rs`、`proxy/forwarder.rs`、`proxy/response_processor.rs`
- CLIManager `Cargo.lock` 直接依赖版本验证

### Secondary (MEDIUM confidence)
- [Tauri + Async Rust Process](https://rfdonnelly.github.io/posts/tauri-async-rust-process/)
- [Static streams for faster async proxies](https://blog.adamchalmers.com/streaming-proxy/)
- [axum + reqwest 代理讨论](https://github.com/tokio-rs/axum/discussions/1821)

---
*Research completed: 2026-03-13*
*Ready for roadmap: yes*
