---
phase: 27-log-pipeline
plan: "02"
subsystem: proxy-handler-logging
tags: [proxy, handler, logging, token-extraction, mpsc]
dependency_graph:
  requires: [27-01]
  provides: [extract_anthropic_tokens, extract_openai_chat_tokens, extract_responses_tokens, extract_tokens_from_response, protocol_type_str, send_error_log, proxy_handler日志埋点]
  affects: [proxy/handler.rs]
tech_stack:
  added: []
  patterns: [token-extraction-before-move, fire-and-forget try_send, mutable-log-vars-per-branch]
key_files:
  created: []
  modified:
    - src-tauri/src/proxy/handler.rs
key_decisions:
  - "token 提取在 resp_value move 之前完成（直接调用协议专用函数，非通用 extract_tokens_from_response）"
  - "method.clone() 传给 reqwest 构建器，保留 method 所有权用于错误日志发送"
  - "send_error_log 仅覆盖 UpstreamUnreachable 分支（NoUpstreamConfigured 无 upstream 信息，不记录）"
  - "流式和 Passthrough 分支 token 字段全部 None，duration_ms 也为 None（Phase 28 在 stream EOF 后处理）"
metrics:
  duration_minutes: 4
  completed_date: "2026-03-18"
  tasks_completed: 2
  files_modified: 1
  tests_passed: 433
---

# Phase 27 Plan 02: proxy_handler 日志采集埋点 Summary

**一句话：** proxy_handler 完整日志采集——三协议 token 提取函数、各分支 LogEntry 构建、try_send 非阻塞发送、UpstreamUnreachable 错误日志、6 个单元测试覆盖全部提取场景

## What Was Built

为完成 Phase 27 端到端日志采集链路，在 proxy_handler 中嵌入日志采集逻辑：

1. **token 提取辅助函数**（handler.rs 新增，handler 前部）：
   - `extract_anthropic_tokens` — 从 Anthropic 原生格式提取 input/output/cache_creation/cache_read/stop_reason
   - `extract_openai_chat_tokens` — 从 OpenAI Chat Completions 原始格式提取 prompt_tokens/completion_tokens/cached_tokens/finish_reason
   - `extract_responses_tokens` — 从 OpenAI Responses API 原始格式提取 input_tokens/output_tokens（cache 字段 Phase 27 留 null）
   - `extract_tokens_from_response` — 统一入口，根据 ResponseTranslationMode 分发
   - `protocol_type_str` — 将 ProtocolType 转换为小写字符串（"anthropic"/"open_ai_chat_completions"/"open_ai_responses"）
   - `send_error_log` — UpstreamUnreachable 错误分支发送错误日志的辅助函数

2. **proxy_handler 埋点**：
   - 函数顶部记录 `Instant::now()` 和 `SystemTime` epoch ms
   - 在 body_bytes move 之前提取 `log_request_model`（适用于所有分支）
   - 步骤 H（UpstreamUnreachable）错误 map_err 中调用 `send_error_log`
   - 步骤 J 各非流式分支：在 resp_value move 前提取 token → 赋值 log_* 变量
   - 步骤 J 结束后统一构建 LogEntry → `try_send`（失败仅 log::warn）
   - 流式分支（SSE）：is_streaming=1，token 全 None，duration_ms=None（Phase 28 处理）
   - Passthrough/4xx-5xx 分支：token 全 None

3. **6 个单元测试**（#[cfg(test)] mod tests 新增）：
   - `test_extract_anthropic_tokens_full` — 完整响应 5 字段验证
   - `test_extract_anthropic_tokens_no_usage` — 无 usage 返回全 None
   - `test_extract_openai_chat_tokens_full` — 含 prompt_tokens_details + finish_reason
   - `test_extract_openai_chat_tokens_no_cache` — 无 prompt_tokens_details 时 cache_read=None
   - `test_extract_responses_tokens` — input/output 正确，cache/stop_reason 为 None
   - `test_protocol_type_str` — 三种协议类型小写字符串验证

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] method move 后无法在错误闭包中借用**
- **Found during:** Task 1 编译
- **Issue:** `state.http_client.request(method, ...)` 将 `method` move 到 reqwest builder，导致 `send_error_log` 闭包中无法再借用 `&method`
- **Fix:** 改为 `state.http_client.request(method.clone(), ...)`，保留原 `method` 所有权
- **Files modified:** src-tauri/src/proxy/handler.rs
- **Commit:** af7d1ac（修复内含）

## Self-Check: PASSED

- src-tauri/src/proxy/handler.rs: FOUND
- Commit af7d1ac (feat 27-02): FOUND
- Commit 46e34c8 (test 27-02): FOUND
- 433 tests passed (全套)
