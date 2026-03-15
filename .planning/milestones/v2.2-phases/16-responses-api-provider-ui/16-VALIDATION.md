---
phase: 16
slug: responses-api-provider-ui
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-14
---

# Phase 16 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` / `#[tokio::test]` + cargo test |
| **Config file** | `src-tauri/Cargo.toml`（已配置） |
| **Quick run command** | `cd src-tauri && cargo test -p cli-manager-lib -- translate::responses 2>&1` |
| **Full suite command** | `cd src-tauri && cargo test -p cli-manager-lib 2>&1` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test -p cli-manager-lib -- translate::responses 2>&1 | tail -5`
- **After every plan wave:** Run `cd src-tauri && cargo test -p cli-manager-lib 2>&1`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 16-01-01 | 01 | 1 | RAPI-02 | unit | `cargo test translate::responses_request` | ❌ W0 | ⬜ pending |
| 16-01-02 | 01 | 1 | RAPI-02 | unit | `cargo test responses_request::test_max_tokens_mapping` | ❌ W0 | ⬜ pending |
| 16-01-03 | 01 | 1 | RAPI-02 | unit | `cargo test responses_request::test_system_to_instructions` | ❌ W0 | ⬜ pending |
| 16-01-04 | 01 | 1 | RAPI-02 | unit | `cargo test responses_request::test_tools_format` | ❌ W0 | ⬜ pending |
| 16-01-05 | 01 | 1 | RAPI-03 | unit | `cargo test responses_response::test_text_response` | ❌ W0 | ⬜ pending |
| 16-01-06 | 01 | 1 | RAPI-03 | unit | `cargo test responses_response::test_function_call_response` | ❌ W0 | ⬜ pending |
| 16-01-07 | 01 | 1 | RAPI-03 | unit | `cargo test responses_response::test_usage_passthrough` | ❌ W0 | ⬜ pending |
| 16-01-08 | 01 | 1 | RAPI-04 | unit (async) | `cargo test responses_stream::test_text_stream` | ❌ W0 | ⬜ pending |
| 16-01-09 | 01 | 1 | RAPI-04 | unit (async) | `cargo test responses_stream::test_function_call_stream` | ❌ W0 | ⬜ pending |
| 16-01-10 | 01 | 1 | RAPI-01 | unit | `cargo test handler::tests` | ✅ | ⬜ pending |
| 16-02-01 | 02 | 1 | MODL-04 | manual | 手动 UI 验证 | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/proxy/translate/responses_request.rs` — stubs for RAPI-02（纯函数单元测试）
- [ ] `src-tauri/src/proxy/translate/responses_response.rs` — stubs for RAPI-03（纯函数单元测试）
- [ ] `src-tauri/src/proxy/translate/responses_stream.rs` — stubs for RAPI-04（tokio::test 异步测试）

*Existing test infrastructure (cargo test) covers framework needs.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ProviderDialog 模型映射 UI 交互 | MODL-04 | 前端 UI 交互无法自动化 | 创建/编辑 Provider，切换协议类型验证条件显示，增删映射对后保存 |
| 协议类型下拉三选项 | RAPI-01 | 前端 UI | 打开 Provider 编辑，确认三个选项显示正确 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
