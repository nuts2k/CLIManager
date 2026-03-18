---
phase: 26-sqlite
plan: "01"
subsystem: traffic
tags: [sqlite, rusqlite, schema-migration, tauri-state, wal]
dependency_graph:
  requires: []
  provides: [TrafficDb, init_traffic_db, get_traffic_db_path, open_traffic_db, MIGRATIONS]
  affects: [src-tauri/src/lib.rs]
tech_stack:
  added: [rusqlite@0.38, rusqlite_migration@2.4]
  patterns: [Mutex<Connection>, Tauri manage(), TDD, rusqlite_migration to_latest()]
key_files:
  created:
    - src-tauri/src/traffic/mod.rs
    - src-tauri/src/traffic/schema.rs
    - src-tauri/src/traffic/db.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs
decisions:
  - "使用 dirs::data_local_dir() 而非 Tauri AppHandle 获取路径，与现有 storage 模块风格一致，且不需要 AppHandle 引用"
  - "daily_rollups 实际列数为 13（id + provider_name + rollup_date + 10 聚合字段），UNIQUE 约束不计为列"
  - "WAL pragma 持久化在 DB 文件，busy_timeout 每次连接都需重新设置，configure_connection 同时设两者保证正确性"
  - "降级运行策略：init_traffic_db 返回 None 时不调用 manage()，Phase 27 通过 try_state::<TrafficDb>() 安全检查可用性"
metrics:
  duration: "4 分钟"
  completed_date: "2026-03-18"
  tasks_completed: 2
  files_created: 3
  files_modified: 2
  tests_added: 8
  tests_total: 420
---

# Phase 26 Plan 01: SQLite 基础设施初始化 Summary

**一句话：** 通过 rusqlite（bundled）+ rusqlite_migration，在 `~/Library/Application Support/com.climanager.app/traffic.db` 以 WAL 模式初始化 SQLite，建立 request_logs（19 列）和 daily_rollups（13 列）两张表及 3 个索引，TrafficDb 通过 Tauri `.manage()` 注入全局状态，失败时降级运行不 panic。

## Tasks Completed

| Task | 描述 | Commit | 文件 |
|------|------|--------|------|
| 1 | 创建 traffic 模块 + 依赖安装（TDD） | b2a4b6c | traffic/mod.rs, traffic/schema.rs, traffic/db.rs, Cargo.toml, lib.rs（mod声明） |
| 2 | 集成到 lib.rs setup 闭包 | b9b2cc0 | src-tauri/src/lib.rs |

## Success Criteria Verification

- [x] traffic.db 在正确路径初始化（`~/Library/Application Support/com.climanager.app/traffic.db`）
- [x] WAL 模式和 busy_timeout=5000 pragma 已配置
- [x] request_logs 表含 19 列 + 2 索引，daily_rollups 表含 13 列 + 1 索引 + UNIQUE 约束
- [x] schema 迁移幂等（重复调用 to_latest() 不报错）
- [x] DB 初始化失败时降级运行（不 panic，if let Some 模式）
- [x] 所有 8 个单元测试通过（traffic 模块专项）
- [x] cargo build 无 error，完整测试套件 420 tests 全部通过

## Architecture Decisions

### 路径获取策略
使用 `dirs::data_local_dir()` 手动拼接 `com.climanager.app/traffic.db`，而非 Tauri 的 `app.path().app_local_data_dir()`。原因：`dirs` crate 已在项目中（v5.0），不需要 AppHandle 引用，与现有 `storage/local.rs` 风格一致，macOS 结果相同（`~/Library/Application Support`）。

### 连接模型
`std::sync::Mutex<rusqlite::Connection>` 单连接，通过 `TrafficDb { conn: Mutex<Connection> }` 包裹后 `.manage()` 注入。选择标准 Mutex 而非 tokio Mutex：rusqlite 是同步 API，符合 Tauri 2 最佳实践，无 async 过头问题。

### 降级运行
`init_traffic_db()` 返回 `Option<TrafficDb>`，lib.rs 中使用 `if let Some` 模式。DB 不可用时不调用 `.manage()`，Phase 27+ 通过 `app_handle.try_state::<TrafficDb>()` 安全访问（避免 panic）。

## Deviations from Plan

### daily_rollups 列数说明

计划文档中描述"14 列（id + provider_name + rollup_date + 10 聚合字段 + UNIQUE 约束）"，但 UNIQUE 约束不算独立的列，实际 SQL 列数为 13 列（id, provider_name, rollup_date, 10 个聚合字段）。测试中使用 `assert_eq!(col_count, 13)` 反映实际情况，符合 CONTEXT.md 的字段设计决策。

除此之外，计划执行完全按照规划进行，无其他偏差。

## Self-Check: PASSED

所有关键文件存在，所有 commit 已验证：
- traffic/mod.rs — FOUND
- traffic/schema.rs — FOUND
- traffic/db.rs — FOUND
- 26-01-SUMMARY.md — FOUND
- commit b2a4b6c (Task 1) — FOUND
- commit b9b2cc0 (Task 2) — FOUND
