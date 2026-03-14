---
phase: 15
slug: handler
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-14
---

# Phase 15 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust 内置 `cargo test`（rustc test harness） |
| **Config file** | `src-tauri/Cargo.toml` |
| **Quick run command** | `cargo test --package cli-manager proxy::handler proxy::mod proxy::state` |
| **Full suite command** | `cargo test --package cli-manager` |
| **Estimated runtime** | ~30 seconds |

**基线状态（Phase 14 验证后）:** 295 个测试，294 passed，1 failed（UX-01 遗留端口冲突，与 Phase 15 代码无关）。

---

## Sampling Rate

- **After every task commit:** Run `cargo test --package cli-manager proxy::handler proxy::mod proxy::state`
- **After every plan wave:** Run `cargo test --package cli-manager`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 15-01-01 | 01 | 1 | ROUT-01 | integration | `cargo test --package cli-manager proxy::mod::tests::test_openai_compatible_non_streaming_roundtrip` | ❌ W0 | ⬜ pending |
| 15-01-02 | 01 | 1 | ROUT-01 | integration | `cargo test --package cli-manager proxy::mod::tests::test_openai_compatible_streaming_roundtrip` | ❌ W0 | ⬜ pending |
| 15-01-03 | 01 | 1 | ROUT-02 | integration | `cargo test --package cli-manager proxy::mod::tests` | ✅ 已有 | ⬜ pending |
| 15-01-04 | 01 | 1 | MODL-03 | unit | `cargo test --package cli-manager proxy::handler::tests::test_model_exact_match_wins_over_default` | ❌ W0 | ⬜ pending |
| 15-01-05 | 01 | 1 | MODL-03 | unit | `cargo test --package cli-manager proxy::handler::tests::test_model_fallback_to_upstream_model` | ❌ W0 | ⬜ pending |
| 15-01-06 | 01 | 1 | MODL-03 | unit | `cargo test --package cli-manager proxy::handler::tests::test_model_preserved_when_no_mapping` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/proxy/handler.rs` — 新增 `apply_upstream_model_mapping()` + 3 个 unit tests
- [ ] `src-tauri/src/proxy/state.rs` — `UpstreamTarget` 新增 2 个字段 + 更新辅助函数
- [ ] `src-tauri/src/proxy/mod.rs` — 新增 2 个 OpenAiChatCompletions 集成测试
- [ ] `src-tauri/src/commands/proxy.rs` — 更新 `build_upstream_target_from_provider()` + 构造点

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Claude Code 配置 OpenRouter 后正常显示输出 | ROUT-01 SC1 | 需要真实 Provider API key 和 Claude Code 运行环境 | 配置 OpenRouter Provider，发送请求，验证响应正常显示 |
| 流式 SSE 逐 token 显示无截断 | ROUT-01 SC4 | 流式显示需要 Claude Code UI 观察 | 流式请求验证逐 token 显示和工具调用解析 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
