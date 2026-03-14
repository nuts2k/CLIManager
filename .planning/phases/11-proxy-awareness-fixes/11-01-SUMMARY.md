---
phase: 11-proxy-awareness-fixes
plan: 01
subsystem: proxy-awareness
status: complete
started: 2026-03-14
completed: 2026-03-14
requirements-completed: [LIVE-01, LIVE-03, UX-01]
tags: [bug-fix, proxy, tray, provider, documentation]
dependency_graph:
  requires: []
  provides: [tray-proxy-aware-switch, update-provider-skip-patch, ux-01-doc-sync]
  affects: [src-tauri/src/tray.rs, src-tauri/src/commands/provider.rs]
tech_stack:
  added: []
  patterns:
    - "TraySwitchMode 枚举 + determine_tray_switch_mode 纯函数（代理感知逻辑可测试化）"
    - "spawn_blocking → tauri::async_runtime::spawn（支持 async 代理函数调用）"
    - "_update_provider_in skip_patch 参数（调用层控制是否 patch CLI 配置）"
key_files:
  created: []
  modified:
    - src-tauri/src/tray.rs
    - src-tauri/src/commands/provider.rs
    - .planning/REQUIREMENTS.md
    - .planning/phases/10-live-switching-ui/10-02-SUMMARY.md
decisions:
  - "[11-01] tray.rs spawn_blocking 改为 tauri::async_runtime::spawn，以支持调用 async 代理感知函数"
  - "[11-01] _update_provider_in 增加 skip_patch 参数，代理模式下 update_provider 传 skip_patch=true"
  - "[11-01] determine_tray_switch_mode 提取为纯函数，与 AppHandle async 上下文解耦，便于单元测试"
metrics:
  duration_minutes: 4
  tasks_completed: 3
  files_modified: 4
  tests_added: 6
---

# Phase 11 Plan 01: 代理感知修复与文档同步 Summary

**一句话：** 修复托盘切换和 Provider 编辑路径的代理感知缺失，代理模式下不再覆盖 PROXY_MANAGED 占位值，并同步 UX-01 文档。

## 修复内容

### Bug 1 修复（LIVE-01）：tray.rs handle_provider_click 代理感知

**问题：** `handle_provider_click` 使用 `spawn_blocking` 直接调用 `_set_active_provider_in`（不感知代理），代理模式下会将 PROXY_MANAGED 占位值覆盖为真实 API key，导致 CLI 绕过本地代理。

**修复：**
- 提取 `determine_tray_switch_mode(settings, cli_id) -> TraySwitchMode` 纯函数
- 将 `spawn_blocking` 改为 `tauri::async_runtime::spawn`（支持调用 async 函数）
- 代理模式（`ProxyMode`）下调用 `_set_active_provider_in_proxy_mode`：仅更新 `active_providers` + 代理上游，不 patch CLI 配置文件
- 直连模式（`DirectMode`）下维持现有 `_set_active_provider_in` 行为
- 新增 4 个单元测试覆盖 `determine_tray_switch_mode` 所有分支

### Bug 2 修复（LIVE-03）：_update_provider_in 代理模式下跳过 patch

**问题：** `_update_provider_in` 内部无条件调用 `patch_provider_for_cli`，代理模式下编辑活跃 Provider 时短暂用真实凭据覆盖 PROXY_MANAGED。

**修复：**
- `_update_provider_in` 新增 `skip_patch: bool` 参数
- 条件改为 `if is_active && !skip_patch { ... patch ... }`
- `update_provider` 命令提前读取 settings，用 `find_proxy_cli_ids_for_provider` 判断是否代理模式，传入对应的 `skip_patch` 值
- 代理上游更新逻辑复用已计算的 `proxy_cli_ids`（移除重复读取 settings）
- 更新现有 4 个调用处（传 `false` 保持现有行为）
- 新增 2 个单元测试：`skip_patch=true` 不调用 adapter、`skip_patch=false` 正常调用 adapter

### 文档同步（UX-01）

- `REQUIREMENTS.md`: UX-01 复选框 `[ ]` → `[x]`（端口占用检测已在 Phase 10 plan 02 中实现）
- `10-02-SUMMARY.md` frontmatter: 新增 `requirements-completed: [UX-01]`

## Commits

| Task | Commit | 描述 |
|------|--------|------|
| 1 | `ec2e6d6` | fix(11-01): tray handle_provider_click 代理感知修复 |
| 2 | `08bccfe` | fix(11-01): _update_provider_in 代理模式下跳过 patch_provider_for_cli |
| 3 | `b0904b4` | docs(11-01): 文档同步 — UX-01 标记完成，10-02-SUMMARY 添加 requirements-completed |

## Deviations from Plan

无 - 计划严格按预定方向执行。

Task 1 和 Task 2 均为同步编写实现+测试（非严格 RED-GREEN 分离），因为枚举/纯函数的实现和测试在同一 commit 中天然是绑定的，分开提交无实质意义。

## Self-Check: PASSED

- [x] `src-tauri/src/tray.rs` — 存在，包含 `determine_tray_switch_mode` 和 `TraySwitchMode`
- [x] `src-tauri/src/commands/provider.rs` — 存在，包含 `skip_patch` 参数
- [x] `.planning/REQUIREMENTS.md` — UX-01 标记为 `[x]`
- [x] `.planning/phases/10-live-switching-ui/10-02-SUMMARY.md` — 包含 `requirements-completed: [UX-01]`
- [x] Commit `ec2e6d6` 存在（Task 1 tray.rs 修复）
- [x] Commit `08bccfe` 存在（Task 2 provider.rs 修复）
- [x] Commit `b0904b4` 存在（Task 3 文档同步）
- [x] 全量测试：221 个测试全部通过（`cargo test --lib -- --test-threads=1`）
