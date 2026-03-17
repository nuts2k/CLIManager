# Pitfalls Research

**Domain:** 为现有 Tauri 2 本地 HTTP 代理应用添加流量监控（SQLite 持久化、实时日志流、统计可视化）
**Researched:** 2026-03-17
**Confidence:** HIGH（基于 cc-switch 参考实现代码审查 + 现有 CLIManager 代码结构分析 + Tauri 2 事件机制实践）

---

## Critical Pitfalls

### Pitfall 1: 在 SSE 流式响应中间插入 SQLite 写入，导致代理延迟增加

**What goes wrong:**
在 proxy_handler 的 SSE 流式透传路径中，于每个 chunk 到来时或流结束时同步调用 SQLite 写入，造成 tokio runtime 被阻塞，代理延迟从毫秒级升到数百毫秒，Claude Code 客户端出现首字节延迟或流停顿。

**Why it happens:**
rusqlite（基于 sqlite3 C 库）的所有操作都是同步阻塞调用。如果在 async 上下文中直接调用（哪怕是在 `async_stream::stream!` 宏内），实际上是在 tokio 工作线程上阻塞，影响该线程上的其他 async 任务。cc-switch 的解法（`response_processor.rs` 第 314 行和第 384 行）是：流结束后在回调里调用 `tokio::spawn` 异步派发日志写入任务，完全在流传输链路之外写库。

**How to avoid:**
- token 解析和日志写入必须在响应流完全结束之后触发，绝不在流传输链路中间调用
- 日志写入使用 `tokio::spawn` 独立派发，不 await 也不阻塞流传输
- 对 SQLite 写入考虑使用 `tokio::task::spawn_blocking`（如果写入本身需要 await 上游数据）
- 流式场景：收集所有 SSE events 到内存 buffer，流结束时统一解析 token，再派发写入

**Warning signs:**
- Claude Code 客户端报告 SSE 响应有规律性停顿（每隔固定时间卡一下）
- Tauri 后端日志出现"数据库写入"相关耗时超过 10ms 的条目
- 代理 latency_ms 测量值比直连同一上游高出 100ms 以上

**Phase to address:**
SQLite 初始化 + 日志写入架构设计阶段（handler 改造前必须确定异步写入模式）

---

### Pitfall 2: 用 Mutex 保护单个 SQLite Connection，导致写入串行化死锁

**What goes wrong:**
将 `rusqlite::Connection` 包在 `Arc<Mutex<Connection>>` 中共享给代理 handler，多并发请求同时到来时，锁竞争导致：(a) 所有日志写入串行排队，高峰期写入积压；(b) 如果 tokio 任务在 async 上下文中 await `lock()`，在 tokio 默认配置下可能触发死锁警告甚至 panic（cc-switch 用 `lock_conn!` 宏封装同步 `Mutex::lock().unwrap()`，必须保证持锁期间不跨越 await 点）。

**Why it happens:**
SQLite 默认是单写者模型（WAL 模式下可多读单写）。cc-switch 在 `proxy/response_processor.rs` 里将写入放到 `tokio::spawn` 的闭包中，并用同步 `Mutex`（非 `tokio::sync::Mutex`），持锁调用同步写入后立即释放，不跨越 await 点。这是正确的用法，但一旦有人无意中在持锁期间写了 `.await`，将导致 `std::sync::Mutex` panic（无法跨越 await 持有），或 `tokio::sync::Mutex` 的死锁。

**How to avoid:**
- SQLite 连接使用 `Arc<std::sync::Mutex<Connection>>`（非 tokio Mutex）
- 持锁期间所有操作必须是同步的，锁作用域内不出现 `.await`
- 写入必须在 `tokio::task::spawn_blocking` 内部，或在普通 `tokio::spawn` 中且持锁期间仅调用同步函数
- 开启 WAL 模式：`PRAGMA journal_mode=WAL`，允许读写并发，减少竞争
- 启用 SQLite busy_timeout：`PRAGMA busy_timeout=5000`，防止写入争用时立即报错

**Warning signs:**
- 运行时出现 "cannot recursively acquire mutex" 或 "MutexGuard cannot be sent across threads" 编译错误
- 日志写入任务堆积，内存持续增长
- 数据库文件出现 `-wal` 和 `-shm` 文件但从不清理

**Phase to address:**
SQLite 初始化阶段（数据库模块设计时确定连接模型）

---

### Pitfall 3: 在流式响应完全结束前 token 数就已解析，导致 token 计数错误

**What goes wrong:**
对于 Anthropic 原生 SSE，`input_tokens` 出现在 `message_start` 事件里，`output_tokens` 出现在流末尾的 `message_delta` 事件里。如果在流传输中途（比如收到第一个 chunk 时）就提取 token，会得到 input_tokens 有值但 output_tokens 为 0 的残缺记录。对于 OpenAI Chat Completions 转换后的 SSE，tokens 都在 `message_delta` 里，在中途提取会完全漏掉。

**Why it happens:**
cc-switch 的 `parser.rs` 中 `from_claude_stream_events` 明确要求输入所有 events 的集合，而非单个 event。cc-switch 的 `SseUsageCollector`（`response_processor.rs`）是在流完成后调用 `finish()` 才触发解析。开发者容易在"实时"模式下提前触发解析，尤其当他们想尽快写入数据库时。

**How to avoid:**
- 流式路径：收集所有 SSE events 到 `Vec<Value>` buffer，仅在流 EOF（或 `[DONE]` 信号）后一次性解析 token
- 三种协议的 token 字段位置各不同，必须分协议处理：
  - Anthropic 原生：`message_start.message.usage.input_tokens` + `message_delta.usage.output_tokens`（含 `cache_read_input_tokens`、`cache_creation_input_tokens`）
  - OpenAI Chat Completions 转 Anthropic：token 已在 `message_delta` 里（转换器层已处理），从转换后的 events 解析即可
  - OpenAI Responses API 转 Anthropic：同上，查看 `responses_stream.rs` 确认字段位置
- 非流式路径：响应 body 的顶层 `usage` 对象，Anthropic 和 Responses API 命名相同，Chat Completions 需做 `prompt_tokens`/`completion_tokens` 到 `input_tokens`/`output_tokens` 的映射

**Warning signs:**
- 日志表中大量记录 `output_tokens = 0`，但 `status_code = 200`
- 流式请求的 token 记录与非流式同模型请求差异极大
- `cache_read_tokens` 和 `cache_creation_tokens` 始终为 0

**Phase to address:**
token 提取模块设计阶段（与 SSE 收集器一起设计，不要分开实现）

---

### Pitfall 4: Tauri 事件在前端 webview 就绪前丢失，实时日志出现"启动盲区"

**What goes wrong:**
代理启动后立即开始处理请求并 `app_handle.emit("proxy-log-entry", ...)` 发射事件。React 前端 webview 可能还未挂载监听器，导致前几条日志事件永久丢失。切换到"流量监控"页面时用户发现日志为空，但数据库中明明有记录，产生困惑。

**Why it happens:**
Tauri 的 `emit` 是 fire-and-forget，没有确认机制。前端在 `useEffect` 中用 `listen()` 注册监听，这发生在 React 组件挂载之后，而代理 handler 是 Rust 侧独立 tokio 任务，不感知前端状态。CLIManager 已经遇到过类似问题（PROJECT.md 中"startup 通知缓存队列（take 语义）"设计决策），用队列缓存解决了启动通知，但日志流属于持续事件，机制不同。

**How to avoid:**
- 实时日志事件（`emit`）作为"推送增量"使用，前端挂载时主动通过 Tauri command 拉取最近 N 条（比如最近 100 条）作为初始数据
- 前端监听器注册后再请求初始数据（避免 listen 和初始 fetch 之间的竞态）
- 不依赖事件传输来保证数据完整性，数据库是 source of truth，事件只是实时刷新用
- 流量监控页面路由激活时触发一次初始加载，之后才订阅事件

**Warning signs:**
- 打开应用后立即有几条日志，但流量监控页面打开时表格为空
- 切换 Provider 后刷新页面，日志才出现
- 测试时注意到日志数量与实际请求数对不上

**Phase to address:**
前端流量监控页面初始化 + 事件订阅架构设计阶段

---

### Pitfall 5: 将 SQLite 数据库文件放在 iCloud Drive 同步目录

**What goes wrong:**
SQLite 的 WAL 模式会在数据库旁生成 `-wal` 和 `-shm` 临时文件。如果数据库文件位于 iCloud Drive，iCloud 会尝试同步这些临时文件，导致：WAL 文件在同步中被另一设备"截断"或损坏，SQLite 打开时报 `disk I/O error` 或 `database disk image is malformed`。

**Why it happens:**
PROJECT.md 的关键决策明确指出"避免 SQLite 放云盘"，这是 cc-switch 核心问题之一（iCloud 对单文件 JSON 处理最好，对数据库文件有严重风险）。但开发者在新建 SQLite 数据库时可能习惯性地把它放在 app 的数据目录，而没有检查这个目录是否在 iCloud 下。

**How to avoid:**
- 使用 `dirs::data_local_dir()` 或 `tauri::api::path::app_local_data_dir()`（而非 `data_dir`）确保在 `~/Library/Application Support/` 而非 `~/Library/Mobile Documents/`
- 明确验证数据库路径不含 `Mobile Documents` 或 `iCloud`
- 在 README 或初始化代码注释中明确声明"SQLite 路径强制使用本地存储"

**Warning signs:**
- 数据库初始化后文件路径包含 `com~apple~CloudDocs` 或 `iCloud`
- 数据库文件旁出现 `.icloud` 后缀的占位文件
- 多台 Mac 同时运行应用时，数据库偶发打开失败

**Phase to address:**
SQLite 初始化阶段（第一行代码就确定路径，后续无法安全迁移）

---

### Pitfall 6: 滚动保留逻辑不跑或跑太频繁，数据库无限增长或统计数据损坏

**What goes wrong:**
有两种对立的失败模式：
(a) 不跑 rollup：`proxy_request_logs` 无限增长，一个活跃用户一天可产生数百条记录，半年后数据库体积超过 100MB，查询变慢，启动时间增加。
(b) 跑太频繁或 cutoff 计算错误：`usage_daily_rollups` 被重复 aggregate 同一批数据，`request_count`、`input_tokens` 等统计字段翻倍；cc-switch 用 `INSERT OR REPLACE` + `LEFT JOIN` 合并来防止这个问题，但 cutoff 时间戳算法错误（如用 `DateTime::now()` 而非 `Utc::now().timestamp()`）仍会导致边界错误。

**Why it happens:**
cc-switch 的 `rollup_and_prune` 函数（`usage_rollup.rs`）使用 Unix timestamp 做 cutoff 比对，`proxy_request_logs.created_at` 列也是 Unix timestamp（INTEGER）。如果有人把 `created_at` 存成 ISO 8601 字符串，数值比较会失效，所有记录都满足 `< cutoff`，导致全量 rollup 然后删光所有详细日志。

**How to avoid:**
- `created_at` 统一使用 Unix timestamp（INTEGER 类型），在 schema 定义时明确注释
- rollup 触发时机：应用启动时检查一次 + 每隔 1 小时异步触发一次（使用 `tokio::time::interval`），不要在每次请求写入后触发
- rollup SQL 使用 `SAVEPOINT` 保证聚合和删除的原子性（cc-switch 已有此模式，直接复用）
- 有单元测试覆盖 rollup 逻辑（cc-switch 有 3 个测试，包括合并旧 rollup 的场景）

**Warning signs:**
- 数据库文件持续增大，没有上限
- 统计图表数据异常（某天 request_count 翻倍）
- 启动后 `proxy_request_logs` 表中有超过 7 天的记录

**Phase to address:**
SQLite schema + 数据保留策略阶段（与数据库初始化同步设计）

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| 直接 `Arc<Mutex<Connection>>` 不用连接池 | 实现简单，符合现有 cc-switch 模式 | 高并发写入时性能瓶颈；但本应用每秒并发请求 < 10，基本没问题 | v2.6 MVP 阶段完全可接受 |
| token 解析只处理 Anthropic 协议，不处理 OpenAI 格式 | 节省开发时间 | 使用 OpenAI Provider 时所有 token 显示为 0，用户困惑 | 不可接受，v2.6 必须覆盖三种协议 |
| 前端图表用简单 SVG 手绘而非 charting 库 | 无额外依赖 | 多数据点时性能差，PM 要求增加图表类型时需重写 | 如果数据点 < 100，短期可以接受 |
| 不加 `created_at` 索引 | 省几行 SQL | 7 天日志数据时 `WHERE created_at > ?` 全表扫描，前端加载慢 | 不可接受，索引必须在 schema 创建时就加 |
| 实时事件 emit 不带去重，前端直接追加 | 实现简单 | 多个 webview window 时重复 emit，前端同一条日志显示两次 | 单窗口场景可接受，多窗口场景需处理 |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| axum handler → SQLite | 在 `async fn proxy_handler` 中直接调用 `conn.execute()`（同步阻塞） | `tokio::spawn` 派发写入任务，handler 立即返回响应 |
| Tauri emit → React | 假设 `emit` 后前端立即可见，不加初始数据加载 | 前端挂载时通过 Tauri command 拉取初始数据，事件只做增量更新 |
| OpenAI Responses API → token | 假设 Responses API 的 `usage.input_tokens` 和 Anthropic 完全一样 | 注意 Responses API 可能包含 `input_tokens_details` 子对象，需验证实际响应格式 |
| rusqlite + tokio | 在 `async fn` 中 `await` 持有 `MutexGuard<Connection>` | 用 `std::sync::Mutex`（非 tokio），持锁期间只调用同步函数 |
| SQLite WAL + macOS | 默认 journal 模式可能在某些 macOS 版本下有锁问题 | 初始化时执行 `PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;` |
| Recharts/Chart.js in Tauri webview | 引入重型 charting 库增加 bundle size，webview 首次加载变慢 | 使用轻量库（如 Recharts 按需引入），或纯 CSS/SVG 实现简单图表 |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| 每次 UI 刷新都全量 SELECT 所有日志 | 流量监控页面打开慢，每次有新请求时 UI 抖动 | 分页加载 + 仅 emit 增量事件到前端 | 日志超过 500 条时 |
| 图表渲染不用虚拟化，直接把全部数据点传给 chart | 24 小时内数百条日志全部渲染，帧率下降 | 按时间桶聚合（5 分钟一桶），图表最多显示 288 个点 | 数据点超过 200 时 |
| 每次请求完成后立即触发 SELECT COUNT(*) 刷新统计卡片 | 大量写入时统计卡片频繁刷新，Tauri 进程 CPU 升高 | 统计卡片使用 debounce（500ms），不跟随每条 emit 刷新 | 每分钟超过 10 个请求时 |
| `SELECT *` 从 proxy_request_logs 不带 LIMIT | 返回所有列，传输大量不需要的字段 | 只 SELECT 表格展示需要的列，LIMIT 100 分页 | 日志超过 100 条时 |
| tokio::spawn 写入任务无限积压 | 内存持续增长，应用关闭时日志丢失 | 使用带容量限制的 channel（`tokio::sync::mpsc` 容量 1000）做写入队列 | 请求速率 > 100/s 时 |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| 日志记录请求 body 全文（含 API key） | API key 明文存入 SQLite | 只记录 token 统计字段，不记录 body 内容；header 中的 key 不写入日志 |
| 日志记录响应 body 内容（含用户对话） | 用户私密对话存入本地数据库 | 不记录任何 body 内容，只记录 metadata（token 数、延迟、状态码） |
| 统计接口未限制 provider_id 参数范围 | 理论上可枚举其他 Provider 的统计 | 本应用是单用户桌面应用，不存在跨用户场景，无需鉴权 |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| 流量监控页面默认显示所有时间范围数据 | 数据量大时加载慢，用户看到很多无关历史数据 | 默认显示最近 1 小时或最近 50 条，提供时间筛选器 |
| token 为 0 时直接显示"0"，不区分"未记录"和"确实是 0" | 用户误以为请求没有消耗 token | 对 `status_code != 200` 或 `input_tokens == 0 && output_tokens == 0` 的记录用灰色或"—"表示 |
| 实时日志表格每次收到新条目时闪烁/重排 | 用户正在查看日志时被打断，视觉不舒适 | 新条目追加到顶部，仅在用户已滚动到顶部时自动滚动；否则显示"有 N 条新日志"提示 |
| 统计图表不显示"无数据"时的空状态 | 用户看到空白区域，不知道是加载中还是真的没有数据 | 明确的空状态（"最近 7 天暂无代理请求"）+ 适当的加载态 |
| 清空日志按钮无二次确认 | 用户误触删除所有历史数据 | 二次确认对话框（"确认清空全部 24 小时日志？此操作不可恢复"） |
| 实时日志无法暂停 | 用户正在检查某条记录时，新记录涌入导致目标记录滚走 | 提供"暂停实时刷新"开关，暂停时新事件缓存不更新 UI |

---

## "Looks Done But Isn't" Checklist

- [ ] **SQLite 初始化：** 看起来创建了表，但未设置 WAL 模式和 busy_timeout — 验证初始化 SQL 包含 `PRAGMA journal_mode=WAL` 和 `PRAGMA busy_timeout=5000`
- [ ] **流式 token 提取：** 看起来能解析 token，但只测了非流式响应 — 验证流式路径（Anthropic 原生 SSE + OpenAI Chat Completions 转换后 + OpenAI Responses API 转换后）三种场景都有 token 记录
- [ ] **数据保留：** rollup 逻辑写好了，但没有定时触发 — 验证应用启动后有定时 rollup 任务，且重启后 7 天前的日志确实被清理
- [ ] **前端事件订阅：** 前端能收到实时事件，但没有初始数据加载 — 验证打开流量监控页面时显示历史记录，不只是等待新事件
- [ ] **iCloud 路径检查：** 数据库文件创建了，但未验证路径 — 验证 `db_path.to_string_lossy()` 不含 `Mobile Documents` 或 `iCloud`
- [ ] **错误请求日志：** 成功请求有日志，但 4xx/5xx 响应没有记录 — 验证上游返回错误时也写入一条日志（token 为 0，status_code 为实际错误码）
- [ ] **Provider 筛选：** 筛选 UI 有，但底层 SQL 的 `WHERE provider_id = ?` 没有对应索引 — 验证 `idx_request_logs_provider` 索引存在
- [ ] **统计合计：** 7 天统计看起来有数字，但 rollup 未跑时从 proxy_request_logs 计算，rollup 跑后从 usage_daily_rollups 计算，两处来源未合并 — 验证统计查询同时 UNION 两个表

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| 数据库放进了 iCloud 目录 | HIGH | 停止应用，复制数据库到本地目录，修改初始化代码，重新打包 |
| token 解析只收集了流式 events 的一部分 | MEDIUM | 修复 SSE 收集器的 finish 触发时机，历史数据无法修复但新数据正确 |
| 没有索引，查询变慢 | LOW | 在线执行 `CREATE INDEX IF NOT EXISTS ...`，SQLite 支持不停服加索引 |
| rollup 逻辑重复计算，统计数据翻倍 | HIGH | 清空 `usage_daily_rollups` 表，重新从 `proxy_request_logs`（若还有详细数据）重建 |
| 前端事件丢失启动阶段日志 | LOW | 增加初始数据加载，不影响后续事件接收 |
| SQLite 写入在流传输链路上，造成延迟 | MEDIUM | 重构写入逻辑到流结束后的 `tokio::spawn`，需修改 handler 结构 |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 流传输链路中的同步 SQLite 写入 | SQLite + 日志写入架构设计阶段 | 代理延迟测试：流式请求 latency 与直连同一上游差值 < 20ms |
| Mutex 死锁 / 持锁跨 await | SQLite 初始化阶段 | `cargo test` 全量通过 + 并发 10 个流式请求不出现 panic |
| 流式 token 解析不完整 | token 提取模块阶段 | 三协议各 1 个流式 + 1 个非流式的集成测试，token 数与实际 API 响应一致 |
| Tauri 事件丢失（启动盲区） | 前端流量监控页面初始化阶段 | 打开页面时历史日志立即显示，不依赖等待新事件 |
| SQLite 放进 iCloud 目录 | SQLite 初始化阶段 | 单元测试断言 db_path 不含 "Mobile Documents" |
| 数据无限增长 | 数据保留策略阶段 | 写入 1000 条模拟日志后，运行 rollup，验证 `proxy_request_logs` 只保留最近 24 小时 |
| 统计来源不一致（rollup 前后） | 统计查询层设计阶段 | 对比 rollup 前后统计卡片数字相同 |
| 图表渲染性能 | 前端统计可视化阶段 | 200 条日志下图表渲染时间 < 100ms，无明显帧率下降 |

---

## Sources

- cc-switch 参考代码：`src-tauri/src/proxy/response_processor.rs`（`SseUsageCollector`、`spawn_log_usage`、`log_usage_internal` 的异步写入模式）
- cc-switch 参考代码：`src-tauri/src/proxy/usage/parser.rs`（三协议 token 字段位置的差异，尤其是 `from_claude_stream_events` 要求完整 events 集合）
- cc-switch 参考代码：`src-tauri/src/database/dao/usage_rollup.rs`（滚动保留 + SAVEPOINT 原子性 + INSERT OR REPLACE 合并逻辑）
- cc-switch 参考代码：`src-tauri/src/database/schema.rs`（proxy_request_logs 表结构、5 个索引定义）
- cc-switch 参考代码：`src-tauri/src/database/dao/proxy.rs`（`lock_conn!` 宏 + 同步 Mutex 用法 + 不跨 await 持锁模式）
- CLIManager PROJECT.md：Key Decisions "每 Provider 一个 JSON 文件（iCloud 对单文件更新处理最好，避免 SQLite 放云盘的经典雷区）"
- CLIManager PROJECT.md：Key Decisions "startup 通知缓存队列（take 语义）— 解决 setup 阶段 emit 事件前端未就绪的时序问题"
- CLIManager 现有代码：`src-tauri/src/proxy/handler.rs`（现有流式透传路径，是 SQLite 写入集成的目标位置）
- CLIManager 现有代码：`src-tauri/src/proxy/translate/response.rs` 和 `responses_response.rs`（OpenAI Chat Completions 和 Responses API 的 token 字段映射，已有转换逻辑可复用）

---

*Pitfalls research for: Tauri 2 代理应用流量监控功能（SQLite 持久化 + 实时日志流 + 统计可视化）*
*Researched: 2026-03-17*
