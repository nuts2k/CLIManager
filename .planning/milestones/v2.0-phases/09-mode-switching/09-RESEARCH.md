# Phase 9: 模式切换与持久化 - Research

**Researched:** 2026-03-13
**Status:** Complete

## 1. 现有代码资产盘点

### 1.1 ProxyService（proxy/mod.rs）
- `start(cli_id, port, upstream)` — 启动指定 CLI 的代理服务器
- `stop(cli_id)` — 停止指定 CLI 的代理
- `stop_all()` — 停止所有代理，返回 `Vec<(String, Result)>`
- `update_upstream(cli_id, upstream)` — 运行时更新上游目标（不重启代理）
- `status()` — 获取所有代理状态 `ProxyStatusInfo { servers: Vec<ServerStatus> }`
- 使用 `tokio::sync::Mutex` 管理多实例
- 作为 Tauri 托管状态注册：`app.manage(proxy::ProxyService::new())`

### 1.2 CliAdapter trait（adapter/mod.rs）
- `patch(provider)` — 将 CLI 配置文件 patch 为指定 Provider 的凭据
- `clear()` — 清除 CLI 配置中的凭据
- `create_backup() / restore_from_backup()` — 备份/还原
- ClaudeAdapter：修改 `~/.claude/settings.json` 的 `env.ANTHROPIC_AUTH_TOKEN` 和 `env.ANTHROPIC_BASE_URL`
- CodexAdapter：修改 `auth.json` 的 `OPENAI_API_KEY` 和 `config.toml` 的 `base_url`

### 1.3 set_active_provider（commands/provider.rs）
- `_set_active_provider_in(providers_dir, local_settings_path, cli_id, provider_id, adapter)` — 内部函数
- 流程：获取 Provider → adapter.patch(provider) → 更新 active_providers map → 写 local.json
- 代理模式下需要改造：跳过 adapter.patch()，改为调用 proxy_service.update_upstream()

### 1.4 LocalSettings（storage/local.rs）
- 存储路径：`~/.cli-manager/local.json`
- 现有字段：`active_providers`, `cli_paths`, `language`, `test_config`, `schema_version`
- 需扩展：proxy 开关状态 + takeover 标志

### 1.5 lib.rs 当前结构
- `app.run(|_app_handle, _event| {})` — 目前空回调，未处理退出事件
- `setup` 闭包：启动 file watcher、创建 tray
- 需要：在 `app.run()` 闭包中 hook `RunEvent::ExitRequested` 和 `RunEvent::Exit`

### 1.6 代理端口
- 端口在调用 proxy_start 命令时由前端传入，REQUIREMENTS 规定 Claude=15800, Codex=15801
- 代码中无端口常量定义，需要定义

## 2. 模式切换核心设计

### 2.1 开关层级
```
全局总开关 (settings page)
  ├── Claude 独立开关 (main window tab)
  └── Codex 独立开关 (main window tab)
```
- 总开关关 → 所有 CLI 独立开关置灰但保留状态（飞行模式）
- 总开关开 → 恢复之前各 CLI 独立开关状态
- CLI 独立开关需该 CLI 有活跃 Provider 才能启用

### 2.2 开启代理流程（单个 CLI）
```
1. 读取 active_provider_id → 获取 Provider 凭据
2. 构建 UpstreamTarget（真实 api_key + base_url + protocol_type）
3. adapter.patch(proxy_provider) — 将 CLI 配置 patch 为 localhost:port + PROXY_MANAGED
4. proxy_service.start(cli_id, port, upstream) — 启动代理服务器
5. 设置 takeover 标志（local.json）
6. toast 通知
```

### 2.3 关闭代理流程（单个 CLI）
```
1. 获取当前活跃 Provider
2. adapter.patch(active_provider) — 还原 CLI 配置为真实凭据
3. proxy_service.stop(cli_id) — 停止代理
4. 清除该 CLI 的 takeover 标志
5. toast 通知
```

### 2.4 代理模式下的 Provider 凭据构造
开启代理时需构造一个"代理专用 Provider"来 patch CLI 配置：
- Claude: `ANTHROPIC_AUTH_TOKEN=PROXY_MANAGED`, `ANTHROPIC_BASE_URL=http://127.0.0.1:15800`
- Codex: `OPENAI_API_KEY=PROXY_MANAGED`, `base_url=http://127.0.0.1:15801`

可以构造一个临时 Provider 对象传入 adapter.patch()，无需修改 adapter trait。

## 3. LocalSettings 扩展设计

### 3.1 新增字段
```rust
pub struct ProxySettings {
    /// 全局总开关
    pub global_enabled: bool,
    /// 每 CLI 独立开关状态 {"claude": true, "codex": false}
    pub cli_enabled: HashMap<String, bool>,
}

pub struct ProxyTakeover {
    /// 当前被接管的 CLI IDs ["claude", "codex"]
    pub cli_ids: Vec<String>,
}
```

### 3.2 在 LocalSettings 中集成
```rust
pub struct LocalSettings {
    // ...existing fields...
    #[serde(default)]
    pub proxy: Option<ProxySettings>,
    #[serde(default)]
    pub proxy_takeover: Option<ProxyTakeover>,
}
```

- `proxy` 存储用户主动设置的开关状态（持久化，UX-02）
- `proxy_takeover` 存储当前被接管的 CLI IDs（崩溃恢复标志，MODE-06）

## 4. 退出清理与崩溃恢复

### 4.1 Tauri 退出事件 Hook
```rust
app.run(|app_handle, event| {
    match event {
        RunEvent::ExitRequested { .. } => {
            // Cmd+Q 或窗口关闭触发
            // 同步执行：还原 CLI 配置 → 清除 takeover → 停止代理
        }
        RunEvent::Exit => {
            // 最终退出，此处也可做 best-effort 清理
        }
        _ => {}
    }
});
```

**关键约束：** `RunEvent::ExitRequested` 回调在主线程，不能直接 `.await`。
解决方案：
- 使用 `tauri::async_runtime::block_on()` 在回调中执行异步操作
- 或使用 `std::thread::spawn` + `block_on` 避免阻塞主线程
- 还原 CLI 配置（adapter.patch）是同步操作，可直接调用
- 停止代理（proxy_service.stop_all）是异步操作，需要 block_on

### 4.2 退出清理顺序
```
1. 读取 proxy_takeover.cli_ids
2. 对每个 cli_id：adapter.patch(active_provider) 还原为真实凭据
3. 清除 proxy_takeover（写 local.json）
4. proxy_service.stop_all()（best-effort）
```

先还原配置再停代理——确保 CLI 不会在代理停止后仍指向已关闭的 localhost。

### 4.3 崩溃恢复
应用启动时（setup 闭包中）：
```
1. 读取 local.json
2. 检查 proxy_takeover.cli_ids 是否非空
3. 如果非空 → 对每个 cli_id 执行 adapter.patch(active_provider) 还原
4. 清除 proxy_takeover
5. 日志记录恢复操作（不弹通知）
```

### 4.4 启动时自动恢复代理状态（UX-02）
```
1. 检查 proxy.global_enabled 和 proxy.cli_enabled
2. 如果总开关开启且有 CLI 启用 → 重新执行开启代理流程
3. 顺序：先崩溃恢复（还原所有） → 再按持久化状态重新开启
```

## 5. set_active_provider 改造

### 5.1 代理模式判断
在 `_set_active_provider_in` 或上层 `set_active_provider` 中增加逻辑：

```
if 该 cli_id 处于代理模式（proxy_takeover.cli_ids 包含 cli_id）:
    跳过 adapter.patch()
    调用 proxy_service.update_upstream(cli_id, new_upstream)
else:
    正常执行 adapter.patch(provider)
```

### 5.2 关闭代理时的还原
关闭代理时需 patch 为当前活跃 Provider 的真实凭据：
- 读取 active_providers[cli_id] → 获取 provider_id
- 从 iCloud 存储获取 Provider → 构造 adapter.patch(provider)

### 5.3 实现方式
`set_active_provider` 命令目前不接收 `AppHandle` 或 `State<ProxyService>`。
需要改造为注入 `State<ProxyService>` 来判断代理状态和调用 update_upstream。

或者：通过读取 local.json 的 `proxy_takeover` 字段判断是否处于代理模式，
然后通过前端发命令 `proxy_update_upstream` 来更新上游。

**推荐方案：** 后端 `set_active_provider` 注入 `ProxyService` state，一站式处理。
这需要将 `set_active_provider` 从同步命令改为异步命令（因为 ProxyService 使用 async Mutex）。

## 6. 新增 Tauri 命令

### 6.1 模式切换命令
```rust
// 开启指定 CLI 的代理模式
proxy_enable(cli_id: String) -> Result<(), String>

// 关闭指定 CLI 的代理模式
proxy_disable(cli_id: String) -> Result<(), String>

// 设置全局总开关
proxy_set_global(enabled: bool) -> Result<(), String>

// 获取代理模式状态（包含开关状态 + takeover 状态）
proxy_get_mode_status() -> Result<ProxyModeStatus, String>
```

### 6.2 ProxyModeStatus 结构
```rust
struct ProxyModeStatus {
    global_enabled: bool,
    cli_statuses: Vec<CliProxyStatus>,
}

struct CliProxyStatus {
    cli_id: String,
    enabled: bool,        // 用户开关状态
    active: bool,         // 实际代理运行中
    has_provider: bool,   // 是否有活跃 Provider
    port: Option<u16>,    // 代理端口
}
```

## 7. 端口常量定义

```rust
// proxy/mod.rs 或新建 proxy/config.rs
pub const PROXY_PORT_CLAUDE: u16 = 15800;
pub const PROXY_PORT_CODEX: u16 = 15801;

pub fn proxy_port_for_cli(cli_id: &str) -> Option<u16> {
    match cli_id {
        "claude" => Some(PROXY_PORT_CLAUDE),
        "codex" => Some(PROXY_PORT_CODEX),
        _ => None,
    }
}
```

## 8. 错误处理与回滚

### 8.1 开启失败回滚
开启代理的任一步骤失败时需完整回滚：
```
如果 adapter.patch(proxy) 成功但 proxy_service.start() 失败：
    → adapter.patch(active_provider) 还原 CLI 配置
    → 开关回滚为关闭
    → toast 错误提示
```

### 8.2 关闭失败处理
关闭代理的步骤如果 adapter.patch(real_provider) 失败：
- 尝试 adapter.clear() 清除凭据
- 记录错误日志
- 仍然停止代理并清除 takeover

## 9. 前端事件通知

### 9.1 复用现有事件机制
- 使用 `app_handle.emit("proxy-mode-changed", payload)` 通知前端
- 前端监听该事件更新 UI 状态
- 退出清理和崩溃恢复时不发事件（无前端可接收）

### 9.2 Toast 通知
- 使用已有 sonner 组件
- 成功："Claude Code 代理已开启" / "代理已关闭"
- 失败："开启代理失败: {原因}"

## 10. 计划分解建议

### Plan 1: 后端核心 — LocalSettings 扩展 + 模式切换命令 + 端口常量
- 扩展 LocalSettings（ProxySettings + ProxyTakeover）
- 定义端口常量
- 实现 proxy_enable / proxy_disable / proxy_set_global / proxy_get_mode_status 命令
- 改造 set_active_provider 注入代理模式判断
- 单元测试

### Plan 2: 退出清理 + 崩溃恢复 + 启动自动恢复
- lib.rs RunEvent hook 实现退出清理
- setup 闭包中实现崩溃恢复检查
- 启动时自动恢复代理状态（UX-02）
- 集成测试

## Validation Architecture

### Dimension 8: 测试策略

**单元测试：**
- LocalSettings 序列化/反序列化（ProxySettings + ProxyTakeover 字段）
- proxy_port_for_cli 端口映射
- 模式切换命令的内部函数（_proxy_enable_in, _proxy_disable_in）
- set_active_provider 代理模式分支

**集成测试：**
- 完整开启/关闭代理流程（adapter.patch + proxy_service.start/stop）
- 错误回滚：proxy_service.start 失败时配置被还原
- 崩溃恢复：遗留 takeover 标志 → 启动时自动还原
- 退出清理：多个 CLI 同时代理中 → 全部还原

**测试方式：**
- 使用 `_in` 变体注入路径和 adapter（已有模式）
- 代理模式判断通过读取 local.json proxy_takeover 字段（纯文件操作，可测试）
- ProxyService 使用 port=0（动态端口）避免端口冲突

---

## RESEARCH COMPLETE

*Phase: 09-mode-switching*
*Researched: 2026-03-13*
