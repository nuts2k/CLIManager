---
phase: 28-sse-token
plan: 02
subsystem: proxy/traffic
tags: [streaming, oneshot, token-extraction, background-task, ttfb]
dependency_graph:
  requires: [28-01 (StreamTokenData, update_streaming_log, app_handle 链路)]
  provides: [流式 SSE 请求完整 INSERT→spawn→await EOF→UPDATE→emit 链路]
  affects:
    - src-tauri/src/proxy/translate/stream.rs
    - src-tauri/src/proxy/translate/responses_stream.rs
    - src-tauri/src/proxy/handler.rs
tech_stack:
  added: []
  patterns:
    - tokio::sync::oneshot（流结束信号传递）
    - tokio::spawn 后台 task（异步 UPDATE DB）
    - Option<Sender> + take() 模式（oneshot 单次发送）
key_files:
  created: []
  modified:
    - src-tauri/src/proxy/translate/stream.rs
    - src-tauri/src/proxy/translate/responses_stream.rs
    - src-tauri/src/proxy/handler.rs
decisions:
  - "[28-02] 流式请求跳过 log_worker channel（方案 C）：直接 INSERT 可同步获取 rowid，无需设计复杂的 id 回传机制"
  - "[28-02] TTFB 在 send().await 后立即采样：流式和非流式统一，最准确反映上游响应首字节时间"
  - "[28-02] Option<Sender> + take() 模式：规避 oneshot::Sender 不实现 Copy 导致的借用冲突，确保只 send 一次"
metrics:
  duration: "约 15 分钟"
  completed_date: "2026-03-18"
  tasks_completed: 2
  files_modified: 3
  tests_added: 2
  total_tests_passing: 438
---

# Phase 28 Plan 02: SSE Token Oneshot 回传与 Handler 后台 Task Summary

**一句话：** 为三种协议流函数添加 oneshot token 回传参数，在 handler 流式分支实现 INSERT→spawn→await EOF→UPDATE DB→emit 完整链路，TTFB 统一采样。

## 完成任务

### Task 1: 三个流函数增加 oneshot token 回传

**stream.rs — create_anthropic_sse_stream：**
- 新增 `token_tx: tokio::sync::oneshot::Sender<StreamTokenData>` 参数
- 在 finish_reason chunk 处提取 prompt_tokens / completion_tokens / cache_read / cache_creation / stop_reason（原始 OpenAI 格式）
- stream! 宏末尾通过 `token_tx.take().send(data)` 回传
- 新增测试 `test_token_callback_on_finish_reason`：含 usage 的 finish_reason chunk → 验证回传数据正确

**responses_stream.rs — create_responses_anthropic_sse_stream：**
- 新增 `token_tx` 参数
- 在 `response.completed` 分支，`break 'outer` 前提取 token（独立 `.and_then(|v| v.as_i64())` 提取，而非复用 SSE 输出用的 `unwrap_or(0)`）
- 新增测试 `test_token_callback_on_response_completed`：含 `input_token_details.cached_tokens` → 验证 cache_read_tokens 回传

**handler.rs — create_anthropic_reverse_model_stream：**
- 新增 `token_tx` 参数
- 在逐行处理中检测 Anthropic `message_delta` 事件，解析 usage 字段（input/output/cache_creation/cache_read tokens + stop_reason）
- 所有现有测试中的 dummy 调用均已更新为传入 `(token_tx, _rx)`

### Task 2: handler 流式分支接入 oneshot + 后台 task UPDATE/emit

**TTFB 采样：**
- 在 `send().await` 成功返回后立即采样：`let ttfb_ms = start_time.elapsed().as_millis() as i64;`
- `LogEntry.ttfb_ms` 统一改为 `Some(ttfb_ms)`（流式和非流式均填充）

**三个流式分支改造（OpenAiChatCompletions / OpenAiResponses / AnthropicPassthrough）：**
```rust
let (tx, rx) = tokio::sync::oneshot::channel::<StreamTokenData>();
streaming_token_rx = Some(rx);
Body::from_stream(create_xxx_stream(..., tx))
```

**日志采集方案 C：**
- 流式请求跳过 log_worker channel，直接 `db.insert_request_log(&entry)` 获取 rowid
- emit `type="new"`（token 为 None 的初始状态）
- 非流式请求保持原有 `tx.try_send(entry)` 逻辑

**后台 task：**
```rust
if let (Some(rx), Some(row_id)) = (streaming_token_rx, log_row_id) {
    tokio::spawn(async move {
        match rx.await {
            Ok(token_data) => {
                db.update_streaming_log(row_id, &token_data, Some(ttfb), Some(duration_ms))?;
                handle.emit("traffic-log", &update_payload)?;
            }
            Err(_) => log::debug!("stream 被客户端中断");
        }
    });
}
```

## 验证结果

```
test result: ok. 438 passed; 0 failed; 0 ignored; 0 measured
cargo check: 0 errors（仅已有 warnings）
```

新增 2 个测试：
- `test_token_callback_on_finish_reason`（stream.rs）
- `test_token_callback_on_response_completed`（responses_stream.rs）

## Deviations from Plan

None - plan executed exactly as written.

## Decisions Made

1. **流式请求跳过 log_worker 采用方案 C**（PLAN 列出了 A/B/C，选 C）：直接 INSERT 可同步获取 rowid，无需 channel 设计变更；app_handle = None（测试环境）时自动跳过，不影响现有测试
2. **TTFB 采样位置**：在 `upstream_resp` 返回后立即采样，这是上游首个字节到达的准确时刻，非流式和流式均适用
3. **Option<Sender> + take() 模式**：避免 `token_tx` 被 move 进 async_stream! 宏后的二次使用问题；Rust 借用规则要求 oneshot::Sender 只能 send 一次，用 Option 封装最清晰

## Commits

- `2e38bb3` feat(28-02): 三个流函数增加 oneshot token 回传参数
- `10e43f0` feat(28-02): handler 流式分支接入 oneshot + 后台 task UPDATE/emit

## Self-Check: PASSED
