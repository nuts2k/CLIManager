# Pitfalls Research

**Domain:** 在现有 Tauri 2 桌面应用 (CLIManager v2.0) 中添加本地 HTTP 代理/API 网关
**Researched:** 2026-03-13
**Confidence:** HIGH（结合 Tauri GitHub Issues、axum/hyper 官方讨论、macOS 防火墙文档、以及 CLIManager 现有代码架构分析）

## Critical Pitfalls

### Pitfall 1: macOS 防火墙弹窗 "允许传入连接"（Allow Incoming Connections）

**What goes wrong:**
用户启动代理服务后，macOS 弹出 "你要让应用 'CLIManager' 接受传入网络连接吗？" 提示框。这在开发期间每次重新编译都会触发（因为二进制签名变了），在正式发布版本中也可能因签名问题出现。用户如果点击"拒绝"，代理服务将无法工作但可能没有明确的错误提示。

**Why it happens:**
macOS 应用防火墙按可执行文件签名追踪网络监听权限。当程序绑定到 `0.0.0.0:PORT` 或 `:PORT`（所有网络接口）时，防火墙会拦截并弹窗。每次重新编译生成新二进制文件时，之前的授权失效。这是 macOS Ventura/Sonoma/Sequoia 上的已知行为，多个开源项目都报告过此问题（[gRPC Go PR #1499](https://github.com/grpc/grpc-go/pull/1499)、[Tailscale Issue #7752](https://github.com/tailscale/tailscale/issues/7752)）。

**How to avoid:**
- **必须绑定到 `127.0.0.1:PORT` 而不是 `0.0.0.0:PORT`**。本地代理只需要本机访问，绑定 localhost 完全可以避免防火墙弹窗。这是被 gRPC、Tailscale 等项目验证过的方案。
- 正式发布版本需正确代码签名（`codesign`）。macOS 默认选项"自动允许已签名的软件接收传入连接"会放行有效签名的应用。
- 如果用户点击了"拒绝"，应用需要能检测到 bind 失败并给出明确的错误提示，引导用户到 系统设置 > 网络 > 防火墙 中手动允许。
- Tauri 打包时确保 `Entitlements.plist` 包含 `com.apple.security.network.server` 权限（用于监听端口）。

**Warning signs:**
- 开发期间每次 `cargo tauri dev` 都弹防火墙提示。
- 用户报告"代理模式开启但 CLI 连接超时"（实际是防火墙拦截了）。
- CI/CD 构建的包在新机器上首次运行时出现问题。

**Phase to address:**
Phase 1（代理服务基础架构）-- 第一次绑定端口时就必须使用 `127.0.0.1`，不能推迟修复。

---

### Pitfall 2: 代理服务无法随应用正确退出（端口泄漏 / 僵尸进程）

**What goes wrong:**
用户通过 Cmd+Q、托盘菜单退出、或直接 Force Quit 关闭 CLIManager 后，内嵌的 HTTP 代理服务没有正确关闭。端口仍被占用，下次启动应用时 bind 失败报 "Address already in use"。或者更隐蔽的情况：tokio runtime 的后台任务持续运行，导致进程无法真正退出。

**Why it happens:**
Tauri 在 macOS 上有已知的生命周期管理缺陷：
1. **`MenuItem::Quit` 直接调用 `exit(0)`**，跳过所有清理逻辑（[Issue #7586](https://github.com/tauri-apps/tauri/issues/7586)）。
2. **`RunEvent::ExitRequested` 在 macOS Cmd+Q 时不一定触发**（[Issue #9198](https://github.com/tauri-apps/tauri/issues/9198)）。
3. **`app.exit()` 内部调用 `std::process::exit()`**，不会执行正常的 Rust drop 析构（[Discussion #4662](https://github.com/tauri-apps/tauri/discussions/4662)）。
4. 操作系统关机/注销时也不触发 `RunEvent::Exit`（[Issue #14558](https://github.com/tauri-apps/tauri/issues/14558)）。

CLIManager 现有代码中 `app.run(|_app_handle, _event| {})` 没有处理任何 `RunEvent`。`std::mem::forget(debouncer)` 表明现有架构已接受"不做清理就退出"的模式，但对于持有端口的 HTTP 服务来说这不可接受。

**How to avoid:**
- **启动时先检测端口可用性**：在 bind 之前用 `TcpListener::bind()` 测试端口，如果被占用则报明确错误。不要默默失败。
- **实现"防卫性启动"模式**：如果端口被上一次异常退出的进程占用，提供"强制释放"选项（通过 `lsof -i :PORT` 检查并提示用户）。
- **使用 `tokio::sync::watch` 或 `tokio::sync::broadcast` 发送关闭信号**：axum 的 `with_graceful_shutdown` 接收一个 `Future`，用 `watch::Receiver` 实现。在托盘 "退出" 菜单项中先发送关闭信号，等待代理服务确认关闭后再调用 `app.exit(0)`。
- **设置关闭超时**：给 graceful shutdown 一个合理的超时（例如 5 秒）。超时后强制退出，不要让用户等待。
- **将代理服务句柄存入 Tauri Managed State**：`app.manage(Arc<Mutex<Option<ProxyHandle>>>)`，退出时从 state 取出句柄执行 shutdown。
- **重写托盘退出逻辑**：不再直接调用 `app.exit(0)`，而是先触发代理关闭 -> 等待关闭完成或超时 -> 然后 `app.exit(0)`。
- **在 `app.run()` 回调中处理 `RunEvent::ExitRequested`**：作为兜底，在这里也尝试关闭代理。但注意这不是 100% 可靠的（见上述 macOS 问题）。

**Warning signs:**
- 第二次启动时报 "Address already in use" 错误。
- `lsof -i :PORT` 显示已退出的 CLIManager 仍占用端口。
- 用户报告退出后 CPU 占用不降。

**Phase to address:**
Phase 1（代理服务基础架构）-- 生命周期管理是代理服务的基础设施，不能等到后期。

---

### Pitfall 3: SSE 流式响应代理时数据丢失或延迟堆积

**What goes wrong:**
CLI 通过代理发送 AI API 请求后，收到的 SSE 事件要么延迟几十秒才一次性到达（buffer flush），要么在中途断开后丢失后续内容。在代理模式下 CLI 的"打字机效果"消失，变成长时间等待后一次性输出全部内容。

**Why it happens:**
SSE 流式代理是反向代理领域中最容易出错的场景之一：

1. **中间件压缩层缓冲**：如果在 axum 路由上启用了 `CompressionLayer`（tower-http），压缩编码器会缓冲数据直到达到一定大小才 flush。`text/event-stream` 响应不应被压缩（[axum Discussion #2728](https://github.com/tokio-rs/axum/discussions/2728)）。

2. **响应体类型不匹配**：hyper v1 中 `Client` 返回 `Response<Incoming>`，但 axum handler 期望 `Response<Body>`。如果处理不当，可能触发整体缓冲而非流式转发。

3. **上游连接空闲超时**：AI API（特别是 Claude）的长思考期间可能 30-60 秒无数据。如果代理层没有 TCP keep-alive，中间网络设备可能切断连接。Claude API 官方建议设置 TCP keep-alive（[Anthropic Errors 文档](https://docs.claude.com/en/api/errors)）。

4. **Content-Length vs Transfer-Encoding**：SSE 响应应使用 `Transfer-Encoding: chunked`（无 Content-Length）。如果代理无意中添加了 Content-Length header 或去掉了 chunked 编码，客户端会等待完整响应。

5. **reqwest 客户端流式传输未启用**：reqwest 需要启用 `stream` feature 并使用 `bytes_stream()` 才能逐块转发，否则会将整个响应缓冲到内存后再发送（[reqwest 文档](https://docs.rs/reqwest/latest/reqwest/struct.Response.html)）。

**How to avoid:**
- **SSE 路由不加压缩**：如果使用 `CompressionLayer`，对 `/v1/messages`（Claude）和 `/v1/chat/completions`（OpenAI）路由排除压缩。
- **使用 `StreamBody` 转发**：从 reqwest 获取 `bytes_stream()`，包装为 `http_body_util::StreamBody`，直接作为 axum 响应体返回。不要先 `collect()` 再发送。
- **正确转发 headers**：确保上游的 `Content-Type: text/event-stream`、`Transfer-Encoding: chunked` 被正确传递到下游，同时剥离 hop-by-hop headers（`Connection`、`Transfer-Encoding` 等需按 HTTP 规范处理）。
- **设置 TCP keep-alive**：在 reqwest 客户端配置中启用 `tcp_keepalive(Duration::from_secs(30))`。
- **为代理连接设置合理的读超时**：AI 完成请求可能持续 5-10 分钟。reqwest 的 `timeout()` 是整体超时，对长流式响应不适用。应使用 `read_timeout()` 检测"无数据到达"（例如 120 秒无数据则断开重试）。

**Warning signs:**
- 代理模式下 CLI 输出是"一坨一坨"出现而非逐字出现。
- 非代理模式（直连）下 CLI 流式输出正常。
- 长任务在代理模式下报超时错误。

**Phase to address:**
Phase 2（请求转发与 SSE 流式代理）-- 这是代理核心功能，必须在实现转发时就正确处理。

---

### Pitfall 4: 代理模式下切换 Provider 时的在途请求处理

**What goes wrong:**
用户在 CLI 正在进行 AI 对话（有一个长时间运行的 SSE 流式请求正在传输）时切换 Provider。代理将新请求发往新 Provider，但旧的在途请求可能出现以下情况之一：(1) 突然断开导致 CLI 报错；(2) 继续使用旧 Provider 的凭据完成传输（正确行为）；(3) 混用新旧 Provider 的凭据导致认证失败。

**Why it happens:**
这是本项目独有的核心复杂性：代理需要在运行时动态切换上游目标。`reqwest::Client` 的配置（包括 proxy、base_url 等）在创建时就固定了，无法修改。如果使用共享的上游配置（如 `Arc<RwLock<UpstreamConfig>>`），切换时在途请求读到的配置可能是新的也可能是旧的，取决于读锁的时序。

**How to avoid:**
- **关键原则：在途请求应使用建立时的 Provider 完成，新请求才使用新 Provider**。这是代理切换的"读取时快照"语义。
- **实现方式**：在请求进入代理时立即读取当前活跃 Provider（snapshot），将 `(api_key, base_url, model)` 绑定到该请求的上下文中。后续转发用这个 snapshot，不再读共享状态。
- **不要用全局共享的 `reqwest::Client`**：为每个 Provider 维护独立的 reqwest Client（或使用同一个 Client 但在每个请求中动态设置 URL 和 headers）。推荐后者以利用连接池。
- **使用 `ArcSwap<UpstreamConfig>` 或 `tokio::sync::watch`**：`ArcSwap` 提供 lock-free 的原子替换，读操作几乎无开销。请求进入时调用 `config.load()` 获取当前配置的 `Arc` 副本，该副本在请求生命周期内不变。Provider 切换时调用 `config.store(new_config)` 替换，不影响已经持有旧 `Arc` 的在途请求。
- **不需要特殊的"排空在途请求"逻辑**：因为切换只改变配置指针，旧请求持有旧配置的 Arc 引用，自然会用旧配置完成。这比 graceful drain 简单得多。

**Warning signs:**
- 切换 Provider 后正在进行的对话突然报 401 认证错误。
- 切换后新请求仍然使用旧 Provider 的凭据。
- 切换操作导致 CLI 崩溃或无响应。

**Phase to address:**
Phase 3（动态上游切换）-- 这是代理模式区别于直连模式的核心能力。

---

### Pitfall 5: 双模式（直连 vs 代理）切换导致 CLI 配置不一致

**What goes wrong:**
用户在直连模式和代理模式之间来回切换后，CLI 配置文件处于错误状态。常见场景：
- 从代理模式切到直连模式后，CLI 配置仍指向 `localhost:PORT`，但代理已关闭 -> CLI 请求全部失败。
- 从直连模式切到代理模式，但 CLI 配置没有更新为 `localhost:PORT` -> CLI 仍然直连上游。
- 用户在代理模式下删除了 Provider，代理对应的上游配置被清除但 CLI 配置仍指向代理端口。

**Why it happens:**
CLIManager 现在有一个 surgical patch 机制（`CliAdapter.patch()`），只修改 `ANTHROPIC_BASE_URL`（Claude）和 `base_url`（Codex）字段。添加代理模式后，这些字段的值来源变了：
- 直连模式：`base_url = provider.base_url`（如 `https://api.anthropic.com`）
- 代理模式：`base_url = http://127.0.0.1:PORT`

**这意味着模式切换本质上是一次 Provider 切换操作。** 如果把"写入正确的 base_url"和"模式切换"当作两个独立操作，就容易出现状态不一致。

**How to avoid:**
- **模式切换必须原子化执行 CLI patch**：切换到代理模式 = patch CLI 配置指向 `127.0.0.1:PORT`；切换到直连模式 = patch CLI 配置指向 Provider 的实际 `base_url`。这两步不可分。
- **复用现有 `set_active_provider` + `CliAdapter.patch()` 管线**：只需在 patch 时根据当前模式决定写入什么 `base_url`，而不是另建一套 patch 逻辑。
- **模式状态必须持久化到 `LocalSettings`**：添加 `proxy_enabled: bool`（全局）和 `per_cli_proxy: HashMap<String, bool>`。这样应用重启后知道应该恢复到什么模式。
- **应用启动时检查模式一致性**：如果上次是代理模式退出，这次启动时应先启动代理再 patch CLI。如果代理启动失败，应自动回退到直连模式并 patch CLI 指向实际上游。
- **代理关闭（无论正常还是异常）必须触发 CLI 回退 patch**：如果代理因 panic 或端口冲突停止，必须把 CLI 配置恢复为直连模式，否则 CLI 会一直尝试连接已经关闭的 localhost 端口。

**Warning signs:**
- 关闭代理后 CLI 报 "connection refused" 到 `127.0.0.1:PORT`。
- 切换模式后 CLI 的行为与预期不符（仍在直连 / 仍在走代理）。
- 应用崩溃重启后 CLI 无法连接。

**Phase to address:**
Phase 4（双模式切换与 CLI 配置联动）-- 这涉及整个 patch 管线的改造。

---

### Pitfall 6: 代理模式下 API Key 泄露到本地网络

**What goes wrong:**
代理监听 `0.0.0.0` 或局域网可达的地址时，同一网络内的其他设备可以向代理发送请求，利用代理中配置的 API key 访问 AI API，造成 API key 泄露和费用消耗。

**Why it happens:**
开发者为了方便调试可能绑定到 `0.0.0.0`，或者测试时使用了非 localhost 地址。如果忘记改回来，发布版本也会监听所有接口。加上 CLIManager 目前没有任何认证机制（也不需要，因为是本地工具），任何能连接到代理端口的进程都可以使用。

**How to avoid:**
- **强制绑定 `127.0.0.1`**：在代码中硬编码而非配置化。不提供"监听地址"配置项，从源头消除风险。
- **即使绑定 localhost，仍应验证请求来源**：检查连接的源 IP 是否为 `127.0.0.1`（通常 bind localhost 后 OS 会保证这一点，但作为防御性编程仍值得加）。
- **不在代理响应头中暴露 API key 或 Provider 信息**：确保转发上游响应时剥离可能泄露的 headers。
- **代理端口不要使用常见端口号**（如 8080、3000、5000），避免与其他开发工具冲突，同时减少被扫描到的概率。建议使用 10000+ 的端口。

**Warning signs:**
- `netstat -an | grep LISTEN` 显示代理监听 `0.0.0.0:PORT` 而非 `127.0.0.1:PORT`。
- 在同一 WiFi 网络的另一台电脑上可以 `curl http://[your-ip]:PORT/v1/models`。

**Phase to address:**
Phase 1（代理服务基础架构）-- 第一次创建监听 socket 时就必须绑定正确的地址。

---

### Pitfall 7: 现有 FSEvents 文件监听与代理服务的交互冲突

**What goes wrong:**
代理模式下 Provider 切换不再需要 patch CLI 配置文件（因为 CLI 配置始终指向 localhost:PORT，切换只改代理的上游指向）。但现有的 FSEvents watcher 在检测到 iCloud 同步的 Provider 变更后，仍会调用 `sync_changed_active_providers` 去 patch CLI 配置。如果 watcher 在代理模式下把 `base_url` patch 回了 Provider 的实际上游地址，就会破坏代理模式 -- CLI 绕过代理直连上游。

**Why it happens:**
现有 `process_events` 流程是无条件的：iCloud 文件变化 -> 读取活跃 Provider -> patch CLI 配置。它不知道当前是代理模式还是直连模式。`sync_changed_active_providers` 直接调用 `CliAdapter.patch(provider)`，用 Provider 的原始 `base_url` 覆盖 CLI 配置。

**How to avoid:**
- **`sync_changed_active_providers`（以及所有 patch 入口）必须感知当前模式**：在 patch CLI 配置时检查全局代理状态。如果代理模式开启，patch 写入的 `base_url` 应为 `http://127.0.0.1:PORT`；如果直连模式，写入 Provider 的原始 `base_url`。
- **将模式判断逻辑下沉到 adapter 层**：让 `CliAdapter.patch()` 接收一个参数指定代理模式和端口，而非在调用方分支处理。这样所有 patch 路径（手动切换、托盘切换、iCloud 同步、应用启动）都自动正确。
- **代理模式下 iCloud 同步仍然需要更新代理的上游配置**：当另一台设备同步了 Provider 变更（如改了 API key），watcher 应通知代理服务更新其内部的上游凭据，但不应修改 CLI 的 `base_url`。

**Warning signs:**
- iCloud 同步后代理模式突然失效，CLI 开始直连上游。
- 在代理模式下切换设备，CLI 配置被同步覆盖为非 localhost 地址。
- `SelfWriteTracker` 记录的 write 与预期不符。

**Phase to address:**
Phase 4（双模式切换与 CLI 配置联动）-- 需要改造整个 patch 管线，是最复杂的集成点。

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| 为每个 CLI 硬编码固定端口号 | 简单，用户知道端口 | 端口冲突无法自动解决；增加新 CLI 需改代码 | v2.0 可接受。端口号选 10000+ 范围，冲突概率低。项目说明明确要求固定端口。 |
| `std::mem::forget` 代理服务句柄 | 无需管理所有权 | 无法 graceful shutdown，端口泄漏 | **绝不可接受**。代理持有网络端口，必须可控关闭。 |
| reqwest Client 不设 timeout | 不用处理超时逻辑 | AI 长对话可能永远不返回，资源泄漏 | 绝不可接受。必须设 read_timeout（120s+）和 connect_timeout（10s）。 |
| 代理内不做请求日志 | 代码简单 | 调试困难，无法排查"为什么 CLI 报错" | v2.0 可接受（日志属于 2.x 流量监控功能）。但应预留日志 hook 点。 |
| 全局共享一个 reqwest::Client | 复用连接池 | 不同 Provider 可能需要不同 TLS 配置 | v2.0 可接受。当前所有 Provider 都是 HTTPS 到标准 API 端点，TLS 配置相同。 |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| FSEvents watcher + 代理模式 | watcher 无脑 patch CLI 配置为 Provider 原始 base_url，破坏代理模式 | patch 逻辑检查当前模式，代理模式下写入 localhost:PORT |
| 托盘 Provider 切换 + 代理模式 | 代理模式下托盘切换仍然 patch CLI 配置 | 代理模式下托盘切换只更新代理内部上游配置，不动 CLI 配置 |
| iCloud 同步 + 代理上游配置 | 同步了 Provider 的 API key 变更，但代理内部缓存的是旧 key | watcher 检测到活跃 Provider 变更时，通知代理服务刷新上游凭据 |
| `SelfWriteTracker` + 代理模式 patch | 代理模式切换 patch 了 CLI 配置（写入 localhost），但没有 record_write | 所有 CLI 配置写入操作都必须经过 SelfWriteTracker |
| 应用启动顺序 | 先 patch CLI 配置指向 localhost，再启动代理 -> CLI 在代理就绪前发请求失败 | 先启动代理 -> 确认 bind 成功 -> 再 patch CLI 配置指向 localhost |
| Tauri setup 中启动代理 | 在 `setup()` 闭包中同步启动 HTTP 服务，阻塞 Tauri 事件循环 | 使用 `tauri::async_runtime::spawn` 异步启动代理，通过 channel 回传 bind 结果 |
| 代理 + reqwest（CLIManager 已有依赖） | 代理服务器用 axum（基于 hyper），但 reqwest 也基于 hyper -> 两个 runtime？ | 不冲突。axum 和 reqwest 都使用 tokio runtime，Tauri 2 已内置 tokio。确保只有一个 tokio runtime 实例。 |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| 代理转发时整体缓冲响应体 | 长 AI 回复延迟 30 秒+ 才一次性到达 CLI | 使用 `reqwest::Response::bytes_stream()` + `StreamBody` 逐块转发 | 任何 SSE 流式响应 |
| CompressionLayer 压缩 SSE 流 | SSE 事件堆积后一次性 flush | 对 SSE 路由排除压缩，或不使用 CompressionLayer | 启用压缩中间件时 |
| 每个请求创建新 reqwest::Client | 无法复用 TCP 连接，TLS 握手开销大 | 预创建 Client 并复用。每个 Provider 切换不需要重建 Client（只需改 header 和 URL） | 高频请求时 |
| 无超时限制的代理连接 | 某个请求挂住后占用连接池、阻塞后续请求 | 设 connect_timeout(10s) + read_timeout(120s) | AI API 故障或网络异常时 |
| SSE 长连接导致连接池耗尽 | 多个 CLI 会话并发时新请求排队 | reqwest 默认连接池大小通常足够，但如果限制了 `pool_max_idle_per_host`，需要调大或不限 idle | 并发 5+ SSE 流时 |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| 代理监听 0.0.0.0 | 局域网内任何设备可使用你的 API key | 强制绑定 127.0.0.1，在代码中硬编码 |
| 代理请求/响应中记录 API key | 日志泄露凭据 | 日志中 mask API key（只显示前后几位）。v2.0 不实现日志功能则无此风险。 |
| 代理模式下 CLI 配置文件明文存储 localhost URL | 低风险。但 localhost URL 本身不泄露凭据 | 可接受。API key 仍在 Provider JSON 中（与直连模式相同安全级别） |
| 代理不校验请求路径 | 可能被利用做开放代理（如转发到任意 URL） | 代理只接受特定 API 路径（`/v1/messages`, `/v1/chat/completions` 等），其他路径返回 404 |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| 代理启动失败时无提示 | 用户以为代理在运行，实际 CLI 无法连接 | 启动失败时在 UI 显示明确错误，并自动回退到直连模式 |
| 模式切换没有确认/反馈 | 用户不确定当前是什么模式 | 在 UI 和托盘 tooltip 中显示当前模式状态 |
| 切换到代理模式后 CLI 需要重启才生效 | 影响代理模式的核心价值（无需重启 CLI） | 代理模式的价值就是不需要重启。确保 CLI 的 base_url 已被 patch，CLI 下次请求自然走代理 |
| 直连模式切换 Provider 后仍需等 CLI 重新读取配置 | 用户切换后立即输入命令但 CLI 还在用旧 Provider | 这是直连模式的固有限制，无法避免。在 UI 中提示"CLI 将在下次请求时使用新 Provider" |
| 代理端口被其他程序占用 | 应用启动时代理无法启动 | 明确的错误提示 + 提供"查看端口占用"的帮助链接。不要使用 8080/3000 等常见端口 |
| 模式切换后不回退 | 切到代理模式出错后 CLI 配置留在 localhost，直连也不工作 | 模式切换失败时自动 rollback CLI 配置到切换前状态 |

## "Looks Done But Isn't" Checklist

- [ ] **SSE 流式转发：** 验证 CLI 的逐字输出效果（打字机效果）在代理模式下与直连模式一致。不能只测试"响应是否完整"，要测试"是否实时流式到达"。
- [ ] **AI 长思考场景：** 发送一个触发长思考的请求（如复杂代码重构），确认代理在 2-5 分钟的流式传输期间不超时、不断开。
- [ ] **并发请求：** 同时运行两个 CLI 会话（如两个终端窗口各开一个 Claude Code），确认两个 SSE 流不互相干扰。
- [ ] **Cmd+Q 后端口释放：** 通过 Cmd+Q 退出应用后，用 `lsof -i :PORT` 确认端口已释放。然后重新启动应用确认可正常 bind。
- [ ] **模式双向切换：** 直连 -> 代理 -> 直连 -> 代理，每次切换后验证 CLI 请求正常工作。
- [ ] **iCloud 同步 + 代理模式：** 在另一台设备修改活跃 Provider 的 API key，验证代理服务拿到新 key（而非旧 key），同时 CLI 配置仍指向 localhost。
- [ ] **代理崩溃恢复：** 手动 kill 代理线程/进程，验证应用检测到代理停止并自动回退 CLI 到直连模式。
- [ ] **应用重启后状态恢复：** 在代理模式下退出应用，重启后验证代理自动启动且 CLI 配置正确。
- [ ] **错误路径 patch 回退：** 在 patch CLI 配置的过程中模拟失败（如磁盘满），验证不会留下半写状态。

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| 端口泄漏 / bind 失败 | LOW | `lsof -i :PORT` 找到进程 -> `kill -9 PID` -> 重启应用。可以在应用启动逻辑中自动化这一步。 |
| SSE 流缓冲延迟 | MEDIUM | 排查压缩中间件和响应体类型。将 `collect()` 改为 `bytes_stream()`，确保逐块转发。需修改代理核心转发逻辑。 |
| 模式切换状态不一致 | LOW | 手动运行 `set_active_provider` 重新 patch CLI 配置。增加启动时一致性检查作为永久修复。 |
| 在途请求凭据混乱 | MEDIUM | 实现 per-request snapshot 语义（`ArcSwap`）。需重构上游配置管理。 |
| watcher 破坏代理模式 | MEDIUM | 修改 patch 管线的所有入口点使其模式感知。需 audit 所有 `CliAdapter.patch()` 调用路径。 |
| 代理未 graceful shutdown | LOW | 添加 `watch::channel` 关闭信号 + 退出时发信号。标准 axum graceful shutdown 模式。 |
| 防火墙弹窗 | LOW | 将 bind 地址从 `0.0.0.0` 改为 `127.0.0.1`。一行代码修复。 |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| macOS 防火墙弹窗 | Phase 1: 代理基础架构 | `cargo tauri dev` 启动时不弹防火墙提示；`netstat` 确认只监听 127.0.0.1 |
| 端口泄漏 / 无法 graceful shutdown | Phase 1: 代理基础架构 | Cmd+Q 退出后 `lsof -i :PORT` 无结果；再次启动 bind 成功 |
| API key 泄露到网络 | Phase 1: 代理基础架构 | 从另一台电脑 curl 代理端口，连接被拒绝（Connection refused） |
| SSE 流式转发延迟 | Phase 2: 请求转发 | CLI 打字机效果在代理模式下与直连模式一致 |
| 在途请求凭据混乱 | Phase 3: 动态上游切换 | 在 SSE 流传输期间切换 Provider，旧流正常完成，新请求使用新 Provider |
| 双模式 CLI 配置不一致 | Phase 4: 双模式切换 | 直连 <-> 代理反复切换 10 次，每次切换后 CLI 请求均正常 |
| watcher 破坏代理模式 | Phase 4: 双模式切换 | 代理模式下触发 iCloud 同步，CLI 配置 base_url 仍为 localhost |
| 代理启动顺序 | Phase 4: 双模式切换 | 应用启动 -> 代理先就绪 -> 再 patch CLI -> CLI 首次请求成功 |

## Sources

- [Tauri Discussion #2751: HTTP server in Tauri app](https://github.com/tauri-apps/tauri/discussions/2751) -- 在 Tauri 中运行 HTTP 服务器的社区讨论和官方建议（HIGH confidence）
- [Tauri Issue #9198: ExitRequested not fired on macOS](https://github.com/tauri-apps/tauri/issues/9198) -- macOS Cmd+Q 不触发退出事件（HIGH confidence）
- [Tauri Issue #12978: Support applicationShouldTerminate for macOS](https://github.com/tauri-apps/tauri/issues/12978) -- macOS graceful shutdown 功能请求（HIGH confidence）
- [Tauri Issue #7586: Have Quit Menu Item Emit Event](https://github.com/tauri-apps/tauri/issues/7586) -- MenuItem::Quit 跳过事件（HIGH confidence）
- [Tauri Discussion #4662: app.exit() and RunEvent::Exit](https://github.com/tauri-apps/tauri/discussions/4662) -- app.exit() 不触发 RunEvent::Exit（HIGH confidence）
- [Tauri Issue #14558: Graceful shutdown on OS shutdown](https://github.com/tauri-apps/tauri/issues/14558) -- 操作系统关机不触发退出事件（HIGH confidence）
- [axum Discussion #2728: CompressionLayer buffering SSE](https://github.com/tokio-rs/axum/discussions/2728) -- 压缩中间件导致 SSE 缓冲（HIGH confidence）
- [axum Discussion #2529: Proxy with axum v0.7 and hyper v1](https://github.com/tokio-rs/axum/discussions/2529) -- 代理实现中的类型不匹配（HIGH confidence）
- [axum graceful shutdown example](https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs) -- 官方 graceful shutdown 示例（HIGH confidence）
- [Static streams for faster async proxies](https://blog.adamchalmers.com/streaming-proxy/) -- 流式代理 vs 缓冲代理的性能分析（MEDIUM confidence）
- [gRPC Go PR #1499: Fix macOS firewall dialog](https://github.com/grpc/grpc-go/pull/1499) -- 绑定 localhost 避免防火墙弹窗（HIGH confidence）
- [Anthropic API Errors Documentation](https://docs.claude.com/en/api/errors) -- Claude API 超时和流式处理建议（HIGH confidence）
- [Claude Code Issue #25979: SSE streaming stalls](https://github.com/anthropics/claude-code/issues/25979) -- Claude API 流式连接挂住问题（MEDIUM confidence）
- [Claude Code Issue #18028: API streaming stalls](https://github.com/anthropics/claude-code/issues/18028) -- 流式传输 59-138 秒延迟（MEDIUM confidence）
- [Sōzu HTTP reverse proxy](https://www.sozu.io/) -- 运行时动态配置切换不丢连接（MEDIUM confidence）
- [reqwest Response docs](https://docs.rs/reqwest/latest/reqwest/struct.Response.html) -- bytes_stream() 流式 API（HIGH confidence）
- CLIManager 现有代码架构分析：`lib.rs`、`watcher/mod.rs`、`adapter/claude.rs`、`commands/provider.rs`、`storage/local.rs` -- 现有 patch 管线和生命周期管理（HIGH confidence，直接代码审查）

---
*Pitfalls research for: CLIManager v2.0 Local Proxy*
*Researched: 2026-03-13*
