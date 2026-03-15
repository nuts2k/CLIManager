---
phase: 14
slug: data-model-translate-core
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-14
---

# Phase 14 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust 内置 `cargo test`（rustc test harness） |
| **Config file** | `src-tauri/Cargo.toml`（`[dev-dependencies]`） |
| **Quick run command** | `cargo test --package cli-manager translate` |
| **Full suite command** | `cargo test --package cli-manager` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --package cli-manager translate`
- **After every plan wave:** Run `cargo test --package cli-manager`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 14-01-01 | 01 | 1 | MODL-01 | unit | `cargo test --package cli-manager provider::tests::test_upstream_model` | ❌ W0 | ⬜ pending |
| 14-01-02 | 01 | 1 | MODL-02 | unit | `cargo test --package cli-manager provider::tests::test_upstream_model_map` | ❌ W0 | ⬜ pending |
| 14-01-03 | 01 | 1 | MODL-01+02 | unit | `cargo test --package cli-manager provider::tests::test_new_fields_backward_compat` | ❌ W0 | ⬜ pending |
| 14-02-01 | 02 | 2 | REQT-01 | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ W0 | ⬜ pending |
| 14-02-02 | 02 | 2 | REQT-02 | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ W0 | ⬜ pending |
| 14-02-03 | 02 | 2 | REQT-03 | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ W0 | ⬜ pending |
| 14-02-04 | 02 | 2 | REQT-04 | unit | `cargo test --package cli-manager proxy::translate::request::tests::test_build_proxy_endpoint_url` | ❌ W0 | ⬜ pending |
| 14-02-05 | 02 | 2 | REQT-05 | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ W0 | ⬜ pending |
| 14-02-06 | 02 | 2 | REQT-06 | unit | `cargo test --package cli-manager proxy::translate::request::tests::test_clean_schema` | ❌ W0 | ⬜ pending |
| 14-02-07 | 02 | 2 | REQT-07 | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ W0 | ⬜ pending |
| 14-02-08 | 02 | 2 | REQT-08 | unit | `cargo test --package cli-manager proxy::translate::request::tests` | ❌ W0 | ⬜ pending |
| 14-03-01 | 03 | 2 | RESP-01 | unit | `cargo test --package cli-manager proxy::translate::response::tests` | ❌ W0 | ⬜ pending |
| 14-03-02 | 03 | 2 | RESP-02 | unit | `cargo test --package cli-manager proxy::translate::response::tests` | ❌ W0 | ⬜ pending |
| 14-03-03 | 03 | 2 | RESP-03 | unit | `cargo test --package cli-manager proxy::translate::response::tests` | ❌ W0 | ⬜ pending |
| 14-03-04 | 03 | 2 | RESP-04 | unit | `cargo test --package cli-manager proxy::translate::response::tests` | ❌ W0 | ⬜ pending |
| 14-03-05 | 03 | 2 | RESP-05 | manual | n/a — Phase 15 handler 集成测试覆盖 | ❌ Out of scope | ⬜ pending |
| 14-04-01 | 04 | 2 | STRM-01 | unit (async) | `cargo test --package cli-manager proxy::translate::stream::tests` | ❌ W0 | ⬜ pending |
| 14-04-02 | 04 | 2 | STRM-02 | unit (async) | `cargo test --package cli-manager proxy::translate::stream::tests::test_streaming_delays_tool_start` | ❌ W0 | ⬜ pending |
| 14-04-03 | 04 | 2 | STRM-03 | unit (async) | `cargo test --package cli-manager proxy::translate::stream::tests::test_streaming_tool_calls_routed_by_index` | ❌ W0 | ⬜ pending |
| 14-04-04 | 04 | 2 | STRM-04 | unit (async) | `cargo test --package cli-manager proxy::translate::stream::tests` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/Cargo.toml` — 新增 `bytes = "1"` + `futures = "0.3"` + `async-stream = "0.3"` 依赖
- [ ] `src-tauri/src/proxy/translate/mod.rs` — 模块声明文件
- [ ] `src-tauri/src/proxy/translate/request.rs` — `anthropic_to_openai()` + 测试 stubs
- [ ] `src-tauri/src/proxy/translate/response.rs` — `openai_to_anthropic()` + 测试 stubs
- [ ] `src-tauri/src/proxy/translate/stream.rs` — `create_anthropic_sse_stream()` + 测试 stubs
- [ ] `src-tauri/src/proxy/mod.rs` — 新增 `pub mod translate;`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| 4xx/5xx 错误响应直接透传 | RESP-05 | handler 层逻辑，不在 translate 层；需 Phase 15 集成测试 | Phase 15 handler 集成时验证非 2xx 响应不调用 `openai_to_anthropic()` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
