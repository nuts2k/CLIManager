---
phase: 8
slug: proxy-core
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-13
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
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib -p cli-manager -- proxy`
- **After every plan wave:** Run `cargo test --lib -p cli-manager`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 08-01-xx | 01 | 1 | PROXY-01 | integration | `cargo test --lib -p cli-manager -- proxy::tests::test_proxy_forward` | ❌ W0 | ⬜ pending |
| 08-01-xx | 01 | 1 | PROXY-02 | integration | `cargo test --lib -p cli-manager -- proxy::tests::test_sse_streaming` | ❌ W0 | ⬜ pending |
| 08-01-xx | 01 | 1 | PROXY-03 | unit | `cargo test --lib -p cli-manager -- proxy::handler::tests::test_credential_replacement` | ❌ W0 | ⬜ pending |
| 08-01-xx | 01 | 1 | PROXY-04 | integration | `cargo test --lib -p cli-manager -- proxy::tests::test_dual_port` | ❌ W0 | ⬜ pending |
| 08-01-xx | 01 | 1 | PROXY-05 | unit | `cargo test --lib -p cli-manager -- proxy::error::tests::test_error_response_format` | ❌ W0 | ⬜ pending |
| 08-01-xx | 01 | 1 | UX-03 | integration | `cargo test --lib -p cli-manager -- proxy::tests::test_health_check` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/proxy/mod.rs` — proxy 模块入口 + ProxyService
- [ ] `src-tauri/src/proxy/server.rs` — ProxyServer 启停逻辑 + tests
- [ ] `src-tauri/src/proxy/handler.rs` — 请求转发 + 凭据替换 + tests
- [ ] `src-tauri/src/proxy/error.rs` — ProxyError + IntoResponse + tests
- [ ] `src-tauri/src/proxy/state.rs` — UpstreamTarget + ProxyState + tests
- [ ] `Cargo.toml` — 新增 axum 0.8, reqwest stream feature, tokio features, tower-http

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| SSE 实时性体感测试 | PROXY-02 | 延迟需人工感知 | 向代理发送流式请求，观察响应是否逐 chunk 到达 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
