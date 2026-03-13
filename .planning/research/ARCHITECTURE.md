# Architecture Patterns: Local Proxy Integration

**Domain:** 本地代理服务集成到现有 Tauri 2 桌面应用 (CLIManager)
**Researched:** 2026-03-13
**Confidence:** HIGH (基于对完整代码库的逐文件阅读 + 官方文档验证)

## 现有架构概览（作为集成基线）

当前 CLIManager 的模块结构（v1.1 shipped）：

```
src-tauri/src/
  lib.rs            — Tauri Builder setup, 窗口事件, 文件监听, 托盘初始化
  provider.rs       — Provider/ProtocolType/ModelConfig 数据结构
  error.rs          — AppError 统一错误枚举
  adapter/
    mod.rs          — CliAdapter trait + PatchResult + backup/restore 工具函数
    claude.rs       — ClaudeAdapter: patch ~/.claude/settings.json
    codex.rs        — CodexAdapter: patch ~/.codex/auth.json + config.toml
  commands/
    mod.rs          — 命令模块注册
    provider.rs     — 所有 Tauri 命令: CRUD, set_active, sync, test_provider 等
    onboarding.rs   — 首次导入 CLI 配置
  storage/
    mod.rs          — atomic_write 工具函数
    icloud.rs       — iCloud 同步层: Provider JSON 文件读写
    local.rs        — 本地层: LocalSettings (active_providers, cli_paths, language 等)
  tray.rs           — 系统托盘菜单构建与事件处理
  watcher/
    mod.rs          — FSEvents 文件监听 + providers-changed 事件
    self_write.rs   — SelfWriteTracker 防止无限循环
```

### 关键数据流（直连模式 — 当前行为）

```
用户点击切换 Provider
  → set_active_provider 命令
    → 读取 Provider 数据 (icloud storage)
    → get_adapter_for_cli() 创建 ClaudeAdapter/CodexAdapter
    → adapter.patch(provider) — surgical patch 写入 CLI 配置文件
    → 更新 LocalSettings.active_providers
    → CLI 下次运行时读取新配置（需重启生效）
```

## 推荐架构：代理模块集成

### 新增模块

```
src-tauri/src/
  proxy/                     ← 新增顶层模块
    mod.rs                   — 公共类型 + ProxyManager 定义
    server.rs                — 单个 axum 代理服务器的创建/启停
    handler.rs               — HTTP 请求转发处理器（透明代理）
    state.rs                 — ProxyState: 共享的活跃 Provider 路由表
```

### 修改现有模块

| 模块 | 修改内容 | 原因 |
|------|----------|------|
| `storage/local.rs` | `LocalSettings` 新增 `proxy_settings` 字段 | 存储代理模式配置 |
| `commands/provider.rs` | `set_active_provider` 流程内新增代理模式分支 | 代理模式下更新路由表而非 patch 文件 |
| `commands/mod.rs` | 注册新的代理相关命令 | 暴露代理状态/控制给前端 |
| `lib.rs` | setup 中初始化 ProxyManager, 注册 Tauri State | 代理服务随应用启动 |
| `adapter/mod.rs` | `CliAdapter` trait 新增 `patch_for_proxy` 方法 | 代理模式写 localhost 而非真实凭据 |
| `error.rs` | 新增 `Proxy` 错误变体 | 代理启停失败、端口占用等错误 |

### 不修改的模块

| 模块 | 为什么不改 |
|------|-----------|
| `storage/icloud.rs` | Provider 数据模型不变，代理不影响同步层 |
| `watcher/` | 文件监听逻辑不变，代理模式下 Provider 变更仍触发路由表更新 |
| `tray.rs` | v2.0 不在托盘显示代理状态，保持现有菜单结构 |
| `provider.rs` | Provider 数据结构不变，代理是传输层概念 |

### 组件边界与职责

| 组件 | 职责 | 与其他组件通信 |
|------|------|----------------|
| **ProxyManager** | 管理多个代理服务器实例的生命周期（启动/停止/重启） | 被 lib.rs setup 初始化，被 commands 调用，注册为 Tauri State |
| **ProxyServer** | 单个 CLI 的 axum 服务器实例（绑定到固定端口） | 被 ProxyManager 创建和管理 |
| **ProxyState** | 共享状态：当前活跃 Provider 的 API key / base_url / headers | 被 handler 读取，被 commands 写入 |
| **handler** | 接收 CLI 请求 -> 从 ProxyState 读取当前凭据 -> 转发到真实 API | 读取 ProxyState，使用 reqwest 发出上游请求 |
| **ProxySettings** | 持久化配置：全局开关、每 CLI 开关、端口号 | 存储在 LocalSettings.proxy_settings |

## 详细设计

### 1. ProxySettings 数据结构（存储在 local.json）

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProxyCliConfig {
    pub enabled: bool,        // 该 CLI 是否启用代理
    pub port: u16,            // 固定端口号
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProxySettings {
    pub enabled: bool,                                   // 全局总开关
    pub cli_configs: HashMap<String, ProxyCliConfig>,    // 每 CLI 独立配置
}

impl Default for ProxySettings {
    fn default() -> Self {
        let mut cli_configs = HashMap::new();
        cli_configs.insert("claude".to_string(), ProxyCliConfig {
            enabled: false,
            port: 18510,    // Claude Code 默认端口
        });
        cli_configs.insert("codex".to_string(), ProxyCliConfig {
            enabled: false,
            port: 18520,    // Codex 默认端口
        });
        Self {
            enabled: false,
            cli_configs,
        }
    }
}
```

**端口选择策略：** 使用 185xx 范围（IANA 未分配端口），按 CLI 固定分配。固定端口比动态端口更适合本项目，因为：
- CLI 配置文件需要写入固定地址，动态端口每次重启都需重新 patch
- 用户多次重启应用后，CLI 配置无需重新 patch
- 不存在多实例运行场景（桌面应用单实例）

**存储位置：** 追加到现有 `LocalSettings`，因为代理设置是设备本地的，不应跨设备同步（与 active_providers、cli_paths 同层）。

```rust
// storage/local.rs — LocalSettings 新增字段
pub struct LocalSettings {
    // ...现有字段不变...
    #[serde(default)]
    pub proxy_settings: ProxySettings,
}
```

### 2. ProxyState：共享路由表

```rust
// proxy/state.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::provider::ProtocolType;

#[derive(Debug, Clone)]
pub struct ProviderRoute {
    pub api_key: String,
    pub base_url: String,         // 上游 API 地址（如 https://api.anthropic.com）
    pub protocol_type: ProtocolType,
}

/// 代理路由表 — 存储每个 CLI 当前应该转发到哪个上游
pub struct ProxyState {
    routes: Arc<RwLock<HashMap<String, ProviderRoute>>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 更新某 CLI 的路由（Provider 切换时调用）
    pub async fn update_route(&self, cli_id: &str, route: ProviderRoute) {
        let mut map = self.routes.write().await;
        map.insert(cli_id.to_string(), route);
    }

    /// 获取某 CLI 的当前路由（每个代理请求调用）
    pub async fn get_route(&self, cli_id: &str) -> Option<ProviderRoute> {
        let map = self.routes.read().await;
        map.get(cli_id).cloned()
    }

    /// 移除某 CLI 的路由（禁用代理时调用）
    pub async fn remove_route(&self, cli_id: &str) {
        let mut map = self.routes.write().await;
        map.remove(cli_id);
    }
}
```

**为什么用 `tokio::sync::RwLock` 而非 `std::sync::Mutex`：**
- 路由表读多写少（每个请求都读，只有切换 Provider 时写）
- RwLock 允许多个并发请求同时读取路由信息
- axum handler 是 async 上下文，tokio RwLock 更自然
- 注意：对于极低并发（本地单用户），`std::sync::Mutex` 也完全可行，但 RwLock 语义更准确

### 3. ProxyManager：服务器生命周期管理

```rust
// proxy/mod.rs
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

pub struct ProxyManager {
    pub state: ProxyState,
    /// cli_id -> (task_handle, shutdown_sender)
    servers: Mutex<HashMap<String, (JoinHandle<()>, oneshot::Sender<()>)>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            state: ProxyState::new(),
            servers: Mutex::new(HashMap::new()),
        }
    }

    /// 启动某 CLI 的代理服务器
    pub fn start_server(&self, cli_id: &str, port: u16) -> Result<(), AppError> { ... }

    /// 停止某 CLI 的代理服务器
    pub fn stop_server(&self, cli_id: &str) -> Result<(), AppError> { ... }

    /// 检查某 CLI 的代理是否运行中
    pub fn is_running(&self, cli_id: &str) -> bool { ... }

    /// 获取所有代理的运行状态
    pub fn get_status(&self) -> ProxyStatus { ... }
}
```

**生命周期流程：**

```
应用启动 (lib.rs setup)
  -> 读取 ProxySettings from local.json
  -> 创建 ProxyManager（注册为 Tauri State）
  -> 对每个 enabled 且全局 enabled 的 CLI:
     -> 从 active_providers 读取当前 Provider
     -> 初始化 ProxyState.routes
     -> 启动 ProxyServer（绑定端口）

应用运行中
  -> Provider 切换：仅更新 ProxyState.routes（毫秒级，无文件 IO）
  -> 模式切换（直连->代理）：启动 ProxyServer + patch CLI 配置指向 localhost
  -> 模式切换（代理->直连）：停止 ProxyServer + patch CLI 配置恢复真实凭据
  -> 端口变更：停止旧服务器 -> 启动新服务器 -> 重新 patch CLI 配置

应用退出
  -> ProxyManager.servers 中的 JoinHandle 随 tokio runtime 自动 drop
  -> CLI 配置保持 localhost（下次启动 CLIManager 会自动恢复代理）
```

### 4. handler：透明代理转发

```rust
// proxy/handler.rs — 核心转发逻辑

async fn proxy_handler(
    State(proxy_state): State<ProxyState>,
    req: Request,
) -> Result<Response, StatusCode> {
    // 1. 从 ProxyState 读取当前路由
    let route = proxy_state.get_route(&cli_id).await
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // 2. 构建上游 URL
    //    保留原始 path + query（如 /v1/messages?stream=true）
    //    前缀替换为 route.base_url

    // 3. 构建上游请求
    //    - 透传原始 body（包括 streaming）
    //    - 透传大部分 header（content-type, anthropic-beta, anthropic-version 等）
    //    - 移除原始认证 header
    //    - 注入真实认证 header:
    //      Anthropic: x-api-key: {route.api_key}
    //      OpenAI:    Authorization: Bearer {route.api_key}

    // 4. 发送请求并透传响应
    //    - 直接透传 status code
    //    - 直接透传 response headers
    //    - 直接透传 response body（streaming SSE 透传）
}
```

**为什么自己实现 handler 而非使用 `axum-reverse-proxy` crate：**
- `axum-reverse-proxy` 适合静态路由的通用反向代理，但我们需要**动态注入认证信息**
- 我们的场景很简单：接收请求 -> 读路由表获取凭据 -> 替换 header -> 转发 -> 透传响应
- 自己实现约 80-100 行代码，用 `reqwest`（已有依赖）即可
- 避免引入不需要的功能（dns discovery, load balancing, retry, RFC 9110 compliance）
- 完全可控，方便未来扩展（协议转换、请求日志等 2.x 特性）

**协议处理细节：**

| CLI | 入站请求格式 | 需要注入的 Header | 需要透传的 Header | 上游 API |
|-----|-------------|-------------------|-------------------|----------|
| Claude Code | Anthropic Messages (`/v1/messages`) | `x-api-key` | `anthropic-version`, `anthropic-beta`, `content-type` | `{route.base_url}/v1/messages` |
| Claude Code | Token counting (`/v1/messages/count_tokens`) | `x-api-key` | 同上 | `{route.base_url}/v1/messages/count_tokens` |
| Codex | OpenAI Compatible (`/v1/chat/completions`) | `Authorization: Bearer {key}` | `content-type` | `{route.base_url}/v1/chat/completions` |

**Streaming 支持（关键）：**
- CLI 的 API 调用几乎全部使用 SSE streaming
- 必须支持将上游的 streaming response 透传给 CLI
- reqwest 的 `.bytes_stream()` + axum 的 `Body::from_stream()` 可以实现零缓冲透传
- 不要在代理中将整个响应缓存到内存再返回

### 5. CliAdapter 扩展：代理模式 patch

代理模式下，需要将 CLI 配置指向 localhost 而非真实 API。这是对现有 patch 流的扩展：

```rust
// adapter/mod.rs — 新增方法
pub trait CliAdapter {
    fn cli_name(&self) -> &str;
    fn patch(&self, provider: &Provider) -> Result<PatchResult, AppError>;
    fn clear(&self) -> Result<PatchResult, AppError>;

    /// 代理模式: 将 CLI 配置指向 localhost proxy
    fn patch_for_proxy(&self, port: u16) -> Result<PatchResult, AppError>;
}
```

**Claude Code 代理 patch 行为：**

```json
// ~/.claude/settings.json — 代理模式下写入
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "cli-manager-proxy",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:18510"
  }
}
```

关于 `ANTHROPIC_AUTH_TOKEN` 的处理：
- Claude Code 要求此值非空才会发送请求
- 代理模式下填入占位符 `"cli-manager-proxy"`，代理 handler 会用真实 key 替换
- 代理收到请求时忽略入站的 `x-api-key` header，从 ProxyState 注入真实值
- 这符合 Claude Code 官方文档中 LLM gateway 的使用模式

**Codex 代理 patch 行为：**

Codex 有一个已知限制：覆盖内建 `openai` provider 的 base_url 在 config.toml 中可能不被尊重（[issue #11698](https://github.com/openai/codex/issues/11698)）。

推荐方案：为 Codex 创建自定义 provider 配置以绕过此限制：

```json
// ~/.codex/auth.json
{ "OPENAI_API_KEY": "cli-manager-proxy" }
```

```toml
# ~/.codex/config.toml — 代理模式下写入
model_provider = "climanager"

[model_providers.climanager]
name = "CLIManager Proxy"
base_url = "http://127.0.0.1:18520"
env_key = "OPENAI_API_KEY"
```

这样避开了内建 provider 覆盖问题，也不依赖环境变量。

**`patch_for_proxy` vs `patch` 的区别：**
- `patch_for_proxy` 不需要 backup（写入的是固定的 localhost 地址，不涉及敏感凭据）
- `patch_for_proxy` 不需要 rollback 逻辑（失败时只需报错，不会损坏凭据）
- `patch_for_proxy` 仍然使用 surgical patch（保留 CLI 配置的其他字段）

### 6. set_active_provider 数据流变更

现有流程 vs 代理模式流程的分支点在 `patch_provider_for_cli`：

```
set_active_provider(cli_id, provider_id)
  -> 读取 Provider 数据
  -> 读取 ProxySettings

  分支判断: 该 CLI 是否启用代理？
  条件: proxy_settings.enabled && cli_config.enabled && proxy_server 运行中

  【直连模式（现有行为，不变）】
    -> adapter.patch(provider)       // 写入真实凭据到 CLI 配置
    // CLI 需重启才能读取新配置

  【代理模式（新行为）】
    -> proxy_state.update_route(cli_id, ProviderRoute::from(provider))
    // 纯内存操作，CLI 的下一个 API 调用立即使用新凭据
    // 不需要 patch 文件，CLI 配置已指向 localhost

  -> 更新 LocalSettings.active_providers（两种模式都执行）
  -> emit providers-changed（两种模式都执行）
  -> 更新托盘菜单（两种模式都执行）
```

**关键收益：** 代理模式下切换 Provider 是纯内存操作（约 1 微秒），无文件 IO，CLI 正在进行的对话下一次 API 调用就会使用新凭据。

### 7. 模式切换数据流

```
用户切换 直连->代理模式（某 CLI）：
  1. 启动该 CLI 的 ProxyServer（绑定端口）
  2. 从 active_providers 读取当前活跃 Provider
  3. 初始化 ProxyState 路由表
  4. adapter.patch_for_proxy(port)  — CLI 配置指向 localhost
  5. 保存 ProxySettings 到 local.json
  6. emit proxy-status-changed

用户切换 代理->直连模式（某 CLI）：
  1. 读取当前活跃 Provider
  2. adapter.patch(provider)  — CLI 配置恢复为真实凭据
  3. 停止该 CLI 的 ProxyServer
  4. 清除 ProxyState 路由
  5. 保存 ProxySettings 到 local.json
  6. emit proxy-status-changed
```

**注意切换顺序：**
- 开启代理：先启动服务器，再 patch CLI 配置。避免 CLI 请求到 localhost 但服务器还没就绪。
- 关闭代理：先恢复 CLI 配置，再停止服务器。避免 CLI 请求到 localhost 但服务器已关闭。

### 8. 前端集成

**新增 Tauri 命令：**

```rust
#[tauri::command]
fn get_proxy_status(app: AppHandle) -> ProxyStatus;

#[tauri::command]
fn toggle_proxy_global(app: AppHandle, enabled: bool) -> Result<ProxySettings, AppError>;

#[tauri::command]
fn toggle_proxy_cli(app: AppHandle, cli_id: String, enabled: bool) -> Result<ProxySettings, AppError>;

#[tauri::command]
fn update_proxy_port(app: AppHandle, cli_id: String, port: u16) -> Result<ProxySettings, AppError>;
```

**新增 Tauri 事件：**

```rust
// 代理状态变化时 emit（服务器启停、模式切换）
app_handle.emit("proxy-status-changed", ProxyStatusPayload { ... });
```

**前端类型扩展（types/settings.ts）：**

```typescript
interface ProxyCliConfig {
  enabled: boolean;
  port: number;
}

interface ProxySettings {
  enabled: boolean;
  cli_configs: Record<string, ProxyCliConfig>;
}

// 运行时状态（不持久化，每次从后端获取）
interface ProxyStatus {
  global_enabled: boolean;
  servers: Record<string, {
    config_enabled: boolean;   // 配置上是否启用
    running: boolean;          // 实际是否运行中
    port: number;
    error?: string;            // 启动失败时的错误信息
  }>;
}
```

## 整体架构图

```
┌─────────────────────────────────────────────────────────┐
│                    CLIManager 应用                       │
│                                                         │
│  ┌─────────────┐  ┌────────────────────────────────┐    │
│  │   Frontend   │  │          Tauri 命令层           │    │
│  │  (React 19)  │──│  provider.rs + proxy commands  │    │
│  └─────────────┘  └─────────┬──────────────────────┘    │
│                              │                           │
│         ┌────────────────────┼────────────────┐          │
│         │                    │                │          │
│         ▼                    ▼                ▼          │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐     │
│  │   Storage    │  │ ProxyManager │  │  Adapter   │     │
│  │ local.json   │  │  (NEW)       │  │ CliAdapter │     │
│  │ (设置+代理)  │  │  ┌────────┐  │  │            │     │
│  └─────────────┘  │  │ State  │  │  └──────┬─────┘     │
│                    │  │(路由表)│  │         │           │
│                    │  └───┬────┘  │         │           │
│                    │      │       │         │           │
│                    │  ┌───▼────┐  │         │           │
│                    │  │Server  │  │         ▼           │
│                    │  │:18510  │  │  ┌──────────────┐   │
│                    │  │(claude)│  │  │ CLI 配置文件  │   │
│                    │  ├────────┤  │  │ settings.json│   │
│                    │  │Server  │  │  │ auth.json    │   │
│                    │  │:18520  │  │  │ config.toml  │   │
│                    │  │(codex) │  │  └──────────────┘   │
│                    │  └───┬────┘  │                      │
│                    └──────┼──────┘                       │
│                           │                              │
└───────────────────────────┼──────────────────────────────┘
                            │
              ┌─────────────┼─────────────┐
              ▼                           ▼
   ┌──────────────────┐       ┌──────────────────┐
   │ Claude Code CLI   │       │ Codex CLI         │
   │ -> localhost:18510│       │ -> localhost:18520│
   │ -> /v1/messages   │       │ -> /v1/chat/...   │
   └────────┬─────────┘       └────────┬─────────┘
            │                          │
            ▼                          ▼
   ┌──────────────────┐       ┌──────────────────┐
   │ 代理 handler      │       │ 代理 handler      │
   │ 注入真实凭据       │       │ 注入真实凭据       │
   │ 转发到上游 API     │       │ 转发到上游 API     │
   └────────┬─────────┘       └────────┬─────────┘
            │                          │
            ▼                          ▼
   ┌──────────────────┐       ┌──────────────────┐
   │ Anthropic API     │       │ OpenAI API        │
   │ 或自定义 base_url │       │ 或自定义 base_url │
   └──────────────────┘       └──────────────────┘
```

## 应遵循的模式

### Pattern 1: Tauri State + Arc 共享状态

**什么：** ProxyManager 通过 Tauri 的 `.manage()` 注册为全局状态，命令层通过 `State<ProxyManager>` 访问。

**为什么：** 与现有 SelfWriteTracker 的管理方式完全一致（`app.manage(watcher::SelfWriteTracker::new())`），是 Tauri 原生支持的状态共享模式。

**示例：**
```rust
// lib.rs setup
let proxy_manager = proxy::ProxyManager::new();
let builder = tauri::Builder::default()
    .manage(watcher::SelfWriteTracker::new())
    .manage(proxy_manager)  // 新增
```

### Pattern 2: _in/_to 内部函数变体延续

**什么：** 现有代码用 `_set_active_provider_in(providers_dir, settings_path, ...)` 的内部函数变体实现可测试性。代理相关函数也应延续此模式。

**为什么：** 测试隔离无需 mock 文件系统路径或真实网络端口。这是全项目一致的测试模式（见 PROJECT.md Key Decisions）。

**示例：**
```rust
// 公开 Tauri 命令
#[tauri::command]
fn toggle_proxy_cli(app: AppHandle, cli_id: String, enabled: bool) -> Result<...> {
    let manager = app.state::<ProxyManager>();
    _toggle_proxy_cli_in(&manager, &cli_id, enabled, &settings_path, adapter)
}

// 可测试的内部实现
fn _toggle_proxy_cli_in(
    manager: &ProxyManager,
    cli_id: &str,
    enabled: bool,
    settings_path: &Path,
    adapter: Option<Box<dyn CliAdapter>>,
) -> Result<...> { ... }
```

### Pattern 3: 优雅关闭 via oneshot channel

**什么：** 每个 ProxyServer 启动时保留一个 `oneshot::Sender<()>`，关闭时发送信号触发 `axum::serve().with_graceful_shutdown()`。

**为什么：** 确保进行中的 streaming 请求能完成，避免 CLI 收到断开连接错误。

**示例：**
```rust
let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

tauri::async_runtime::spawn(async move {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async { shutdown_rx.await.ok(); })
        .await?;
});

// 需要关闭时
let _ = shutdown_tx.send(());
```

### Pattern 4: 使用 tauri::async_runtime::spawn 而非 tokio::spawn

**什么：** 在 Tauri 应用中启动 async 任务时，使用 `tauri::async_runtime::spawn`。

**为什么：** `tokio::spawn` 在 Tauri handler 中可能静默失败（已知问题，见 [Discussion #11831](https://github.com/tauri-apps/tauri/discussions/11831)）。`tauri::async_runtime::spawn` 使用 Tauri 管理的 tokio runtime，更可靠。

### Pattern 5: 事件复用 providers-changed

**什么：** 代理模式下切换 Provider 后，复用现有 `providers-changed` 事件通知前端刷新，新增 `proxy-status-changed` 仅用于代理开关/状态变化。

**为什么：** 前端已有 `useSyncListener` 监听 `providers-changed` 事件并刷新 UI。代理模式下 Provider 切换的最终效果相同（活跃 Provider 变了），复用同一事件保持一致。

### Pattern 6: 端口绑定检测 + 明确错误

**什么：** 启动代理服务器时，`TcpListener::bind()` 失败应返回清晰的错误信息（端口被占用），而非静默失败。

**为什么：** 用户可能有其他服务占用 185xx 端口。错误信息应引导用户到设置页修改端口。

## 应避免的反模式

### Anti-Pattern 1: 动态端口分配

**什么：** 每次启动代理时随机分配端口（如 `TcpListener::bind(("127.0.0.1", 0))`）。

**为什么不好：** CLI 配置文件中已写入端口号，端口变化意味着需要重新 patch 所有 CLI 配置。如果应用崩溃后重启，分配到不同端口，CLI 配置指向已失效的端口。

**替代方案：** 固定端口 + 启动时检测端口冲突，报错提示用户修改端口号。

### Anti-Pattern 2: 每次请求读取 Provider 文件

**什么：** 代理 handler 中每个请求都从 iCloud JSON 文件读取 Provider 数据。

**为什么不好：** 文件 IO 延迟会直接影响每个 CLI 请求的响应时间。iCloud 文件驱逐问题（.icloud 占位文件）会导致读取失败。

**替代方案：** 路由表在内存中维护（ProxyState），仅在 Provider 切换时更新。

### Anti-Pattern 3: 代理和直连模式共享同一个 patch 逻辑

**什么：** 代理模式写入 localhost 地址时，走完整的 `adapter.patch()` 流（backup + surgical patch + validate + rollback）。

**为什么不好：** 代理模式写入的是固定的 localhost 地址，不涉及敏感凭据。backup/rollback 机制增加无谓复杂度，失败回滚语义不明确（回滚到什么？上一个 localhost 地址？）。

**替代方案：** 新增 `patch_for_proxy(port)` 方法，简化写入逻辑。仍然用 surgical patch 保留其他配置，但不做 backup。

### Anti-Pattern 4: 应用退出时自动恢复 CLI 配置

**什么：** 应用退出时自动将 CLI 配置从 localhost 恢复为直连模式（真实凭据）。

**为什么不好：**
- 应用可能非正常退出（崩溃、kill -9），恢复逻辑不会执行
- 用户可能希望下次启动时继续使用代理模式
- 恢复需要知道"直连时的凭据"，存储在哪？增加状态管理复杂度

**替代方案：** 不自动恢复。应用启动时检测代理设置，自动启动代理服务。CLI 在代理未运行时请求会失败，用户知道需要启动 CLIManager。

### Anti-Pattern 5: 全局 reqwest Client 复用

**什么：** 所有 CLI 的代理 handler 共用一个 reqwest Client 实例。

**为什么需要注意：** reqwest Client 维护连接池。不同 CLI 连接不同上游（Anthropic vs OpenAI），连接池不会互相干扰。但如果未来加入 TLS 客户端证书等配置，需要 per-CLI Client。

**建议：** v2.0 可以用一个共享 Client（简单），但架构上预留 per-CLI Client 的扩展点。

## 扩展性考量

| 关注点 | 当前阶段（v2.0） | 未来（2.x+） |
|--------|------------------|-------------|
| 并发请求 | 单用户本地代理，并发极低（<10 并发） | 不变 |
| Provider 数量 | 路由表 < 20 条，RwLock 无性能问题 | 不变 |
| 协议转换 | 不做。Claude->Anthropic, Codex->OpenAI 各自透传 | 2.x 加 Anthropic<->OpenAI 转换层 |
| 请求日志 | 不做。仅启停日志 | 2.x 加流量监控与可视化 |
| Streaming | 必须支持。CLI 的 API 调用几乎全是 SSE streaming | 不变 |
| 多端口管理 | 2 个端口（claude + codex） | 新增 CLI 时加端口 |
| 热更新 | Provider 切换 = 写 ProxyState（微秒级） | 不变 |
| Failover | 不做 | 2.x 加自动切换备用 Provider |

## 构建顺序（尊重现有模块依赖）

基于对现有代码的分析，推荐以下构建顺序。每个 Phase 结束后系统仍可正常运行。

### Phase 1: 数据层 + 核心类型
**目标：** 代理设置可持久化，类型系统就绪

1. `error.rs` — 新增 `Proxy(String)` 错误变体
2. `proxy/state.rs` — ProxyState + ProviderRoute 类型
3. `proxy/mod.rs` — ProxyManager 骨架（空实现）
4. `storage/local.rs` — LocalSettings 新增 `proxy_settings: ProxySettings` 字段

**依赖：** 无新外部依赖，纯数据结构
**验证：** 单元测试确认序列化兼容（旧 local.json 不含 proxy_settings 时 default 生效）

### Phase 2: 代理服务器核心
**目标：** 能启动/停止 axum 服务器，能转发请求

1. `proxy/handler.rs` — HTTP 转发处理器（先做硬编码路由测试）
2. `proxy/server.rs` — 单个 axum 服务器启停（bind + graceful shutdown）
3. `proxy/mod.rs` — ProxyManager 完整实现

**依赖：** Cargo.toml 新增 `axum = "0.8"`, `tower`, `hyper`, `hyper-util`, `http-body-util`
**注意：** Tauri 2 已使用 tokio runtime，axum 复用同一 runtime，不会冲突
**验证：** 集成测试：启动代理 -> 发送请求 -> 验证转发

### Phase 3: Adapter 扩展
**目标：** CLI 配置可被 patch 指向 localhost

1. `adapter/mod.rs` — CliAdapter trait 加 `patch_for_proxy(port)`
2. `adapter/claude.rs` — 实现 Claude 的 proxy patch
3. `adapter/codex.rs` — 实现 Codex 的 proxy patch（含自定义 provider 方案）

**依赖：** Phase 1 的 ProxySettings（读取端口号）
**验证：** 单元测试：patch_for_proxy 后 CLI 配置指向正确的 localhost 端口

### Phase 4: 命令层集成
**目标：** 前端可控制代理，Provider 切换在代理模式下走新路径

1. `commands/proxy.rs` — 新增代理相关 Tauri 命令
2. `commands/provider.rs` — 修改 `patch_provider_for_cli` 加代理模式分支
3. `commands/mod.rs` — 注册新命令
4. `lib.rs` — setup 中初始化 ProxyManager，注册 Tauri State，按设置自动启动

**依赖：** Phase 1-3 全部完成
**验证：** 端到端测试：从 Tauri 命令启用代理 -> 切换 Provider -> 验证路由表更新

### Phase 5: 前端 UI
**目标：** 用户可通过 UI 控制代理

1. `types/settings.ts` — 新增 ProxySettings/ProxyStatus TypeScript 类型
2. `lib/tauri.ts` — 新增代理相关 invoke 封装
3. Settings 页面 — 全局总开关
4. CLI Tab — 每 CLI 独立开关 + 端口显示 + 运行状态指示器
5. hooks — 扩展 useSettings 或新增 useProxyStatus

**依赖：** Phase 4 的 Tauri 命令
**验证：** 手动测试：UI 操作 -> 观察代理启停 -> 验证 CLI 请求能通过代理

## 技术栈新增依赖

```toml
# Cargo.toml 新增
axum = "0.8"            # HTTP 框架（代理服务器骨架）
tower = "0.5"           # middleware 基础设施
hyper = "1"             # HTTP 底层（axum 已依赖，显式声明用于 body 类型）
hyper-util = "0.1"      # Body 类型转换工具
http-body-util = "0.1"  # Body streaming 支持

# 已有，复用：
# reqwest — 用于上游请求转发
# tokio — 被 Tauri 引入，axum 复用同一 runtime
# serde, serde_json — 序列化
```

**注意：** `axum 0.8` (2025-01) 使用新的路径语法 `/{param}` 替代旧的 `/:param`。

## 关键技术决策记录

| 决策 | 理由 |
|------|------|
| 固定端口而非动态端口 | CLI 配置写入后不应变化，避免端口漂移问题 |
| 透明代理而非协议转换 | v2.0 范围明确：只做凭据注入+转发，协议转换留给 2.x |
| ProxyState 内存路由表 | 避免每请求文件 IO，Provider 切换瞬时生效 |
| `tauri::async_runtime::spawn` | 避免 tokio::spawn 在 Tauri 中的静默失败问题 |
| oneshot channel 优雅关闭 | 保证进行中的 streaming 请求不被截断 |
| `patch_for_proxy` 独立方法 | 代理 patch 逻辑比直连简单，不需要 backup/rollback |
| 代理设置存 local.json | 设备级配置，不跨设备同步（与 active_providers 同层） |
| 自实现 handler 而非 axum-reverse-proxy | 动态认证注入场景，通用库反而增加复杂度 |
| Codex 用自定义 provider 而非覆盖 openai | 绕过 Codex 已知的内建 provider 覆盖限制 |
| 不在退出时恢复 CLI 配置 | 崩溃场景无法保证执行，且下次启动会自动恢复代理 |

## 来源

- [Tauri Discussion #2751: HTTP server in Tauri app](https://github.com/tauri-apps/tauri/discussions/2751) — Tauri 官方对嵌入 HTTP 服务器的建议
- [Tauri Discussion #11831: tokio::spawn silent failure](https://github.com/tauri-apps/tauri/discussions/11831) — 必须用 tauri::async_runtime::spawn
- [Tauri v2 State Management docs](https://v2.tauri.app/develop/state-management/) — 共享状态管理模式
- [Axum 0.8.0 announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) — 最新 axum 版本和路径语法变更
- [axum reverse-proxy example](https://github.com/tokio-rs/axum/blob/main/examples/reverse-proxy/src/main.rs) — 官方反向代理示例
- [axum-reverse-proxy crate](https://crates.io/crates/axum-reverse-proxy) — 评估后决定不使用
- [Claude Code LLM gateway configuration](https://code.claude.com/docs/en/llm-gateway) — Claude Code 代理要求（必须转发 anthropic-beta/anthropic-version header）
- [Codex CLI Advanced Configuration](https://developers.openai.com/codex/config-advanced/) — Codex 代理配置方式
- [Codex issue #11698: Allow overriding base URL](https://github.com/openai/codex/issues/11698) — Codex 内建 provider 覆盖限制

---
*Architecture research for: CLIManager v2.0 Local Proxy Integration*
*Researched: 2026-03-13*
