---
phase: 24-claude-settings-overlay-end-to-end
plan: 04
subsystem: ui
tags: [tauri, rust, overlay, apply, watcher, i18n, toast, startup-cache]

requires:
  - phase: 24-01
    provides: overlay 存储（get_icloud_config_dir / read/write_claude_settings_overlay）
  - phase: 24-02
    provides: overlay UI 编辑器与 set_claude_settings_overlay 命令
  - phase: 24-03
    provides: patch_claude_json 深度合并引擎 + ClaudeAdapter overlay 集成 + strip_protected_fields

provides:
  - apply_claude_settings_overlay() 函数（有/无活跃 Provider 两条路径）
  - ClaudeOverlayStartupNotificationQueue state（startup 通知缓存队列）
  - take_claude_overlay_startup_notifications tauri 命令（take 语义）
  - watcher 扩展：同时监听 config_dir 下 overlay 文件变更并触发 apply
  - useSyncListener 统一处理三类 overlay 通知（success/failed/protected_fields_ignored）
  - startup 缓存回放链路：setup apply -> 队列 -> take -> toast replay

affects:
  - phase-25-test-coverage
  - useSyncListener（overlay 通知已接入）

tech-stack:
  added: []
  patterns:
    - "startup 通知缓存策略：setup 早于 WebView listener，通知写队列而非直接 emit，前端挂载后 take/replay"
    - "apply 分流：save/watcher 实时 emit，startup 写缓存队列"
    - "watcher 多目录：同一 debouncer 监听 providers_dir + config_dir，按目录来源分流处理"
    - "ClaudeAdapter.patch() 用于有活跃 Provider 场景，保证保护字段由 Provider 决定"
    - "无活跃 Provider 场景：apply_overlay_without_provider() strip 保护字段后直接合并"

key-files:
  created: []
  modified:
    - src-tauri/src/commands/claude_settings.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/watcher/mod.rs
    - src/lib/tauri.ts
    - src/hooks/useSyncListener.ts
    - src/i18n/locales/zh.json
    - src/i18n/locales/en.json

key-decisions:
  - "startup apply 结果不依赖实时 emit，改为写入 ClaudeOverlayStartupNotificationQueue，前端 take/replay，彻底解决 setup 时序问题"
  - "take 语义（取出即清空）：effect 因语言切换重跑时不会重复弹 startup toast"
  - "set 命令保存后立即 apply（强一致 COVL-09）：apply 失败则 set 整体返回 Err"
  - "watcher 扩展：新增 config_dir watch，providers_dir == config_dir 时按文件名区分 overlay vs provider"
  - "overlay apply 通知统一模型 ClaudeOverlayApplyNotification：kind/source/settings_path/error/paths"

requirements-completed:
  - COVL-09
  - COVL-10
  - COVL-11
  - COVL-12
  - COVL-08

duration: 25min
completed: 2026-03-17
---

# Phase 24 Plan 04: Overlay apply 端到端 Summary

**overlay apply 完整链路：保存即 apply（强一致）+ startup 通知缓存队列 + iCloud watcher 自动触发 + 前端 useSyncListener 统一 toast/i18n 反馈**

## Performance

- **Duration:** 25 min
- **Started:** 2026-03-17T00:30:00Z
- **Completed:** 2026-03-17T00:55:00Z
- **Tasks:** 2 auto + 1 auto-approved checkpoint
- **Files modified:** 7

## Accomplishments

- 后端 apply 命令完整实现：有活跃 Provider 走 ClaudeAdapter.patch()，无活跃 Provider 走 strip+merge 直写
- startup 通知缓存策略落地：setup 阶段 spawn async apply，结果写入 ClaudeOverlayStartupNotificationQueue，前端挂载后 take/replay，彻底解决时序丢失
- watcher 扩展：同时监听 config_dir（overlay 文件）和 providers_dir（provider 文件），按目录/文件名分流处理
- 前端 useSyncListener 统一处理 success/failed/protected_fields_ignored 三类实时通知 + startup 缓存回放
- i18n 全覆盖：zh/en 均新增 claudeOverlayApply 文案 key（source 标签/成功/失败/保护字段提示）

## Task Commits

1. **Task 1: 后端 apply 命令 + startup 通知缓存策略** - `8c8eaec` (feat)
2. **Task 2: watcher overlay 变更监听 + 前端统一通知与 startup 回放** - `f5edee8` (feat)
3. **Task 3: 人工验证（auto-approved）** - checkpoint:human-verify，auto_advance=true

## Files Created/Modified

- `/Users/kelin/Workspace/CLIManager/src-tauri/src/commands/claude_settings.rs` - apply 命令、通知模型、startup 队列、take 命令
- `/Users/kelin/Workspace/CLIManager/src-tauri/src/lib.rs` - 注册 startup 队列 state，setup 阶段 spawn startup apply，注册新命令
- `/Users/kelin/Workspace/CLIManager/src-tauri/src/watcher/mod.rs` - 扩展 watcher 监听 config_dir，overlay 变更触发 apply
- `/Users/kelin/Workspace/CLIManager/src/lib/tauri.ts` - ClaudeOverlayApplyNotification 类型 + takeClaudeOverlayStartupNotifications 封装
- `/Users/kelin/Workspace/CLIManager/src/hooks/useSyncListener.ts` - 统一处理三类通知 + startup take/replay
- `/Users/kelin/Workspace/CLIManager/src/i18n/locales/zh.json` - claudeOverlayApply 中文文案
- `/Users/kelin/Workspace/CLIManager/src/i18n/locales/en.json` - claudeOverlayApply 英文文案

## Decisions Made

- startup apply 结果不依赖实时 emit：彻底规避 setup 早于 WebView listener 的时序问题
- take 语义（取出即清空）：useSyncListener effect 因语言切换重跑时不会重复弹 startup toast
- apply 失败时 set_claude_settings_overlay 整体返回 Err（强一致 COVL-09）
- watcher 多目录监听：同一 debouncer 实例同时 watch providers_dir 和 config_dir
- providers_dir == config_dir 降级场景：按文件名区分 overlay vs provider，避免把 overlay 文件当 provider 处理

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

编译过程中修复了三处小问题（均在 Task 1 同一 commit 内修复）：
1. 缺少 `use crate::adapter::CliAdapter` trait import（E0599）
2. `e.to_string()` 类型推断问题（E0282）：加 `.clone()` 解决
3. `try_state` 返回 `Option` 不是 `Result`（E0308）：改 `if let Ok` 为 `if let Some`

均属编译期发现的小问题，不影响设计意图。

## Next Phase Readiness

- Phase 25（测试覆盖）可立即开始
- apply 链路端到端已接线：保存/启动/watcher 三条路径均已实现
- 人工验证 checkpoint（Task 3）由 auto_advance=true 自动通过，实际功能验证可在开发环境手动执行

---
*Phase: 24-claude-settings-overlay-end-to-end*
*Completed: 2026-03-17*
