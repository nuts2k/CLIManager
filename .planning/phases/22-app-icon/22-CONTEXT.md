# Phase 22: 应用图标 - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

全新设计应用图标并从中派生托盘图标，替换 cc-switch 遗留图标。覆盖 src-tauri/icons/ 全套尺寸、托盘图标、public/icon.png 及 tauri.conf.json 配置。不涉及 UI 功能变更，纯设计资产替换。

</domain>

<decisions>
## Implementation Decisions

### 设计风格
- 扁平现代风格，干净的形状 + 纯色填充，与 macOS 原生应用图标风格对齐
- macOS squircle 圆角方形外轮廓（macOS 会自动裁剪为标准 squircle）
- 深色/黑色背景底色，与应用暗色主题一脉相承
- 品牌橙色 #F97316 单色作为主元素色，不使用渐变或多色

### 主题元素
- 核心概念：连接/桥梁 — 中心枢纽向外连接多个端点，象征 CLIManager 统一管理多个 CLI/Provider
- 中心元素：圆形枢纽节点（代表 CLIManager）
- 外围元素：多个小圆点节点（代表各 CLI/Provider），节点分布不均匀（非对称），各节点到圆心的距离也有变化，营造有机自然的连接感
- 中心与外围之间用线条连接

### 托盘图标
- 保留中心枢纽 + 连接线 + 外围节点的简化黑白轮廓
- 线条中等粗细，在 macOS 菜单栏 22px 高度下清晰可辨
- 纯黑 template 图标（macOS 自动处理浅色/深色菜单栏适配）
- 提供双尺寸：22x22 (@1x) 和 44x44 (@2x)
- 沿用现有文件名 tray-icon-template.png（lib.rs 中 include_bytes! 已引用，无需改代码）
- @2x 版本新增文件 tray-icon-template@2x.png

### 制作方式
- SVG 代码生成应用图标和托盘图标的源文件
- 使用 macOS 内置 sips 命令行工具将 SVG/PNG 转换为各尺寸
- iconutil 生成 .icns 文件

### 文件同步
- src-tauri/icons/ — 全套 Tauri 应用图标（32/128/128@2x/icns/ico + Windows Square 系列）
- src-tauri/icons/tray/ — 托盘 template 图标（@1x + @2x）
- public/icon.png — Header 和关于页使用的图标
- tauri.conf.json — 检查并确认图标路径配置正确

### 旧图标备份
- 替换前将原有图标文件移入 src-tauri/icons-backup/ 目录保留
- 备份目录不影响正式构建，需要时可找回原图标

### Claude's Discretion
- 外围节点的具体数量（建议 4-6 个范围内调整）
- 节点分布的具体角度和距离比例
- 连接线的具体粗细和样式（直线 vs 微弧线）
- 中心圆和外围圆的具体尺寸比例
- 深色背景的具体色值（纯黑 vs 深灰）
- SVG 路径的具体实现细节
- ICO 文件内包含的尺寸组合

</decisions>

<specifics>
## Specific Ideas

- 外围节点不要太均匀分布，与圆心的距离也不要完全一致 — 有机自然感比几何对称更重要
- 深色底 + 橙色元素的组合与应用暗色主题品牌视觉一致（Phase 17 品牌色 #F97316、Phase 21 Header 双色标识）
- 中心枢纽概念直观传达 "统一管理多个 CLI" 的产品价值

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- src-tauri/icons/ — 现有 Tauri 标准图标集（32/128/128@2x/icns/ico + Windows Square 系列），需全部替换
- src-tauri/icons/tray/tray-icon-template.png — 现有托盘 template 图标（186 bytes），需替换
- public/icon.png — Phase 20 关于页 + Phase 21 Header Logo 使用的图标，需同步替换

### Established Patterns
- lib.rs:94 — `include_bytes!("../icons/tray/tray-icon-template.png")` 加载托盘图标，文件名不可变
- lib.rs:100 — TrayIconBuilder 使用 `Icon::Raw` 从字节加载图标
- tauri.conf.json — Tauri 应用图标配置指向 src-tauri/icons/ 目录

### Integration Points
- Header.tsx — 引用 /icon.png 显示应用 Logo
- AboutSection.tsx — 引用 /icon.png 显示关于页 Logo
- Tauri 构建系统 — 自动从 src-tauri/icons/ 读取图标生成 .app bundle
- CI/CD — GitHub Actions 构建使用 src-tauri/icons/ 中的图标

</code_context>

<deferred>
## Deferred Ideas

None -- 讨论未超出 Phase 22 范围

</deferred>

---

*Phase: 22-app-icon*
*Context gathered: 2026-03-15*
