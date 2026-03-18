---
phase: 28-sse-token
plan: 01
subsystem: traffic/proxy
tags: [infrastructure, streaming, token-extraction, app-handle]
dependency_graph:
  requires: [Phase 27 - 日志采集基础设施（log_tx、log_worker）]
  provides: [StreamTokenData 结构体, update_streaming_log 方法, ProxyState.app_handle]
  affects: [src-tauri/src/traffic/log.rs, src-tauri/src/proxy/state.rs, src-tauri/src/proxy/server.rs, src-tauri/src/proxy/mod.rs, src-tauri/src/lib.rs]
tech_stack:
  added: []
  patterns: [TDD（RED-GREEN）, app_handle 注入链路, Option<tauri::AppHandle>]
key_files:
  created: []
  modified:
    - src-tauri/src/traffic/log.rs
    - src-tauri/src/proxy/state.rs
    - src-tauri/src/proxy/server.rs
    - src-tauri/src/proxy/mod.rs
    - src-tauri/src/lib.rs
decisions:
  - "[28-01] StreamTokenData 直接覆盖 7 个字段（非条件更新）：因流式请求初次 INSERT 时这些字段全为 None，UPDATE 时统一设置，无需 CASE WHEN 条件逻辑"
  - "[28-01] app_handle 用 Option<tauri::AppHandle> 而非强制 AppHandle：非测试环境下注入，测试中传 None 保持向后兼容"
  - "[28-01] ProxyService.app_handle 用 std::sync::RwLock（与 log_tx 对称），start() 调用时读取 clone"
metrics:
  duration: "约 5 分钟"
  completed_date: "2026-03-18"
  tasks_completed: 2
  files_modified: 5
  tests_added: 3
  total_tests_passing: 436
---

# Phase 28 Plan 01: SSE Token 提取基础设施 Summary

**一句话：** 为流式 SSE token 提取建立基础设施——StreamTokenData 结构体 + TrafficDb.update_streaming_log() 方法 + ProxyState/ProxyServer/ProxyService 的 app_handle 传递链路（含 lib.rs 注入点）。

## 完成任务

### Task 1: StreamTokenData + update_streaming_log 方法（TDD）

按 TDD（RED-GREEN）流程实现：

**RED：** 先写 3 个失败测试（StreamTokenData 和 update_streaming_log 尚不存在导致编译失败）。

**GREEN：** 在 `src-tauri/src/traffic/log.rs` 中新增：
- `StreamTokenData` 结构体（5 个 Option 字段：input/output/cache_creation/cache_read tokens + stop_reason）
- `TrafficDb::update_streaming_log()` 方法：按 rowid 更新 7 个字段（input/output/cache_creation/cache_read tokens、stop_reason、ttfb_ms、duration_ms）

3 个单元测试全部通过：
- `test_update_streaming_log_fills_tokens`：INSERT 流式记录后 UPDATE 填充，query 验证字段正确
- `test_update_streaming_log_nonexistent_id`：UPDATE 不存在 id 返回 Ok（0 行受影响，不 panic）
- `test_update_streaming_log_partial_none`：UPDATE 部分字段为 None，写入 SQL NULL 正确

### Task 2: ProxyState/ProxyServer/ProxyService app_handle 传递链路

**ProxyState（state.rs）：**
- 新增 `app_handle: Option<tauri::AppHandle>` 字段
- `ProxyState::new()` 增加 `app_handle` 参数
- 新增 `app_handle()` getter

**ProxyServer（server.rs）：**
- `ProxyServer::new()` 增加 `app_handle` 参数，传递给 `ProxyState::new()`
- 所有测试调用更新（末尾加 `None`）

**ProxyService（mod.rs）：**
- 新增 `app_handle: std::sync::RwLock<Option<tauri::AppHandle>>` 字段
- 新增 `set_app_handle()` 注入方法
- `ProxyService::start()` 中读取 `app_handle.clone()` 并传给 `ProxyServer::new()`

**lib.rs setup：**
- 注入：`proxy_service.set_app_handle(app.handle().clone())`

## 验证结果

```
test result: ok. 436 passed; 0 failed; 0 ignored; 0 measured
cargo check: 0 errors
```

## Deviations from Plan

None - plan executed exactly as written.

## Decisions Made

1. `StreamTokenData` 字段设计选择直接覆盖（而非条件更新 `CASE WHEN`）：因流式请求初次 INSERT 时 7 个目标字段全为 None，stream EOF 后的 UPDATE 是首次写入，无需保护已有值
2. `app_handle: Option<tauri::AppHandle>` 而非强制 `AppHandle`：保持测试中传 `None` 的向后兼容性，ProxyService 注入前 start() 也能工作（app_handle 为 None 时下游 Phase 28-02 检查 is_some() 后使用）
3. `ProxyService.app_handle` 使用 `std::sync::RwLock` 与 `log_tx` 完全对称，保持代码风格一致

## Commits

- `51db0e6` test(28-01): 添加 update_streaming_log 失败测试（RED）
- `0a46867` feat(28-01): 新增 StreamTokenData 结构体和 update_streaming_log 方法
- `111b496` feat(28-01): ProxyState/ProxyServer/ProxyService 增加 app_handle 传递链路

## Self-Check: PASSED
