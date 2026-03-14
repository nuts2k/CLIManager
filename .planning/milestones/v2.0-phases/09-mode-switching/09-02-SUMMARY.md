---
phase: 09-mode-switching
plan: 02
subsystem: proxy
tags: [lifecycle, crash-recovery, exit-cleanup, auto-restore, takeover]

# Dependency graph
requires:
  - phase: 09-01
    provides: "ProxySettings + ProxyTakeover 持久化 + _proxy_enable_in/_proxy_disable_in 内部函数 + get_adapter_for_cli"
  - phase: 08-02
    provides: "ProxyService.stop_all() 停止所有代理实例"
provides:
  - "cleanup_on_exit_sync: 正常退出时同步还原所有被接管 CLI 配置并清除 takeover"
  - "recover_on_startup: 崩溃后重启时检测遗留 takeover 并静默还原 CLI 配置"
  - "restore_proxy_state: 根据持久化开关状态自动重新开启代理（UX-02）"
  - "lib.rs ExitRequested 退出清理 hook: 先同步还原配置 -> 再异步停止代理"
  - "lib.rs setup 崩溃恢复 + 自动恢复代理状态"
affects: [10-realtime-ui]

# Tech tracking
tech-stack:
  added: []
  patterns: [tauri::async_runtime::block_on 在 RunEvent 同步回调中执行异步代码, tauri::async_runtime::spawn 在 setup 闭包中异步恢复代理状态]

key-files:
  created: []
  modified:
    - src-tauri/src/commands/proxy.rs
    - src-tauri/src/lib.rs

key-decisions:
  - "cleanup_on_exit_sync 为同步函数，确保在 RunEvent::ExitRequested 回调中直接执行（adapter.patch 已是同步）"
  - "代理停止（stop_all）通过 tauri::async_runtime::block_on 在退出回调中异步执行"
  - "恢复顺序：先崩溃恢复（同步还原遗留 takeover）→ 再 spawn 异步恢复代理状态"
  - "restore_proxy_state 通过 tauri::async_runtime::spawn 异步执行，不阻塞 setup 闭包"

patterns-established:
  - "退出清理模式: 同步还原配置 → 清除 takeover → 异步 stop_all（确保 CLI 不指向已关闭的 localhost）"
  - "崩溃恢复模式: 启动时检测 takeover 标志 → best-effort 还原 → 清除标志"

requirements-completed: [MODE-05, MODE-06, UX-02]

# Metrics
duration: 5min
completed: 2026-03-13
---

# Phase 9 Plan 2: 生命周期管理 Summary

**退出清理 (cleanup_on_exit_sync) + 崩溃恢复 (recover_on_startup) + 启动自动恢复 (restore_proxy_state) 三大生命周期逻辑实现**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-13T14:55:40Z
- **Completed:** 2026-03-13T15:01:00Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments
- 正常退出时同步还原所有被接管 CLI 配置为真实凭据，清除 takeover 标志，然后异步停止代理
- 崩溃后重启时自动检测遗留 proxy_takeover.cli_ids，静默还原 CLI 配置（仅日志记录）
- 启动时根据持久化 proxy.global_enabled 和 cli_enabled 自动重新开启代理（UX-02）
- 恢复顺序正确：先崩溃恢复（还原）-> 再按持久化状态重新开启
- 190 个测试全部通过，cargo build 无错误

## Task Commits

Each task was committed atomically:

1. **Task 1: 退出清理 + 崩溃恢复 + 启动自动恢复逻辑** - `aa16905` (feat)

## Files Created/Modified
- `src-tauri/src/commands/proxy.rs` - 新增 cleanup_on_exit_sync / recover_on_startup / restore_proxy_state 三个公共函数 + 4 个单元测试
- `src-tauri/src/lib.rs` - setup 闭包添加崩溃恢复和自动恢复逻辑，app.run() 闭包添加 ExitRequested 退出清理

## Decisions Made
- cleanup_on_exit_sync 为同步函数，adapter.patch() 本身是同步的，适合在 RunEvent 回调中直接执行
- 代理停止（stop_all）通过 tauri::async_runtime::block_on 在退出回调中异步执行（需要等待完成再退出）
- restore_proxy_state 通过 tauri::async_runtime::spawn 异步执行，不阻塞应用启动
- 恢复顺序：先崩溃恢复 -> 再自动恢复代理，确保不会在被接管状态下启动代理

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - 无需外部服务配置。

## Next Phase Readiness
- Phase 9 两个 plan 全部完成：模式切换后端核心 + 生命周期管理
- Phase 10 可开始实现前端 UI 集成，通过 Tauri invoke 调用已有的代理命令
- 代理生命周期完整覆盖：正常退出、异常崩溃、重启恢复三个场景

---
*Phase: 09-mode-switching*
*Completed: 2026-03-13*

## Self-Check: PASSED

All 2 modified files verified. Task commit (aa16905) confirmed in git log. SUMMARY.md exists.
