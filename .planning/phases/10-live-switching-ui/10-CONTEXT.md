# Phase 10: 实时切换与 UI 集成 - Context

**Gathered:** 2026-03-13
**Status:** Ready for planning

<domain>
## Phase Boundary

代理模式下切换 Provider 对 CLI 完全透明且即时生效，用户通过前端 UI 控制所有代理相关设置。Phase 10 负责：前端开关 UI（设置页全局开关 + Tab 内 CLI 独立开关）、iCloud 同步和 CRUD 操作自动联动代理上游、端口占用错误处理、代理状态指示。不包含代理引擎本身（Phase 8）或模式切换后端命令（Phase 9）。

</domain>

<decisions>
## Implementation Decisions

### 开关 UI 布局
- 全局代理开关放设置页，位于「语言」section 和「测试配置」section 之间，作为独立的「代理模式」section
- section 包含 shadcn Switch 组件 + 简短说明文字（"CLI 请求经本地代理转发，切换 Provider 无需重启 CLI"）
- 全局开关旁不显示额外状态概要，开关本身即表达状态
- CLI 独立代理开关放在 Tab 内容区的标题栏同行，位于左侧「代理」标签后，与右侧「新建」按钮同行
- 独立开关使用纯开关（无状态文字），开关本身表达开/关状态
- 当全局开关关闭或该 CLI 无活跃 Provider 时，独立开关置灰不可点 + tooltip 提示原因（"请先在设置中开启代理"或"请先设置活跃 Provider"）

### Provider 变更联动代理
- LIVE-02（iCloud 同步变更）：在 Rust 侧 watcher 的 process_events 中自动联动，检测到活跃 Provider 文件变更且处于代理模式时，直接调用 update_upstream() 更新代理上游，前端无需参与
- LIVE-03（前端 CRUD 操作）：在 Rust 命令层（update_provider/delete_provider）自动判断，如果被修改/删除的是当前活跃 Provider 且处于代理模式，自动调用 update_upstream()
- 删除活跃 Provider 时（代理模式下）：自动关闭该 CLI 的代理模式（调用 proxy_disable），避免代理无上游目标

### 端口占用错误处理
- 检测时机：在 axum bind 失败时检测错误类型是否为 AddrInUse，而非启动前预检测
- 错误展示：使用已有 sonner toast 弹出错误提示，开关回滚为关闭状态（复用 Phase 9 已决定的失败回滚机制）
- 错误消息：简短 + 建议，如"端口 15800 已被占用，无法启动 Claude Code 代理。请关闭占用该端口的程序后重试。"

### 代理运行状态指示
- Tab 标签上显示状态点：绿色小圆点表示该 CLI 代理已开启，未开启时无圆点
- 前端通过事件驱动获取代理状态：监听已有的 proxy-mode-changed 事件，触发时重新查询 proxy_get_mode_status 刷新 UI
- 设置页全局开关旁不显示额外状态

### Claude's Discretion
- Switch 组件的具体样式和尺寸
- tooltip 的具体实现方式（shadcn Tooltip 或原生 title 属性）
- 绿色状态点的具体 CSS 样式（大小、位置、是否带动画）
- proxy-mode-changed 事件监听的 hook 设计（独立 hook 或扩展 useSyncListener）
- i18n 翻译文案的具体措辞

</decisions>

<specifics>
## Specific Ideas

- 设置页全局开关的布局参考现有语言选择 section 的样式，保持一致性
- Tab 内开关的位置参考当前 TabsList + "新建"按钮的同行布局，开关放左侧与按钮对称
- 端口占用错误消息需要 i18n 支持（中英双语）
- proxy-mode-changed 事件已由 Phase 9 的 proxy_enable/proxy_disable 命令 emit

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `SettingsPage.tsx`: 已有语言/测试配置/关于/导入四个 section，新增「代理模式」section 遵循相同模式
- `ProviderTabs.tsx`: 已有 TabsList + "新建"按钮同行布局，CLI 独立开关插入同一行
- `sonner` toast 组件: 已用于各种操作反馈，端口占用和开关切换通知复用
- `proxy_enable/proxy_disable/proxy_set_global/proxy_get_mode_status`: Phase 9 已实现的四个 Tauri 命令
- `proxy-mode-changed` 事件: Phase 9 已在 proxy_enable/proxy_disable 中 emit
- `update_upstream()`: Phase 8 已实现，支持运行时动态切换上游
- `useSyncListener` hook: 已监听 providers-changed 事件，可参考模式新建代理状态监听 hook
- `useSettings` hook: 已有 settings 状态管理，代理开关状态可扩展此 hook
- shadcn Switch 组件: UI 库已包含，可直接使用
- `ProxyModeStatus/CliProxyStatus` 类型: Phase 9 已定义，proxy_get_mode_status 返回此结构

### Established Patterns
- Tauri 命令层（commands/）+ 业务层分离
- 前端通过 `src/lib/tauri.ts` 封装 invoke 调用
- 事件驱动刷新：Rust emit 事件 → 前端 listen → 重新查询刷新 UI
- `_in/_to` 内部函数变体用于测试隔离
- Fire-and-forget 非关键操作（如 refreshTrayMenu().catch(() => {})）

### Integration Points
- `SettingsPage.tsx`: 新增代理模式 section（位于语言和测试配置之间）
- `ProviderTabs.tsx`: 新增 CLI 独立开关行（TabsList 和 ProviderList 之间）
- `src/lib/tauri.ts`: 新增 proxy_enable/disable/set_global/get_mode_status 的 TS 封装函数
- `src/types/settings.ts`: LocalSettings 类型需扩展 proxy 相关字段（proxy.global_enabled, proxy.cli_enabled 等）
- `watcher/mod.rs` 的 `process_events`: 新增代理模式下的 update_upstream 调用
- `commands/provider.rs` 的 `update_provider/delete_provider`: 新增代理模式感知逻辑

</code_context>

<deferred>
## Deferred Ideas

None — 讨论保持在 Phase 10 范围内

</deferred>

---

*Phase: 10-live-switching-ui*
*Context gathered: 2026-03-13*
