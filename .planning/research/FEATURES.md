# Feature Landscape

**Domain:** 本地 API 代理服务 — AI CLI 工具与上游 Provider 之间的透明转发层
**Researched:** 2026-03-13
**Confidence:** HIGH（基于竞品分析 cc-switch v3.9+、CLIProxyAPI、LiteLLM，以及 Claude Code / Codex 官方配置文档）

## Table Stakes

用户对"本地代理模式"的基本预期。缺少任何一项 = 功能不完整，不如不做。

| Feature | 为何必须 | 复杂度 | 备注 |
|---------|---------|--------|------|
| HTTP 反向代理核心 | 代理的本质：接收请求、转发上游、透传响应。没有它一切特性无从谈起 | Med | Rust 端用 axum 监听 + reqwest/hyper_util 转发；必须支持 SSE 流式响应逐块透传 |
| SSE 流式响应透传 | AI API 的核心交互模式是 Server-Sent Events 流式输出，非流式代理 = 无法使用 | Med | 不能缓冲整个响应再发回，需逐 chunk 透传；axum 的 `Body` 流可直接转发 |
| 请求头注入（API Key 替换） | 代理模式下 CLI 发的是占位 key，代理需要替换为真实 API key 后转发上游 | Med | 拦截请求头，替换 `x-api-key`/`anthropic-api-key`（Claude）或 `Authorization: Bearer`（Codex） |
| 按 CLI 固定端口监听 | Claude Code 和 Codex 各自需要独立端口，互不干扰 | Low | Claude Code 端口如 15800，Codex 端口如 15801；端口配置写入 local.json 不同步 |
| 代理模式下自动 patch CLI 配置指向 localhost | 开启代理后 CLI 必须知道请求应该发往 localhost:port | Med | 复用现有 `CliAdapter::patch`，base_url 改为 `http://127.0.0.1:{port}`，api_key 设置为占位值 |
| 关闭代理时还原 CLI 配置 | 关闭代理后必须恢复直连 Provider 凭据，否则 CLI 请求打到空端口 | Med | 复用现有 `CliAdapter::patch`，写回当前活跃 Provider 的真实凭据和上游 base_url |
| Provider 实时热切换（代理模式核心价值） | 代理模式的核心差异化：切换 Provider 时不动配置文件、不需重启 CLI 会话 | Med | 代理进程内部 `ArcSwap` / `RwLock` 持有当前活跃 Provider 的上游 URL 和 API key，切换只更新内存 |
| 双模式切换 UI（直连 vs 代理） | 用户需要明确知道当前处于哪种模式，并能在两者之间切换 | Low | 全局总开关放设置页；每 CLI 独立开关放对应 Tab 内显示 |
| 代理启停随应用生命周期 | 应用关闭时代理必须停止，否则 CLI 请求打到已关闭的端口会 connection refused | Med | Tauri app 退出事件中停止 HTTP server；需 graceful shutdown（tokio CancellationToken） |
| 上游不可达时透传错误 | CLI 工具需要收到有意义的错误（JSON 格式）而非空响应或连接重置 | Low | 上游返回非 2xx → 直接透传原始响应；connect 失败 → 返回 502 + 结构化 JSON 错误体 |
| 代理设置本地存储 | 代理配置（端口、开关状态）不应跨设备同步，避免端口冲突和状态紊乱 | Low | 存入 `~/.cli-manager/local.json`（扩展现有 `LocalSettings`），不放 iCloud 同步目录 |

## Differentiators

让产品在众多 CLI Provider 管理工具中脱颖而出的特性。不是必需，但能显著提升体验。

| Feature | 价值主张 | 复杂度 | 备注 |
|---------|---------|--------|------|
| 端口冲突自动检测 | 启动代理时如果端口被占用，给出清晰提示而非静默失败 | Low | 绑定前用 `TcpListener::bind` 探测；失败时 UI 提示用户改端口或关闭占用进程 |
| 代理健康自检 | 启动后自动验证代理是否可达，快速发现绑定失败 | Low | 绑定成功后向自己发一个 GET `/health`，确认监听正常 |
| 代理状态实时指示（托盘图标） | 用户无需打开主窗口即可知道代理是否运行 | Med | 托盘图标变化（如绿点 = 代理运行中）；复用现有 `TrayIconBuilder`；cc-switch 已验证此模式 |
| 自定义端口配置 | 高级用户可能有端口冲突需要改端口 | Low | 设置页或 Tab 内提供端口输入框，保存到 local.json |
| 启动时自动恢复代理状态 | 应用重启后自动恢复之前的代理开关状态，无需手动重新开启 | Low | local.json 中记录 `proxy_enabled` per CLI，app 启动时读取并自动启动代理 |
| 托盘菜单显示当前模式 | 快速查看每个 CLI 当前是"直连"还是"代理"，不必打开主窗口 | Low | 在已有的托盘菜单 Provider 列表旁标注模式（如 "[代理]"） |

## Anti-Features

明确不做的特性及原因。这些特性属于 2.x+ 全功能网关里程碑或根本不适合本产品。

| Anti-Feature | 为何不做 | 替代方案 |
|--------------|---------|---------|
| 协议转换（Anthropic <-> OpenAI 格式互转） | v2.0 是透明转发，不是协议网关。协议转换需要完整的请求/响应 schema 映射，复杂度极高 | 2.x+ 里程碑；当前每个 CLI 使用自己的原生协议，代理原样透传 |
| OAuth 桥接（如 OpenAI OAuth 转 Anthropic 协议） | OAuth 涉及浏览器重定向、token 刷新、会话管理，超出透明代理层职责 | 2.x+ 里程碑 |
| 流量监控与可视化（请求日志、token 计数、成本追踪） | 需要持久化存储层、统计聚合和专门的 UI 页面，投入巨大 | 2.x+ 里程碑 |
| 自动 Failover（故障时自动切换到备选 Provider） | 需要完整的健康检查 + 故障判定逻辑 + Provider 优先级队列 + 回退策略 | 2.x+ 里程碑；v2.0 手动切换完全够用 |
| 负载均衡 / 多 Provider 轮询 | 单用户桌面应用场景不需要负载均衡 | 不做，非目标场景 |
| 请求缓存 | AI 生成结果具有随机性，不适合缓存 | 不做 |
| 速率限制 / 配额管理 | 属于上游 Provider 侧职责，本地代理不应越权管理 | 不做 |
| 多用户认证 | 本地代理只服务于当前用户，绑定 127.0.0.1 | 不做 |
| MCP 服务器代理 | 与 Provider 代理是不同维度的功能，有独立的通信协议 | 后续独立里程碑 |
| 自定义中间件/插件系统 | 过度设计，v2.0 使用场景明确 | 不做 |
| 远程代理（非 localhost 监听） | 绑定 0.0.0.0 有安全风险（API key 暴露给局域网），且非目标场景 | 只绑定 127.0.0.1 |
| 自动寻找可用端口 | 端口不确定性会导致配置难以追踪和调试 | 固定默认端口 + 手动修改 |

## Feature Dependencies

```
HTTP 反向代理核心 → SSE 流式响应透传（流式是 AI API 的基本交互模式）
HTTP 反向代理核心 → 请求头注入 / API Key 替换
HTTP 反向代理核心 → 上游不可达错误透传

按 CLI 固定端口监听 → 端口冲突检测
按 CLI 固定端口监听 → 自定义端口配置

双模式切换 UI → 代理启停随应用生命周期
双模式切换 UI → 代理模式下自动 patch CLI 配置
双模式切换 UI → 关闭代理时还原 CLI 配置

代理模式下自动 patch 指向 localhost → Provider 实时热切换（代理模式下不再 patch 文件，改为内存切换）

代理设置本地存储 → 启动时自动恢复代理状态

代理状态实时指示（托盘） → 代理健康自检

[现有] CliAdapter::patch ← 代理模式开关触发的配置变更（复用）
[现有] LocalSettings (local.json) ← 代理设置存储（扩展字段）
[现有] TrayIconBuilder / update_tray_menu ← 托盘模式指示（扩展）
[现有] Provider 数据模型 (base_url + api_key) ← Provider 实时热切换（读取）
[现有] providers-changed 事件 ← 代理内存中的 Provider 热更新触发
[现有] SelfWriteTracker ← 代理开关触发的 CLI 配置写入（避免 FSEvents 无限循环）
[现有] watcher (FSEvents) ← 代理模式下 iCloud 同步的 Provider 变更需更新代理内存（而非 patch 文件）
```

## MVP Recommendation

按优先级排列，依据依赖关系分层：

### P1: 必做（v2.0 核心功能）

1. **HTTP 反向代理核心 + SSE 流式透传 + 请求头注入**
   - 整个功能的基础。没有它其他特性无从谈起。
   - axum HTTP server 在 Tauri Rust 后端启动，接收 CLI 请求，替换 API key 后转发给上游 Provider，SSE 流式响应逐块透传。
   - 含 `/health` 端点用于自检。

2. **按 CLI 固定端口监听 + 端口冲突检测**
   - Claude Code 和 Codex 各自一个端口。端口冲突检测是防止启动失败后用户困惑的最小安全网。

3. **代理模式下自动 patch CLI 配置 + 关闭时还原**
   - 复用现有 `CliAdapter` 系统。开启代理时 base_url 改为 localhost:port + 占位 key；关闭时恢复真实 Provider 凭据。

4. **Provider 实时热切换（代理模式核心价值）**
   - 代理进程内用 `ArcSwap` 持有当前 Provider 的上游信息。切换 Provider 时只更新内存，CLI 完全无感知、无中断。

5. **双模式切换 UI + 代理启停 + 设置存储**
   - 全局总开关在设置页；每 CLI 独立开关在对应 Tab 内。
   - 状态存 local.json。应用退出时 graceful shutdown。

6. **上游错误透传**
   - 上游不可达时返回 502 + JSON 错误体，而非让 CLI 收到 connection refused。

### P2: 应做（v2.0 完善体验）

- **启动时自动恢复代理状态** — 便利性功能，避免每次重启应用后手动开启代理
- **代理健康自检** — 启动后快速确认绑定成功

### P3: 延后（v2.0.x 或 v2.1）

- **代理状态托盘图标** — 有价值但非 MVP；可在 v2.0 完成后快速迭代
- **托盘菜单显示当前模式** — 锦上添花
- **自定义端口配置** — 默认端口先用着，高级用户需求后续加

## Detailed Feature Notes

### HTTP 反向代理核心

**请求流转示意：**
```
Claude Code → POST http://127.0.0.1:15800/v1/messages
  代理服务拦截请求
  → 读取内存中当前活跃 Provider 的 api_key 和 base_url
  → 替换 Authorization / x-api-key 头为真实 API key
  → 转发到上游，如 https://api.anthropic.com/v1/messages
  → 上游 SSE 响应逐 chunk 透传回 Claude Code
```

```
Codex → POST http://127.0.0.1:15801/v1/responses
  代理服务拦截请求
  → 读取内存中当前活跃 Provider 的 api_key 和 base_url
  → 替换 Authorization: Bearer 头为真实 API key
  → 转发到上游，如 https://api.openai.com/v1/responses
  → 上游 SSE 响应逐 chunk 透传回 Codex
```

**关键技术点：**
- Claude Code 通过 `ANTHROPIC_BASE_URL` 环境变量（写入 `settings.json` 的 `env` 块）指定 base URL
- Codex 通过 `base_url` 配置项（在 `config.toml` 的 `model_providers` 表中）或 `OPENAI_BASE_URL` 环境变量指定
- 两者的 API key 在不同的 header 中：Claude 用 `x-api-key` 或 `anthropic-api-key`，Codex 用 `Authorization: Bearer ...`
- SSE 流式响应必须逐块转发，不能缓冲整个响应后再发回
- 代理必须透传所有请求路径（如 `/v1/messages`、`/v1/responses`），不做路径重写

**信心来源：**
- cc-switch v3.9+ 已验证 Claude Code -> localhost -> 上游 的架构可行 [HIGH]
- Claude Code 官方文档确认 `ANTHROPIC_BASE_URL` 支持自定义端点 [HIGH]
- Codex 官方文档确认 `base_url` 和 `OPENAI_BASE_URL` 支持 localhost 端点 [HIGH]
- axum 官方示例提供了完整的反向代理模式参考 [HIGH]

### 双模式切换

**直连模式（现有行为，不变）：**
- `CliAdapter::patch` 将真实 Provider 凭据（api_key, base_url）直接写入 CLI 配置文件
- CLI 直接请求上游 API
- 切换 Provider 需要改文件，正在运行的 CLI 会话可能需要重启或手动刷新

**代理模式（新增）：**
- `CliAdapter::patch` 将 base_url 改为 `http://127.0.0.1:{port}`，api_key 写入固定占位值
- CLI 请求打到本地代理
- 切换 Provider 只更新代理内存中的上游信息，CLI 完全无感知、无中断

**模式切换状态机：**
1. 用户开启代理 → 启动 HTTP server → patch CLI 配置指向 localhost → 代理加载当前活跃 Provider
2. 用户在代理模式下切换 Provider → 只更新内存中的上游信息（不改 CLI 配置文件）
3. 用户关闭代理 → 停止 HTTP server → 用当前活跃 Provider 的真实凭据 patch 回 CLI 配置
4. 用户在直连模式下切换 Provider → 使用现有 surgical patch 行为（行为不变）

### API Key 处理策略

**代理模式下的 API Key 流转：**
```
CLI 配置中的占位 key → CLI 请求带占位 key → 代理拦截 → 替换为真实 key → 发往上游
```

**占位 key 设计：**
- 不能用空字符串（CLI 可能报错拒绝启动）
- 不能用全零或明显无效格式（某些 CLI 会在发请求前做格式校验）
- 建议用固定前缀 `cli-manager-proxy-xxxx`，格式上模拟真实 key 的长度和前缀模式
- Claude Code 占位: `sk-ant-cli-manager-proxy-placeholder-key`
- Codex 占位: `sk-cli-manager-proxy-placeholder-key`

**安全优势：**
- 真实 API key 只在代理进程内存中，不需要写入 CLI 配置文件
- 代理只绑定 127.0.0.1，外部无法访问
- 这比直连模式更安全（直连模式下 API key 以明文存在 CLI 配置文件中）

### 端口策略

**默认端口分配：**
- Claude Code: 15800
- Codex: 15801

**端口选择依据：**
- cc-switch 用 15721（曾因 macOS AirPlay Receiver 冲突从 5000 改过来）
- 15800-15899 范围不常见冲突，且直观好记
- 每个 CLI 独立端口，便于独立启停和故障隔离
- 不用 1024 以下端口（需 root 权限）

**端口冲突处理流程：**
1. 启动前 `TcpListener::bind("127.0.0.1:{port}")` 探测
2. 绑定失败 → 返回错误给 UI，提示"端口 {port} 被占用"
3. 不自动寻找可用端口（会导致配置不确定性，调试困难）
4. 用户可在设置中修改端口

### 与现有系统的集成点

| 现有模块 | 代理功能如何集成 | 改动量 |
|---------|----------------|--------|
| `CliAdapter::patch` | 代理模式开启/关闭时复用，修改 base_url + api_key | 无需改动，直接调用 |
| `LocalSettings` (local.json) | 扩展字段：`proxy_global_enabled`、`proxy_cli_settings: HashMap<String, ProxyCliConfig>` | 小改动，新增字段 |
| `TrayIconBuilder` / `update_tray_menu` | 菜单项中标注模式、新增代理启停操作（P3 延后） | 小改动 |
| `providers-changed` 事件 | 代理模式下监听此事件，更新内存中的上游 Provider 信息 | 新增监听逻辑 |
| `SelfWriteTracker` | 代理模式开关触发的 CLI 配置写入需纳入自写追踪 | 无需改动，现有机制自动生效 |
| `watcher` (FSEvents) | 代理模式下 iCloud 同步的 Provider 变更需特殊处理：更新代理内存而非 patch 文件 | 中等改动，需条件分支 |
| `commands/provider.rs` (`set_active_provider`) | 代理模式下切换 Provider 的逻辑分支：更新代理内存而非 patch 文件 | 中等改动，需模式判断 |
| 前端 `ProviderTabs` | Tab 内新增代理开关 UI 组件 | 小改动 |
| 前端 `SettingsPage` | 新增全局代理总开关 section | 小改动 |

### Claude Code 配置可靠性注意事项

Claude Code 在 v2.0.x 系列中对 `settings.json` 的 `env` 块读取存在已知 bug：

1. **v2.0.65+ 首次安装问题**：如果用户从未运行过旧版本（无 `~/.claude.json`），首次运行时不会读取 `settings.json` 中的环境变量。解决方案：确保占位 key 不会触发登录流程，或在文档中提示用户先登录一次。
2. **v2.0.1 环境变量优先级回归**：环境变量不再正确覆盖 `settings.json`。对代理模式影响不大（我们通过 `settings.json` 设置，不依赖 shell 环境变量覆盖）。
3. **v2.0.7x env 加载变更**：建议标准化使用 shell profile 环境变量。但对我们的场景，通过 `settings.json` 的 `env` 块设置 `ANTHROPIC_BASE_URL` 是最可靠的（与 Codex 的 `config.toml` patch 一致，都是文件级别的修改）。

**结论：** 通过 `settings.json` 的 `env.ANTHROPIC_BASE_URL` 设置代理地址是可行且可靠的，但需要在集成测试中覆盖首次安装场景。[HIGH confidence]

## Sources

- [cc-switch (farion1231)](https://github.com/farion1231/cc-switch) — 竞品参考，v3.9+ 实现了本地代理 + 热切换 + 故障检测 + 断路器 [HIGH]
- [CLIProxyAPI](https://github.com/router-for-me/CLIProxyAPI) — CLI 代理方案参考，Go 实现的多 Provider 代理 [MEDIUM]
- [ai-cli-proxy-api / OmniRoute](https://github.com/ben-vargas/ai-cli-proxy-api) — 多 Provider 智能路由网关参考 [MEDIUM]
- [Claude Code 企业网络配置文档](https://docs.anthropic.com/en/docs/claude-code/corporate-proxy) — ANTHROPIC_BASE_URL 官方文档 [HIGH]
- [Codex 高级配置文档](https://developers.openai.com/codex/config-advanced/) — OPENAI_BASE_URL 官方文档 [HIGH]
- [Codex 配置参考](https://developers.openai.com/codex/config-reference/) — model_providers / base_url 配置结构 [HIGH]
- [Codex 配置示例](https://developers.openai.com/codex/config-sample/) — config.toml 示例 [HIGH]
- [axum-reverse-proxy crate](https://crates.io/crates/axum-reverse-proxy) — Rust 反向代理库参考 [MEDIUM]
- [axum reverse proxy 官方示例](https://github.com/tokio-rs/axum/blob/main/examples/reverse-proxy/src/main.rs) — axum 转发模式参考 [HIGH]
- [LiteLLM](https://github.com/BerriAI/litellm) — AI API 网关领域标杆，功能边界参考 [MEDIUM]
- [Claude Code settings.json Bug #8500](https://github.com/anthropics/claude-code/issues/8500) — 环境变量优先级问题 [HIGH]
- [Claude Code settings.json Bug #13827](https://github.com/anthropics/claude-code/issues/13827) — 首次安装时 settings.json 不生效 [HIGH]
- [ProxyTray (Windows)](https://github.com/Lingxi-Li/ProxyTray) — 托盘代理切换 UX 模式参考 [LOW]
- [menubar-proxy-switch (macOS)](https://github.com/dddd-zdf/menubar-proxy-switch) — macOS 状态栏代理切换参考 [LOW]
- [Kong AI Gateway + Codex 集成指南](https://developer.konghq.com/how-to/use-codex-with-ai-gateway/) — Codex 代理集成模式参考 [MEDIUM]

---
*Feature research for: Local Proxy Service (v2.0 milestone)*
*Researched: 2026-03-13*
