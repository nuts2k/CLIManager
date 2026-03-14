---
phase: 8
slug: proxy-core
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-13
audited: 2026-03-14
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust 内置 `#[cfg(test)]` + `cargo test` |
| **Config file** | 无独立配置——Cargo.toml `[dev-dependencies]` |
| **Quick run command** | `cargo test --lib -p cli-manager -- proxy` |
| **Full suite command** | `cargo test --lib -p cli-manager` |
| **Estimated runtime** | <1 second (57 tests) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib -p cli-manager -- proxy`
- **After every plan wave:** Run `cargo test --lib -p cli-manager`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** <1 second

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 08-01-T3 | 01 | 1 | PROXY-01 | integration | `cargo test --lib -p cli-manager -- proxy::server::tests::test_proxy_forward` | proxy/server.rs | ✅ green |
| 08-01-T3 | 01 | 1 | PROXY-02 | integration | `cargo test --lib -p cli-manager -- proxy::server::tests::test_sse_streaming` | proxy/server.rs | ✅ green |
| 08-01-T2 | 01 | 1 | PROXY-03 | unit+e2e | `cargo test --lib -p cli-manager -- proxy::handler::tests::test_credential_replacement` + `cargo test --lib -p cli-manager -- proxy::server::tests::test_credential_replacement_e2e` | proxy/handler.rs, proxy/server.rs | ✅ green |
| 08-02-T1 | 02 | 2 | PROXY-04 | integration | `cargo test --lib -p cli-manager -- proxy::tests::test_proxy_service_dual_port` | proxy/mod.rs | ✅ green |
| 08-01-T1 | 01 | 1 | PROXY-05 | unit+e2e | `cargo test --lib -p cli-manager -- proxy::error::tests::test_upstream_unreachable_returns_502` + `cargo test --lib -p cli-manager -- proxy::server::tests::test_upstream_unreachable` | proxy/error.rs, proxy/server.rs | ✅ green |
| 08-01-T2 | 01 | 1 | UX-03 | unit+integration | `cargo test --lib -p cli-manager -- proxy::handler::tests::test_health_handler_returns_ok` + `cargo test --lib -p cli-manager -- proxy::server::tests::test_server_start_stop` | proxy/handler.rs, proxy/server.rs | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Complete Test Inventory (57 tests)

### proxy::error::tests (5 tests)
- `test_upstream_unreachable_returns_502` — PROXY-05: 502 状态码 + JSON 格式
- `test_no_upstream_configured_returns_503` — 503 状态码
- `test_bind_failed_returns_500` — 500 状态码
- `test_error_json_format` — PROXY-05: {"error":{"type":"proxy_error","message":"..."}} 结构验证
- `test_serialize_as_string` — Serialize trait 输出字符串格式

### proxy::state::tests (5 tests)
- `test_new_state_has_no_upstream` — 初始状态无 upstream
- `test_update_upstream` — 更新后可读取
- `test_clear_upstream` — 清除后为 None
- `test_update_upstream_replaces_previous` — 覆盖更新
- `test_upstream_target_fields` — 字段访问

### proxy::handler::tests (6 tests)
- `test_health_handler_returns_ok` — UX-03: GET /health 返回 200 + {"status":"ok"}
- `test_is_hop_by_hop` — hop-by-hop header 判断
- `test_credential_replacement_anthropic_placeholder` — PROXY-03: Anthropic x-api-key PROXY_MANAGED 替换
- `test_credential_replacement_openai_placeholder` — PROXY-03: OpenAI Bearer PROXY_MANAGED 替换
- `test_non_placeholder_credential_preserved` — PROXY-03: 非占位值原样保留
- `test_hop_by_hop_headers_filtered` — hop-by-hop headers 过滤

### proxy::server::tests (7 tests)
- `test_server_start_stop` — UX-03: 启动 + 健康检查 + 停止 + 端口释放
- `test_proxy_forward` — PROXY-01: 端到端请求转发
- `test_credential_replacement_e2e` — PROXY-03: 端到端凭据替换验证
- `test_sse_streaming` — PROXY-02: SSE text/event-stream 流式透传
- `test_upstream_unreachable` — PROXY-05: 502 + JSON 错误响应
- `test_bind_failed` — 端口占用返回 BindFailed
- `test_double_start` — 重复启动返回 AlreadyRunning

### proxy::tests (11 tests)
- `test_proxy_service_start_stop` — PROXY-04: 多实例独立启停
- `test_proxy_service_dual_port` — PROXY-04: 双端口同时运行各自转发
- `test_proxy_service_stop_all` — stop_all 停止所有代理
- `test_proxy_service_update_upstream` — 运行时动态切换上游
- `test_proxy_service_already_running` — 重复启动同一 cli_id 报错
- `test_proxy_service_stop_not_running` — 停止不存在的 cli_id 报错
- `test_proxy_port_for_cli_claude` — 端口映射 claude -> 15800
- `test_proxy_port_for_cli_codex` — 端口映射 codex -> 15801
- `test_proxy_port_for_cli_unknown` — 未知 cli_id 返回 None
- `test_proxy_service_update_upstream_not_running` — 更新不存在的 cli_id 报错
- `test_proxy_service_stop_timeout_keeps_server_retriable` — 停机超时后可重试

### commands::proxy::tests (10 tests)
- `test_make_proxy_provider_claude` — 代理 Provider 构造
- `test_make_proxy_provider_codex` — 代理 Provider 构造
- `test_build_upstream_target_from_provider_strips_legacy_path` — URL 路径清理
- `test_build_upstream_target_normalizes_base_url` — URL 规范化
- `test_build_upstream_target_rejects_base_url_with_path` — 路径拒绝
- `test_proxy_port_constants` — 端口常量验证
- `test_recover_on_startup_noop_when_no_takeover` — 启动恢复无操作
- `test_recover_on_startup_clears_takeover` — 启动恢复清除接管
- `test_recover_on_startup_keeps_failed_takeover` — 恢复失败保留接管
- `test_cleanup_on_exit_sync_noop_when_no_takeover` — 退出清理无操作
- `test_cleanup_on_exit_sync_restores_configs` — 退出恢复配置
- `test_cleanup_on_exit_sync_keeps_failed_takeover` — 清理失败保留接管
- `test_proxy_disable_keeps_takeover_when_restore_fails` — 禁用保留失败接管
- `test_proxy_mode_status_default` — 默认模式状态

---

## Wave 0 Requirements

- [x] `src-tauri/src/proxy/mod.rs` — proxy 模块入口 + ProxyService (431 行, 11 测试)
- [x] `src-tauri/src/proxy/server.rs` — ProxyServer 启停逻辑 + tests (489 行, 7 集成测试)
- [x] `src-tauri/src/proxy/handler.rs` — 请求转发 + 凭据替换 + tests (276 行, 6 单元测试)
- [x] `src-tauri/src/proxy/error.rs` — ProxyError + IntoResponse + tests (134 行, 5 单元测试)
- [x] `src-tauri/src/proxy/state.rs` — UpstreamTarget + ProxyState + tests (117 行, 5 单元测试)
- [x] `Cargo.toml` — 新增 axum 0.8, reqwest stream feature, tokio features, tower-http

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| SSE 实时性体感测试 | PROXY-02 | 延迟需人工感知 | 向代理发送流式请求，观察响应是否逐 chunk 到达 |
| macOS 防火墙弹窗 | UX-03 | 取决于系统安全设置 | 首次启动代理，观察是否触发防火墙弹窗（绑定 127.0.0.1 不应触发） |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 1s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** passed

---

## Validation Audit 2026-03-14

| Metric | Count |
|--------|-------|
| Requirements audited | 6 |
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
| Total proxy tests | 57 |
| All tests green | yes |
