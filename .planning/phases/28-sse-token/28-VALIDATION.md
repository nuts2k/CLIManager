---
phase: 28
slug: sse-token
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 28 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust 内置 `#[cfg(test)]` + tokio::test |
| **Config file** | Cargo.toml（无独立测试配置文件） |
| **Quick run command** | `cargo test -p cli-manager-lib 2>&1` |
| **Full suite command** | `cargo test --workspace 2>&1` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p cli-manager-lib 2>&1`
- **After every plan wave:** Run `cargo test --workspace 2>&1`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 28-01-01 | 01 | 1 | COLLECT-03 | unit | `cargo test -p cli-manager-lib traffic::log::tests 2>&1` | ✅ | ⬜ pending |
| 28-01-02 | 01 | 1 | COLLECT-03 | unit | `cargo test -p cli-manager-lib -- streaming_token 2>&1` | ❌ W0 | ⬜ pending |
| 28-01-03 | 01 | 1 | COLLECT-03 | unit | `cargo test -p cli-manager-lib stream::tests 2>&1` | ✅ | ⬜ pending |
| 28-01-04 | 01 | 1 | COLLECT-03 | unit | `cargo test -p cli-manager-lib responses_stream::tests 2>&1` | ✅ | ⬜ pending |
| 28-01-05 | 01 | 1 | COLLECT-03 | unit | `cargo test -p cli-manager-lib handler::tests 2>&1` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `stream.rs` — 新增测试：`test_token_callback_on_eof`（验证 oneshot tx 在 finish_reason 时被调用，携带正确 token 值）
- [ ] `responses_stream.rs` — 新增测试：`test_token_callback_on_response_completed`（验证 response.completed 时 tx 被调用）
- [ ] `handler.rs` / 新模块 — `create_anthropic_reverse_model_stream` 扩展后的 token 解析测试
- [ ] `traffic/log.rs` — 新增测试：`test_update_streaming_log`（INSERT 后 UPDATE，验证 token 字段正确填充）

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| 流式请求端到端 token 写入 | COLLECT-03 | 需要真实 API Provider | 通过代理发送流式请求，检查 SQLite 中 token 字段非 null |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
