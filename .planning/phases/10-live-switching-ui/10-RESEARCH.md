# Phase 10: 实时切换与 UI 集成 - Research

**Researched:** 2026-03-14
**Status:** Complete

## 1. 需求分析

Phase 10 需实现 4 个需求：

| ID | 需求 | 核心挑战 |
|-----|------|---------|
| LIVE-01 | 代理模式下切换 Provider → 更新代理上游 | **已由 Phase 9 实现**：`set_active_provider` 命令已支持代理模式感知，调用 `proxy_service.update_upstream()` |
| LIVE-02 | iCloud 同步变更 → 自动更新代理上游 | watcher `process_events` 需新增代理上游更新逻辑 |
| LIVE-03 | 前端 CRUD → 自动更新代理上游 | `update_provider`/`delete_provider` 命令需新增代理模式感知 |
| UX-01 | 端口占用清晰错误提示 | 前端需区分 BindFailed 错误类型并显示友好 toast |

## 2. 已有基础设施（Phase 8/9 已实现）

### 2.1 后端 Tauri 命令（均已实现）
- `proxy_enable(cli_id)` → 启动代理 + patch CLI 配置 + emit `proxy-mode-changed`
- `proxy_disable(cli_id)` → 停止代理 + 还原 CLI 配置 + emit `proxy-mode-changed`
- `proxy_set_global(enabled)` → 批量启停 + emit `proxy-mode-changed`
- `proxy_get_mode_status()` → 返回 `ProxyModeStatus { global_enabled, cli_statuses }`

### 2.2 数据结构
```rust
// local.json 中的代理设置（Phase 9 已定义）
ProxySettings { global_enabled: bool, cli_enabled: HashMap<String, bool> }
ProxyTakeover { cli_ids: Vec<String> }

// 状态查询返回类型
ProxyModeStatus { global_enabled: bool, cli_statuses: Vec<CliProxyStatus> }
CliProxyStatus { cli_id, enabled, active, has_provider, port }
```

### 2.3 代理核心能力
- `ProxyService::update_upstream(cli_id, target)` — 运行时动态更新上游，不重启代理
- `ProxyState` 使用 `Arc<RwLock<Option<UpstreamTarget>>>` 保证线程安全
- 绑定失败返回 `ProxyError::BindFailed(String)`，包含端口和错误信息

### 2.4 前端 TS 封装
- `src/lib/tauri.ts` **尚未添加** proxy 相关的 TS 封装函数
- `src/types/settings.ts` 的 `LocalSettings` **尚未包含** proxy 相关字段

## 3. LIVE-01 分析：切换 Provider → 更新代理上游

**结论：后端已完全实现，前端无需额外代码。**

`set_active_provider` 命令（`provider.rs:435`）已包含代理模式判断：
- 检测 `proxy_takeover.cli_ids.contains(&cli_id)` 判断是否处于代理模式
- 代理模式下调用 `_set_active_provider_in_proxy_mode`：只更新 `active_providers` + 调用 `proxy_service.update_upstream()`
- 直连模式下保持现有行为（patch 真实凭据到 CLI 配置）

前端 `switchProvider` 调用的是 `setActiveProvider`（`tauri.ts:21`），最终走到这个命令，无需修改。

## 4. LIVE-02 分析：iCloud 同步变更 → 更新代理上游

### 现状
`watcher/mod.rs` 的 `process_events`:
1. 过滤 `.json` 文件变更，排除 self-write
2. 调用 `sync_changed_active_providers()` 重新 patch 活跃 Provider 的 CLI 配置
3. emit `providers-changed` 事件

### 问题
`sync_changed_active_providers` 是同步函数，调用 `_reconcile_active_providers_in`，只处理直连模式的 repatch。它不感知代理模式，也不调用 `proxy_service.update_upstream()`。

### 实现方案
在 `process_events` 中增加代理模式感知：
1. 从 `app_handle` 获取 `ProxyService` 状态
2. 读取 `local.json` 检查哪些 CLI 处于代理模式（`proxy_takeover.cli_ids`）
3. 对处于代理模式的 CLI，检查其活跃 Provider 是否在 `changed_files` 中
4. 如果是，读取新的 Provider 内容并调用 `proxy_service.update_upstream(cli_id, upstream)`

**关键挑战：** `process_events` 在 notify 的回调中执行，是同步上下文。而 `update_upstream` 是 async 操作。

**解决方案：** 使用 `tauri::async_runtime::spawn` 将 async 操作提交到 Tauri 的 tokio runtime：
```rust
let proxy_service = app_handle.state::<ProxyService>();
tauri::async_runtime::spawn(async move {
    proxy_service.update_upstream(&cli_id, upstream).await.ok();
});
```
这与 Phase 9 的 `restore_proxy_state` 使用的模式一致（`proxy.rs` 生命周期管理部分）。

## 5. LIVE-03 分析：前端 CRUD → 更新代理上游

### 5.1 update_provider
当用户编辑活跃 Provider 的 API key 或 base_url 时，需同步更新代理上游。

**现状：** `update_provider`（`provider.rs:401`）是同步命令（`#[tauri::command]`，非 async），不接收 `ProxyService` 状态。

**实现方案：**
1. 将 `update_provider` 改为 async 命令
2. 注入 `proxy_service: State<'_, ProxyService>`
3. 更新 Provider 文件后，检查该 Provider 是否是某个处于代理模式的 CLI 的活跃 Provider
4. 如果是，构造新的 `UpstreamTarget` 并调用 `proxy_service.update_upstream()`

### 5.2 delete_provider
当用户删除活跃 Provider 时：
- 如果该 CLI 处于代理模式，应自动关闭该 CLI 的代理模式（`_proxy_disable_in`）
- 然后执行正常的删除和 reconcile 逻辑

**现状：** `delete_provider`（`provider.rs:418`）也是同步命令。

**实现方案：**
1. 将 `delete_provider` 改为 async 命令
2. 注入 `proxy_service: State<'_, ProxyService>` 和 `app_handle: tauri::AppHandle`
3. 删除前检查：该 Provider 是否是某 CLI 的活跃 Provider 且该 CLI 处于代理模式
4. 如果是：先调用 `_proxy_disable_in` 关闭代理 → 然后正常删除 + reconcile
5. emit `proxy-mode-changed` 事件通知前端

## 6. UX-01 分析：端口占用错误提示

### 现状
`proxy_enable` 启动代理时，如果端口被占用：
- `ProxyServer::start()` → `TcpListener::bind()` 失败 → 返回 `ProxyError::BindFailed`
- `_proxy_enable_in` 回滚 CLI 配置为真实凭据
- 错误传播到前端为字符串：`"代理启动失败: 地址绑定失败: 127.0.0.1:15800: Address already in use"`

### 实现方案
前端在调用 `proxy_enable`/`proxy_set_global` 时：
- 捕获错误字符串
- 检测是否包含 "绑定失败" 或 "Address already in use" 关键词
- 显示友好的 toast 错误消息：`"端口 {port} 已被占用，无法启动代理。请关闭占用该端口的程序后重试。"`
- 开关回滚为关闭状态

后端已经有良好的错误传播链路，主要工作在前端 UI 层。

## 7. 前端 UI 实现方案

### 7.1 设置页全局开关（SettingsPage.tsx）

新增「代理模式」section，位于语言和测试配置之间：
- shadcn Switch 组件 + 说明文字
- 调用 `proxy_set_global(enabled)` + `proxy_get_mode_status()` 查询状态
- 错误时 toast + 开关回滚

**集成点：** `SettingsPage.tsx:118-120`（语言 section 和测试配置 section 之间的 `<Separator />`）

### 7.2 Tab 内 CLI 独立开关（ProviderTabs.tsx）

在 TabsList 和 TabsContent 之间插入代理开关行：
- shadcn Switch，位于标题栏区域
- 调用 `proxy_enable(cli_id)` / `proxy_disable(cli_id)`
- 全局未开启或无活跃 Provider 时置灰 + tooltip
- 监听 `proxy-mode-changed` 事件刷新状态

**集成点：** `ProviderTabs.tsx:163-175`（TabsList 和 TabsContent 之间的 `<div>` 行）

### 7.3 代理状态指示（Tab 标签绿色圆点）

TabsTrigger 内增加绿色圆点：
- 查询 `proxy_get_mode_status()` 获取各 CLI 的 `active` 状态
- active=true 时 TabsTrigger 内显示小绿点

### 7.4 TS 封装（tauri.ts）

新增函数：
```typescript
export async function proxyEnable(cliId: string): Promise<void>
export async function proxyDisable(cliId: string): Promise<void>
export async function proxySetGlobal(enabled: boolean): Promise<void>
export async function proxyGetModeStatus(): Promise<ProxyModeStatus>
```

### 7.5 类型定义（settings.ts）

新增：
```typescript
interface ProxySettings {
  global_enabled: boolean;
  cli_enabled: Record<string, boolean>;
}

interface CliProxyStatus {
  cli_id: string;
  enabled: boolean;
  active: boolean;
  has_provider: boolean;
  port: number | null;
}

interface ProxyModeStatus {
  global_enabled: boolean;
  cli_statuses: CliProxyStatus[];
}
```

LocalSettings 扩展 proxy 字段：
```typescript
interface LocalSettings {
  // ...existing fields
  proxy?: ProxySettings | null;
}
```

### 7.6 事件监听

新增 `useProxyStatus` hook 或扩展 `useSyncListener`：
- 监听 `proxy-mode-changed` 事件
- 触发时重新调用 `proxyGetModeStatus()` 刷新 UI
- SettingsPage 和 ProviderTabs 共用

## 8. i18n 文案

需新增的翻译 key：
- `settings.proxyMode` — "代理模式"
- `settings.proxyModeDescription` — "CLI 请求经本地代理转发，切换 Provider 无需重启 CLI"
- `proxy.enable` — "开启代理"
- `proxy.disable` — "关闭代理"
- `proxy.globalDisabled` — "请先在设置中开启代理"
- `proxy.noProvider` — "请先设置活跃 Provider"
- `proxy.portInUse` — "端口 {{port}} 已被占用，无法启动代理。请关闭占用该端口的程序后重试。"
- `proxy.enableFailed` — "开启代理失败"
- `proxy.enableSuccess` — "代理已开启"
- `proxy.disableSuccess` — "代理已关闭"

## 9. 依赖与风险

### 依赖
- Phase 9 的 4 个 Tauri 命令（已实现且有完整测试）
- Phase 8 的 `ProxyService::update_upstream` 方法（已实现）
- shadcn Switch 组件（UI 库已包含）

### 风险
1. **`update_provider`/`delete_provider` 同步转异步**：改变函数签名需要更新 `lib.rs` 中的命令注册（`invoke_handler`），但 Tauri 2 支持 async 命令
2. **watcher 同步上下文中 spawn async**：与 Phase 9 的 `restore_proxy_state` 模式一致，已验证可行
3. **前端状态同步**：多个组件共享 proxy 状态需确保一致性，建议使用事件驱动刷新（已有 `proxy-mode-changed` 事件）

## 10. 工作量拆分建议

### Plan 01: 后端代理联动（LIVE-02, LIVE-03, UX-01 后端部分）
- watcher `process_events` 新增代理上游更新
- `update_provider` 新增代理模式感知
- `delete_provider` 新增代理模式下自动关闭代理
- 端口占用错误信息优化（后端错误信息已够用，主要是前端）

### Plan 02: 前端 UI 集成（UX-01 前端部分, UI 开关, 状态指示）
- `tauri.ts` 新增 proxy 命令封装
- `settings.ts` 类型扩展
- `useProxyStatus` hook
- SettingsPage 全局开关 section
- ProviderTabs CLI 独立开关 + Tab 绿色状态点
- 端口占用 toast 处理
- i18n 文案
- `useSyncListener` 扩展 `proxy-mode-changed` 事件

## Validation Architecture

### 可测试性设计
1. **后端**：watcher 代理联动使用可注入的函数签名（延续 `_in` 模式），`update_provider`/`delete_provider` 的代理逻辑提取为 `_in` 函数
2. **前端**：proxy 命令封装函数可独立测试，hook 的事件监听逻辑可通过模拟 Tauri 事件测试

### 关键验证点
- LIVE-02：watcher 检测到活跃 Provider 变更时，代理上游自动更新
- LIVE-03：update_provider 修改活跃 Provider 时，代理上游同步更新；delete_provider 删除活跃 Provider 时，代理自动关闭
- UX-01：端口占用时显示友好错误提示
- UI：全局/独立开关联动正确，状态指示准确

## RESEARCH COMPLETE
