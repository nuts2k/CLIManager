---
phase: 27
slug: log-pipeline
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 27 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in) |
| **Config file** | src-tauri/Cargo.toml (`[dev-dependencies]` contains `tempfile`) |
| **Quick run command** | `cd src-tauri && cargo test traffic` |
| **Full suite command** | `cd src-tauri && cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test traffic`
- **After every plan wave:** Run `cd src-tauri && cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 27-01-01 | 01 | 1 | STORE-03 | unit | `cargo test traffic::log::tests` | ❌ W0 | ⬜ pending |
| 27-01-02 | 01 | 1 | COLLECT-01 | unit | `cargo test traffic::log::tests::test_log_entry_fields` | ❌ W0 | ⬜ pending |
| 27-01-03 | 01 | 1 | COLLECT-02 | unit | `cargo test traffic::log::tests::test_token_extraction` | ❌ W0 | ⬜ pending |
| 27-01-04 | 01 | 1 | COLLECT-04 | unit | `cargo test traffic::log::tests::test_error_log` | ❌ W0 | ⬜ pending |
| 27-01-05 | 01 | 1 | LOG-01 | integration | `cargo test proxy::handler::tests::test_log_pipeline_e2e` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/traffic/log.rs` — LogEntry, TrafficLogPayload, insert_request_log, log_worker, unit tests (covers STORE-03, COLLECT-01, COLLECT-02, COLLECT-04)
- [ ] `src-tauri/src/commands/traffic.rs` — get_recent_logs command
- [ ] Existing tests with `UpstreamTarget` construction need `provider_name` field added (~15 locations, compilation fix scope)

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Frontend receives traffic-log event | LOG-01 | Requires running Tauri app with frontend | 1. Start app in proxy mode 2. Send a non-streaming request 3. Observe browser console for traffic-log event |
| No response latency increase | STORE-03 | Performance characteristic, not functional | 1. Measure response time without logging 2. Enable logging 3. Compare response times |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
