---
phase: 12-full-stack-impl
plan: "03"
subsystem: ui
tags: [tauri, tauri-plugin-updater, tauri-plugin-process, react, i18n]

# 依赖图
requires:
  - phase: 12-01
    provides: tauri-plugin-updater 和 tauri-plugin-process 插件注册、Cargo.toml 版本统一

provides:
  - useUpdater hook：check/download/install/dismiss 状态机（idle/checking/available/downloading/ready/error）
  - UpdateDialog 组件：模态对话框 + 进度条（确定态 + 不确定态）
  - AboutSection 组件：版本号显示 + 自动检查更新 + GitHub Releases 链接
  - AppShell 启动时自动检查更新并弹出 UpdateDialog
  - SettingsPage 关于区域替换为 AboutSection，支持手动检查和安装更新

affects: [13-release-pipeline]

# 技术追踪
tech-stack:
  added: []
  patterns:
    - "动态 import tauri 插件（避免开发模式报错）：import { check } from '@tauri-apps/plugin-updater'"
    - "双 useUpdater 实例模式：AppShell（启动检查）+ SettingsPage（手动检查）互不干扰"
    - "UpdateStatus 状态机：idle→checking→available→downloading→ready/error"
    - "progress=-1 表示不确定进度（Started 无 contentLength 时）"

key-files:
  created:
    - src/components/updater/useUpdater.ts
    - src/components/updater/UpdateDialog.tsx
    - src/components/settings/AboutSection.tsx
  modified:
    - src/components/layout/AppShell.tsx
    - src/components/settings/SettingsPage.tsx
    - src/i18n/locales/zh.json
    - src/i18n/locales/en.json

key-decisions:
  - "模态对话框形式（非 Toast）：用户决策，UpdateDialog 复用项目已有 Dialog 组件"
  - "双 useUpdater 实例：AppShell 启动检查与 SettingsPage 手动检查独立，避免状态互扰"
  - "动态 import 插件：规避开发模式下 check() 抛异常，静默忽略所有更新检查失败"
  - "progress=-1 表示不确定进度态：Started 事件 contentLength 可能为 undefined"
  - "版本号只显示，不展示更新日志（用户决策）"

patterns-established:
  - "Tauri 插件动态导入模式：const { check } = await import('@tauri-apps/plugin-updater')"
  - "AboutSection 挂载时自动触发检查更新（useEffect 空依赖数组）"

requirements-completed: [UPD-01, UPD-02, UPD-03, UPD-04]

# 指标
duration: 20min
completed: 2026-03-14
---

# Phase 12 Plan 03: 自动更新 UI Summary

**tauri-plugin-updater 自定义 UI：useUpdater 状态机 hook + UpdateDialog 进度条对话框 + AboutSection 版本/检查更新组件，集成到 AppShell（启动自动检查）和 SettingsPage（手动检查）**

## Performance

- **Duration:** 约 20 min
- **Started:** 2026-03-14T08:12:00Z
- **Completed:** 2026-03-14T08:32:00Z
- **Tasks:** 3（含 1 个 checkpoint:human-verify，auto-advance 自动批准）
- **Files modified:** 7

## Accomplishments

- useUpdater hook 实现完整状态机（idle/checking/available/downloading/ready/error），支持 check/download/install/dismiss
- UpdateDialog 模态对话框支持 4 种状态展示：版本号确认 + 进度条（确定/不确定态）+ 安装完成 + 错误
- AboutSection 关于区域组件：打开自动检查更新、显示版本号、"更新到 vX.X.X"按钮、GitHub Releases 链接
- AppShell 集成：启动时 bootstrap 末尾调用 checkForUpdate()，status=available 时自动弹 UpdateDialog
- SettingsPage 集成：硬编码 `0.1.0` 关于区域替换为 AboutSection，支持手动检查和安装
- 中英文 i18n updater 命名空间完整（14 个键值）
- pnpm build 编译通过

## Task Commits

每个任务原子提交：

1. **Task 1: useUpdater hook + UpdateDialog + AboutSection** - `6fec46d` (feat)
2. **Task 2: 集成到 AppShell 和 SettingsPage** - `b16eaa2` (feat)
3. **Task 3: pnpm build 验证 + Bug 修复** - `3828e1e` (fix)

## Files Created/Modified

- `src/components/updater/useUpdater.ts` - 更新检查状态机 hook，check/download/install/dismiss
- `src/components/updater/UpdateDialog.tsx` - 更新对话框 UI，4 种状态 + 进度条
- `src/components/settings/AboutSection.tsx` - 关于区域组件，版本号 + 检查更新 + Releases 链接
- `src/components/layout/AppShell.tsx` - 添加 useUpdater/UpdateDialog，启动时检查更新
- `src/components/settings/SettingsPage.tsx` - 关于区域替换为 AboutSection，独立 updater 实例
- `src/i18n/locales/zh.json` - 添加 updater 命名空间（14 键）
- `src/i18n/locales/en.json` - 添加 updater 命名空间（14 键）

## Decisions Made

- 复用项目已有 Dialog 组件（模态对话框形式），不使用 Toast
- 双 useUpdater 实例：AppShell（全局启动检查）和 SettingsPage（手动触发）各自独立
- 动态 import tauri 插件，规避开发模式下模块加载异常
- progress=-1 标识 contentLength 未知的不确定下载进度态，UI 显示 animate-pulse 动画

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修复 AboutSection TypeScript TS2367 错误**
- **Found during:** Task 3（pnpm build 验证）
- **Issue:** 在 `(updateStatus === "idle" || updateStatus === "error")` 条件块内，`disabled={updateStatus === "checking"}` 比较永远为 false，TypeScript 报 TS2367 不可达类型比较
- **Fix:** 删除无效的 `disabled` prop（disabled 逻辑本已由条件渲染实现）
- **Files modified:** `src/components/settings/AboutSection.tsx`
- **Verification:** pnpm build 编译通过，无 TypeScript 错误
- **Committed in:** `3828e1e`（独立修复提交）

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** TypeScript 类型正确性修复，无功能影响，无范围扩展。

## Issues Encountered

无重大问题。TypeScript 类型检查发现一处不可达条件表达式，已自动修复。

## User Setup Required

无需额外配置。更新检查依赖 GitHub Releases（`tauri.conf.json` 中的 `endpoints` 配置已在 Phase 12-01 注册）。

## Next Phase Readiness

- 更新 UI 完整实现，等待 Phase 13 CI/CD 流水线发布第一个 GitHub Release
- 发布 v0.2.0 后，latest.json 上线，更新检查将正常工作
- UpdateDialog 和 AboutSection 均已在无 Release 情况下静默处理（不阻断主流程）

---
*Phase: 12-full-stack-impl*
*Completed: 2026-03-14*
