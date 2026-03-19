---
phase: 31-tech-debt-fix
plan: "02"
subsystem: traffic
tags: [rust, tauri, react, i18n, error-handling, safety]
dependency_graph:
  requires: []
  provides: [safe-db-access, frontend-db-error-state, wont-fix-comments]
  affects: [traffic-commands, traffic-hooks, traffic-page]
tech_stack:
  added: []
  patterns: [try_state-pattern, inline-error-banner, i18n-error-keys]
key_files:
  created: []
  modified:
    - src-tauri/src/commands/traffic.rs
    - src/hooks/useTrafficLogs.ts
    - src/hooks/useTrafficStats.ts
    - src/components/traffic/TrafficPage.tsx
    - src/i18n/locales/zh.json
    - src/i18n/locales/en.json
    - src/components/traffic/TrafficTrendChart.tsx
    - src-tauri/src/proxy/handler.rs
decisions:
  - "try_state 替代 State 直接注入：DB 未 manage 时返回 Err 字符串而非 panic（与 lib.rs rollup 定时任务同一模式）"
  - "dbError 内联 banner 而非 toast：DB 不可用是持续状态，不应使用自动消失的通知"
  - "WON'T FIX 项在代码中用注释文档化，防止未来开发者误以为是 bug"
metrics:
  duration: "4m 09s"
  completed_date: "2026-03-19"
  tasks_completed: 2
  files_modified: 8
---

# Phase 31 Plan 02: DB 安全访问 + 前端错误状态 + WON'T FIX 注释 Summary

**一句话总结：** 将 3 个 Tauri traffic command 从 State 直接注入改为 try_state 安全访问（消除 panic 风险），在前端 hook 暴露 dbError 状态并在 TrafficPage 渲染内联警告 banner，同时为 3 项 WON'T FIX 技术债务添加设计意图注释。

## 完成的任务

### Task 1: 后端 try_state 安全访问 + 前端 dbError 状态 + 内联警告

**提交：** `1f0dc8a`

**修改：**
- `src-tauri/src/commands/traffic.rs`：3 个命令（get_recent_logs、get_provider_stats、get_time_trend）参数从 `tauri::State<'_, TrafficDb>` 改为 `tauri::AppHandle`，函数体内使用 `try_state()` 安全获取 DB 引用，DB 未初始化时返回 `Err("数据库不可用...")`
- `src/hooks/useTrafficLogs.ts`：新增 `dbError: string | null` 状态，catch 块中 `setDbError(String(err))`，返回类型新增 dbError 字段
- `src/hooks/useTrafficStats.ts`：同上模式，catch 块中设置 dbError 而非仅 console.error
- `src/components/traffic/TrafficPage.tsx`：解构 useTrafficLogs 时新增 dbError，在 TabsContent logs 中 TrafficStatsBar 之前添加条件渲染内联警告 banner
- `src/i18n/locales/zh.json`：新增 `traffic.dbErrorTitle`、`traffic.dbErrorDesc`
- `src/i18n/locales/en.json`：同上英文版本

### Task 2: WON'T FIX 项添加设计意图注释

**提交：** `175bb57`

**修改：**
- `src/components/traffic/TrafficTrendChart.tsx`：文件顶部 JSDoc 块注释说明 STAT-03/STAT-04 合并实现设计决策（item 2）
- `src-tauri/src/proxy/handler.rs`：NoUpstreamConfigured 处添加注释说明无 provider_name 故不记录日志（item 5）；流式 INSERT 处添加补充注释说明 mpsc fire-and-forget 无法返回 rowid（item 6）

## 验证结果

- `cargo build`：编译通过，无错误
- `cargo test --lib traffic`：24 个通过，1 个失败（`test_query_time_trend_7d` — 预存在的时区边界问题，与本次修改无关，已确认在修改前同样失败）
- `npx tsc --noEmit`：零错误
- `grep "try_state" src-tauri/src/commands/traffic.rs`：3 处匹配
- `grep "dbError" hooks + TrafficPage`：10 处匹配
- `grep "有意设计" handler.rs`：1 处匹配

## 偏差记录

### 预存在的测试失败（超出修复范围）

`traffic::rollup::tests::test_query_time_trend_7d` 在本次修改前已失败（通过 git stash 验证），属于时区边界问题（"今天"的日志在 UTC 边界时可能被归到"昨天"）。按偏差规则，此为超出当前任务范围的预存问题，已记录但未修复。

### rollup.rs 未提交的预存改动

`src-tauri/src/traffic/rollup.rs` 存在 git stash 前就有的未提交改动（新增 `total_cache_creation_tokens` 字段），与本次计划无关，未纳入本次提交。

## Self-Check: PASSED

- 所有修改文件已确认存在
- 两个任务提交（1f0dc8a, 175bb57）均已确认存在
- cargo build 编译通过
- npx tsc --noEmit 零错误
