---
phase: 27-log-pipeline
plan: "01"
subsystem: traffic-log-pipeline
tags: [sqlite, mpsc, tauri-events, log-infrastructure]
dependency_graph:
  requires: [26-01]
  provides: [LogEntry, TrafficLogPayload, insert_request_log, query_recent_logs, log_worker, get_recent_logs, ProxyState.log_tx, ProxyState.cli_id, UpstreamTarget.provider_name]
  affects: [proxy/state.rs, proxy/server.rs, proxy/mod.rs, proxy/handler.rs, commands/proxy.rs, commands/provider.rs, watcher/mod.rs, lib.rs]
tech_stack:
  added: [tokio::sync::mpsc, tauri::Emitter]
  patterns: [mpsc fire-and-forget, log_worker async consumer, TDD green-inline]
key_files:
  created:
    - src-tauri/src/traffic/log.rs
    - src-tauri/src/commands/traffic.rs
  modified:
    - src-tauri/src/traffic/mod.rs
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/proxy/state.rs
    - src-tauri/src/proxy/server.rs
    - src-tauri/src/proxy/mod.rs
    - src-tauri/src/proxy/handler.rs
    - src-tauri/src/commands/proxy.rs
    - src-tauri/src/commands/provider.rs
    - src-tauri/src/watcher/mod.rs
    - src-tauri/src/lib.rs
key_decisions:
  - "log_worker 使用 tauri::Manager trait（use tauri::{Emitter, Manager}）访问 try_state"
  - "ProxyState.log_tx 直接持有 Option<Sender<LogEntry>>（Sender 实现 Clone，无需 Arc<RwLock>）"
  - "build_upstream_target 的 provider_name 设为 \"unknown\"（底层调试 command，无 provider 上下文）"
  - "ProxyService.log_tx 使用 std::sync::RwLock（非 tokio），因为 start() 中只读取一次"
metrics:
  duration_minutes: 11
  completed_date: "2026-03-18"
  tasks_completed: 2
  files_modified: 10
  tests_passed: 427
---

# Phase 27 Plan 01: 日志写入管道基础设施 Summary

**一句话：** mpsc channel 日志写入管道全基础设施——LogEntry/TrafficLogPayload 结构、SQLite 写入/查询方法、log_worker 后台消费者、Tauri traffic-log 事件推送、UpstreamTarget+ProxyState 状态扩展、lib.rs channel 注入

## What Was Built

为 Phase 28 handler 埋点提供所有必需的基础设施：

1. **traffic/log.rs**（新建，~300 行）：
   - `LogEntry` 结构（18 个数据字段，不含 id）
   - `TrafficLogPayload` 结构（id + event_type + 19 列，derive Serialize，event_type serde rename 为 "type"）
   - `TrafficLogPayload::from_entry(id, entry, event_type)` 构造方法
   - `TrafficDb::insert_request_log` — 18 字段 INSERT，返回 last_insert_rowid
   - `TrafficDb::query_recent_logs` — ORDER BY created_at DESC LIMIT ?，返回 Vec<TrafficLogPayload>
   - `log_worker` — mpsc::Receiver 消费者，写入 DB 后 emit "traffic-log" Tauri 事件
   - 6 个单元测试（in-memory DB）

2. **commands/traffic.rs**（新建）：
   - `get_recent_logs` Tauri command，limit 默认 100，上限 1000

3. **proxy/state.rs** 扩展：
   - `UpstreamTarget` 新增 `provider_name: String`
   - `ProxyState` 新增 `log_tx: Option<Sender<LogEntry>>` 和 `cli_id: String`
   - `ProxyState::new` 签名改为 `(client, cli_id, log_tx)`
   - 新增 `log_sender()` 和 `cli_id()` 访问方法

4. **proxy/server.rs** 扩展：
   - `ProxyServer::new` 签名改为 `(port, client, cli_id, log_tx)`

5. **proxy/mod.rs** 扩展：
   - `ProxyService` 新增 `log_tx: std::sync::RwLock<Option<Sender<LogEntry>>>`
   - 新增 `set_log_sender()` 方法
   - `start()` 传递 log_tx 给 ProxyServer::new

6. **lib.rs** 扩展：
   - setup 闭包中创建 mpsc channel(1024)，注入 sender 到 ProxyService，spawn log_worker
   - invoke_handler 注册 `get_recent_logs`

7. **构造点修复**：commands/proxy.rs、commands/provider.rs、watcher/mod.rs、proxy/handler.rs 中共约 20+ 处 UpstreamTarget 构造加 provider_name；约 10 处 ProxyServer::new 测试调用更新签名

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] log_worker 缺少 use tauri::Manager trait**
- **Found during:** Task 1 RED 阶段编译
- **Issue:** `app_handle.try_state::<>()` 调用需要 Manager trait 在作用域内
- **Fix:** `use tauri::{Emitter, Manager};`（单行修复）
- **Files modified:** src-tauri/src/traffic/log.rs
- **Commit:** a984295（修复后一并提交）

## Self-Check: PASSED

- src-tauri/src/traffic/log.rs: FOUND
- src-tauri/src/commands/traffic.rs: FOUND
- Commit a984295: FOUND
- Commit 752dc67: FOUND
