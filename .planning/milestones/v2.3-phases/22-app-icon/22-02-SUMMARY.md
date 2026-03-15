---
phase: 22-app-icon
plan: 02
subsystem: ui
tags: [tauri, tray-icon, template-image, pillow, svg, branding]

# Dependency graph
requires:
  - 22-01 (app-icon.svg 源文件，设计参考)
provides:
  - src-tauri/icons/tray/tray-icon-template.png（44x44 黑白轮廓 template 图标，lib.rs include_bytes! 直接使用）
  - src-tauri/icons/tray/tray-icon-template@2x.png（44x44 Retina 备用）
  - src-tauri/icons/tray/tray-icon.svg（托盘图标 SVG 源文件）
affects:
  - src-tauri/src/lib.rs（通过 include_bytes! 在编译时嵌入托盘图标，路径无需改动）

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Python Pillow 手工绘制透明背景黑色图案 PNG（qlmanage 对透明 SVG 渲染失败时的备选方案）
    - macOS template 图标规范：透明背景 + 纯黑色图案，系统自动处理浅色/深色菜单栏适配

key-files:
  created:
    - src-tauri/icons/tray/tray-icon.svg
    - src-tauri/icons/tray/tray-icon-template@2x.png
    - src-tauri/icons-backup/tray-icon-template.png
  modified:
    - src-tauri/icons/tray/tray-icon-template.png

key-decisions:
  - "托盘图标用 Python Pillow 手工绘制，而非 qlmanage 渲染 SVG（qlmanage 将透明背景填充为白色，不符合 template 要求）"
  - "tray-icon-template.png 和 @2x 均保持 44x44（与现有 lib.rs include_bytes! 兼容，无需改代码）"
  - "中心设计采用外圆环（r=7/r=5 叠加镂空）+ 内实心圆（r=3.5）的双层枢纽视觉，连接线宽 2px"
  - "auto_advance=true，Task 2 视觉验证检查点自动通过"

patterns-established:
  - "macOS template 图标生成：Python Pillow 创建 RGBA 图像（透明背景），用 draw.line/ellipse 绘制纯黑元素"

requirements-completed: [ICON-02]

# Metrics
duration: 2min
completed: 2026-03-15
---

# Phase 22 Plan 02: 托盘图标生成 Summary

**从应用图标派生黑白轮廓 template 托盘图标，用 Python Pillow 绘制透明背景 44x44 PNG，保持 lib.rs include_bytes! 路径不变**

## Performance

- **Duration:** 约 2 min
- **Completed:** 2026-03-15T09:51:35Z
- **Tasks:** 2（Task 1 执行，Task 2 auto-approved）
- **Files modified:** 4

## Accomplishments

- 创建托盘图标 SVG 源文件（44x44，透明背景，黑色枢纽连接轮廓设计）
- 用 Python Pillow 生成两个 template PNG 文件（透明背景 + 纯黑色图案）
- tray-icon-template.png（44x44）：lib.rs include_bytes! 直接嵌入，菜单栏实际显示为 22pt
- tray-icon-template@2x.png（44x44）：Retina 备用文件，供未来运行时加载升级使用
- 旧托盘图标备份至 src-tauri/icons-backup/
- lib.rs 代码无需任何修改

## Task Commits

每个任务独立提交：

1. **Task 1: 生成黑白轮廓托盘 template 图标** - `85495eb` (feat)
2. **Task 2: 视觉验证（auto-approved）** - 无需提交

## Files Created/Modified

- `src-tauri/icons/tray/tray-icon.svg` - 托盘图标 SVG 源文件（44x44，透明背景）
- `src-tauri/icons/tray/tray-icon-template.png` - 44x44 黑白 template 图标（lib.rs 使用）
- `src-tauri/icons/tray/tray-icon-template@2x.png` - 44x44 Retina 备用 template 图标
- `src-tauri/icons-backup/tray-icon-template.png` - 旧托盘图标备份

## Decisions Made

- qlmanage 将透明 SVG 渲染为白底图，不符合 macOS template 要求，改用 Python Pillow 从坐标手工绘制
- 两个文件均保持 44x44（而非严格的 22x22 @1x），与现有文件和 include_bytes! 引用完全兼容
- 枢纽设计参照应用图标结构（中心环形 + 内圆 + 5个外围节点 + 连接线），简化为轮廓风格

## Deviations from Plan

### 自动处理的问题

**1. [Rule 1 - Bug] qlmanage 透明背景渲染失败**
- **发现于：** Task 1 步骤 3
- **问题：** qlmanage -t 将 SVG 透明背景渲染为白色（四角像素 RGBA=(255,255,255,255)），不满足 template 图标要求
- **修复：** 改用 Python Pillow 手工绘制（计划中的备选方案），直接在 RGBA 图像上绘制透明背景黑色图案
- **修改文件：** 仅影响生成方式，输出文件不变
- **Commit：** 85495eb

## Issues Encountered

- qlmanage 无法正确处理 SVG 透明背景（已按计划备选方案切换为 Python Pillow）
- 其余执行顺畅，工具链完全可用

## User Setup Required

无 — 纯设计资产替换，无外部服务配置。

## Next Phase Readiness

- Phase 22 所有图标工作完成（Plan 01 应用图标 + Plan 02 托盘图标）
- 下次 `pnpm tauri dev` 或 `pnpm tauri build` 将自动使用新托盘图标
- lib.rs 无需修改，include_bytes! 路径保持 "../icons/tray/tray-icon-template.png"

## Self-Check: PASSED

- tray-icon-template.png: FOUND
- tray-icon-template@2x.png: FOUND
- tray-icon.svg: FOUND
- icons-backup/tray-icon-template.png: FOUND
- commit 85495eb: FOUND

---
*Phase: 22-app-icon*
*Completed: 2026-03-15*
