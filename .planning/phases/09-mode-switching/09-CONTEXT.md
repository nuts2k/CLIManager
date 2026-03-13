# Phase 9: 模式切换与持久化 - Context

**Gathered:** 2026-03-13
**Status:** Ready for planning

<domain>
## Phase Boundary

用户可在直连模式和代理模式间安全切换，切换时 CLI 配置自动联动（patch 指向 localhost 或还原为真实凭据），应用退出或崩溃后状态正确恢复。Phase 9 只做模式切换逻辑、配置联动、崩溃恢复和设置持久化，不包含代理引擎本身（Phase 8）或前端 UI 控件（Phase 10）。

</domain>

<decisions>
## Implementation Decisions

### 开关联动逻辑
- 全局总开关放设置页，CLI 独立开关放主窗口对应 Tab 内
- 总开关关闭时 CLI 独立开关状态保留但被禁用（置灰），重新开启总开关时自动恢复之前的 CLI 独立状态（飞行模式体验）
- 总开关切换时完整联动 CLI 配置：关闭时先还原 CLI 配置再停代理服务，开启时先 patch CLI 配置再启代理服务
- 首次使用时全局总开关和所有 CLI 独立开关默认关闭
- 总开关无启用前提（可随时打开），CLI 独立开关需要该 CLI 有活跃 Provider 才能启用（否则置灰+提示）

### 异常恢复与退出清理
- 崩溃后重启时检测 takeover 标志，静默还原 CLI 配置，仅日志记录（不弹通知）
- takeover 标志存储在 local.json 中作为新字段（如 `proxy_takeover: { cli_ids: ["claude", "codex"] }`），记录哪些 CLI 被接管
- 正常退出（Cmd+Q）时静默还原所有已代理 CLI 配置并停止代理服务，不弹确认窗口
- 正常退出时先还原 CLI 配置为直连状态、清除 takeover 标志，再停代理服务
- 下次启动时根据持久化的开关状态自动重新开启代理（UX-02），等同于再次执行开启流程

### 模式切换反馈与前提
- 开启/关闭代理时弹出简短 toast 通知（如"Claude Code 代理已开启"），使用已有 sonner 组件
- 开启代理失败时（如端口占用或 patch 失败）弹 toast 错误提示，开关回滚为关闭状态，不留半成品状态

### 代理模式下 Provider 切换
- 代理模式下切换 active Provider 时，CLI 配置不动（仍指向 localhost + PROXY_MANAGED），只调用 ProxyService.update_upstream() 更新代理内存中的上游目标
- 关闭代理模式时，CLI 配置还原为当前活跃 Provider 的真实凭据（执行一次 adapter.patch(active_provider)）
- 改造现有 set_active_provider 命令，加入代理模式判断：代理模式下跳过 adapter.patch()，改为调用 update_upstream()

### Claude's Discretion
- takeover 标志的具体字段命名和结构
- 代理启停的具体操作编排顺序（先 patch 后启动代理 or 先启动后 patch 的微调）
- 开关状态持久化的具体字段设计（在 local.json 中的结构）
- 退出清理的超时策略
- toast 通知的具体文案和展示时长

</decisions>

<specifics>
## Specific Ideas

- Tauri app.exit() 调用 std::process::exit() 不触发 drop，退出时需在 RunEvent::ExitRequested 或 RunEvent::Exit 中显式执行还原逻辑（Phase 8 研究结论）
- stop_all() 已在 Phase 8 预留但未暴露 Tauri 命令，供 Phase 9 退出清理内部使用
- 现有 adapter.patch() 和 adapter.clear() 可直接复用：开启代理时 patch 为 localhost + PROXY_MANAGED，关闭时 patch 为活跃 Provider 真实凭据

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `CliAdapter` trait（adapter/mod.rs）：`patch()` 和 `clear()` 方法可直接复用，开启代理时 patch 为代理凭据，关闭时 patch 为真实凭据
- `ProxyService`（proxy/mod.rs）：`start()`, `stop()`, `stop_all()`, `update_upstream()` 已就绪
- `LocalSettings`（storage/local.rs）：现有设备本地存储模式，可扩展 proxy 相关字段（开关状态、takeover 标志）
- `sonner` toast 组件（components/ui/sonner.tsx）：可直接用于模式切换通知
- `SettingsPage`（components/settings/SettingsPage.tsx）：总开关放置位置
- `restore_from_backup()`（adapter/mod.rs）：备份还原功能已实现

### Established Patterns
- Tauri 托管状态模式（`app.manage(T)` + `State<T>`）：ProxyService 已遵循此模式
- 命令层 + 业务层分离：commands/proxy.rs + proxy 模块
- 事件发射（`providers-changed`）：代理状态变更可复用事件机制通知前端
- `_in/_to` 内部函数变体：测试隔离模式

### Integration Points
- `lib.rs` 的 `app.run()` 闭包：需 hook RunEvent 做退出清理
- `commands/provider.rs` 的 `set_active_provider`：需加入代理模式判断逻辑
- `LocalSettings`：需扩展 proxy 开关状态和 takeover 标志字段
- `lib.rs` 的 `setup` 闭包：需加入启动时崩溃恢复检查和代理自动恢复逻辑

</code_context>

<deferred>
## Deferred Ideas

None — 讨论保持在 Phase 9 范围内

</deferred>

---

*Phase: 09-mode-switching*
*Context gathered: 2026-03-13*
