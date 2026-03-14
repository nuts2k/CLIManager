# Phase 12: 全栈实现 - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

完成所有代码和配置变更：CI 流水线、签名、updater 集成、发版脚本、Gatekeeper 文档。包含 4 个 plan（Wave 1 密钥基础 → Wave 2 三路并行：CI/CD、Updater、发版脚本）。

</domain>

<decisions>
## Implementation Decisions

### 更新提示 UI 交互
- 模态对话框形式（复用已有 Dialog 组件），不用 Toast
- 启动时自动检查更新，发现新版本弹出对话框
- 弹窗内容简洁：只显示当前版本号和新版本号，不展示更新日志
- 用户点「稍后提醒」后本次启动不再弹窗，下次启动再提醒
- 下载过程中显示进度条，完成后可立即安装并重启

### 关于页面
- 在设置（Settings）中新增「关于」页面
- 显示当前版本号
- 打开页面时自动检查更新，有新版本则显示更新按钮
- 提供按钮链接到 GitHub Releases 页面，用户可查看详细更新信息

### CHANGELOG 与 Release Notes
- CHANGELOG 自动生成：发版技能内置脚本解析 git log，按 Conventional Commits 规范分类
- 语言：中文（CHANGELOG 和 Release Notes 均为中文）
- Gatekeeper 安装指引：折叠段落放在 Release Notes 底部（`<details>` 标签）
- 零外部依赖（不用 git-cliff 或 conventional-changelog，未来 UPD-05 再考虑）

### 发版技能工作流
- 命令名：`/ship`（项目局部技能，非全局 `/release`）
- 一键执行：`/ship patch|minor|major`，不逐步确认
- 流程：bump Cargo.toml → 生成 CHANGELOG → commit → tag → push
- 仅 bump Cargo.toml（tauri.conf.json 省略 version 字段，REL-01 已决定）

### 初始版本号
- 首次 CI 发布版本：v0.2.0（从当前 0.1.0 升级，体现代理模式+自动更新等新功能）
- 后续版本升级不严格遵循 semver，随意升级即可（0.x 阶段灵活处理）

### Claude's Discretion
- 进度条具体实现方式（tauri-plugin-updater 的 download 事件 vs 自行计算）
- 关于页面的布局和样式细节
- CHANGELOG 分类模板的具体格式
- Release Notes 模板的具体排版
- `/ship` 技能的错误处理和回滚逻辑

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Dialog` 组件 (src/components/ui/dialog.tsx)：更新弹窗复用
- `Sonner` (src/components/ui/sonner.tsx)：操作反馈 toast
- `Button` 组件 (src/components/ui/button.tsx)：按钮样式
- `Badge` 组件 (src/components/ui/badge.tsx)：版本号标签
- `SettingsPage` (src/components/settings/SettingsPage.tsx)：关于页面的容器
- `Tabs` 组件 (src/components/ui/tabs.tsx)：设置页面已有 tab 布局，关于页面可加为新 tab

### Established Patterns
- Tauri invoke 调用：前端通过 `@tauri-apps/api` 调用 Rust 命令
- i18next 国际化：所有 UI 文本通过 i18n key 引用
- shadcn/ui + Tailwind CSS v4：UI 组件库和样式体系
- React 19 + Vite 7：前端构建链

### Integration Points
- `tauri.conf.json`：需要添加 updater 配置（endpoints、pubkey）、移除 version 字段
- `Cargo.toml`：作为唯一版本来源，`/ship` 技能读写此文件
- `package.json`：已有 `@tauri-apps/api`，需添加 `@tauri-apps/plugin-updater` 和 `@tauri-apps/plugin-process`
- `Cargo.toml` 依赖：需添加 `tauri-plugin-updater` 和 `tauri-plugin-process`
- `.github/workflows/`：尚不存在，需新建 CI 工作流文件
- `SettingsPage.tsx`：关于页面集成入口

</code_context>

<specifics>
## Specific Ideas

- 关于页面打开时自动检查更新，有新版本时显示更新按钮 + Release 页面链接按钮
- `/ship` 命令风格简洁，输出每步状态用 checkmark 标记（类似 preview 中展示的格式）

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 12-full-stack-impl*
*Context gathered: 2026-03-14*
