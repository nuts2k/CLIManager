# Project Research Summary

**Project:** CLIManager v2.6 流量监控
**Domain:** 本地 HTTP 代理流量监控（SQLite 持久化 + 实时日志推送 + 统计可视化）
**Researched:** 2026-03-17
**Confidence:** HIGH

## Executive Summary

CLIManager v2.6 在现有 Tauri 2 + Rust axum 代理基础上叠加流量监控能力：将代理请求的元数据（模型名、token 用量、延迟、状态码）异步持久化到本地 SQLite，通过 Tauri 事件系统实时推送到前端，并在独立页面展示日志表格和统计摘要。研究基于 cc-switch 参考实现的完整代码审阅和现有 CLIManager 代码结构分析，置信度 HIGH。核心结论是：v2.6 只需引入 2 个新 Rust crate（rusqlite 0.39 + rusqlite_migration 2.4）和 1 个前端库（recharts，P2 功能），其余能力由现有栈全覆盖。

推荐架构是"非阻塞日志管道 + 双轨数据加载"模式：proxy/handler.rs 完成响应后通过 `tokio::sync::mpsc` channel 非阻塞发送 LogEntry，独立后台 task 写入 SQLite 并 emit Tauri 事件；前端页面挂载时通过 Tauri command 拉取历史日志，再通过 event listener 追加增量。这一设计确保 SQLite 写入不影响代理延迟，同时避免前端 webview 就绪前的事件丢失问题。

最关键的技术风险集中在两点：流式 SSE token 提取时机（必须等流完全结束后才能得到完整的 input/output tokens），以及 SQLite 文件路径必须放在本地 `app_local_data_dir()` 而非 iCloud 同步目录。cc-switch 已验证了这两个问题的正确解法，v2.6 可直接复用其模式（rollup_and_prune SQL、SseUsageCollector 模式、lock_conn! 同步 Mutex 用法）。

## Key Findings

### Recommended Stack

v2.6 是对现有成熟栈的最小化扩展，不引入架构转变。现有 Tauri 2.10 / React 19 / axum 0.8 / tokio / serde 全部继续沿用，新增依赖仅 3 项（recharts 为 P2 可选）。rusqlite 0.39 bundled 内嵌 SQLite 3.51.3，无系统依赖，与 cc-switch 同款技术选型（cc-switch 用 0.31 bundled）。recharts 3.8 是 cc-switch UsageTrendChart 已验证的图表库，与 shadcn/ui + Tailwind v4 兼容，但需直接使用 `var(--chart-1)` 而非 `hsl(var(--chart-1))`。

**核心新增技术（仅 2 项必须，1 项可选）：**
- `rusqlite 0.39`（bundled）：SQLite 持久化，单连接 `Arc<std::sync::Mutex<Connection>>` 模式——cc-switch 验证 < 10 req/s 场景完全够用
- `rusqlite_migration 2.4`：Schema 版本管理——v2.6 是首次引入 SQLite，用 user_version pragma 追踪迁移版本，未来加字段安全
- `recharts ^3.8`（P2 可选）：趋势图表，React 19 已验证兼容

**明确不引入：** sqlx（async-first 复杂度不必要）、r2d2 连接池（单连接已足够）、tauri-plugin-sql（控制力弱）、@tanstack/react-virtual（< 500 条无需虚拟化）、rust_decimal（不做费用计算）。

### Expected Features

v2.6 聚焦最小可用监控，按优先级严格划分。cc-switch 的 usage 模块有约 70% 超出 v2.6 范围（费用计算、多 API 格式支持、价格表管理），v2.6 只需其核心 30%：schema 设计和 rollup_and_prune 算法。

**P1 必须交付（v2.6 Launch With）：**
- 实时请求日志表格（provider / model / status / tokens / latency / timestamp，分页 20 条，provider 筛选）——核心可观测性
- token 用量提取（非流式 + 流式 SSE，覆盖 Anthropic / OpenAI Chat Completions / OpenAI Responses API 三种协议）——监控核心指标，缺失导致大部分日志 token 显示 0
- Tauri emit 实时推送（`traffic-log` 事件）——比定时轮询体验更好（cc-switch 是轮询，v2.6 改进）
- 统计摘要卡片（总请求数、总 tokens in/out、成功率）——头部全局感知
- SQLite 持久化基础设施 + 滚动清理（24h 明细保留 + 7d 日聚合统计）——防数据膨胀

**P2 余力加入（v2.6.x）：**
- 按 Provider 聚合统计表格
- 趋势折线图/柱状图（需引入 recharts）

**明确 defer 到 v2.7+：**
- 费用估算（cost_usd）——需维护价格表，复杂度高
- 实时告警——需阈值配置 UI + macOS 通知权限
- 导出报表——文件对话框 + 格式化逻辑

**明确不做（Anti-Features）：**
- 请求/响应 body 完整记录（隐私风险 + 存储巨大）
- SQLite 放 iCloud Drive（WAL 文件与 iCloud 最终一致性语义不兼容，数据损坏经典雷区）
- 无清理策略的无限日志保留

### Architecture Approach

v2.6 新增独立的 `traffic/` Rust 模块（db.rs / logger.rs / token.rs / clean.rs）和 `commands/traffic.rs`，修改 `proxy/handler.rs` 和 `proxy/state.rs` 注入 `log_tx`，前端新增 `components/traffic/` 目录和 `TrafficPage`。整体是对现有架构的分层叠加，不需要重构现有代理逻辑。

**主要组件及职责：**
1. `traffic/db.rs` — SQLite 连接管理、schema 初始化（WAL + busy_timeout PRAGMA）、CRUD 查询
2. `traffic/logger.rs` — 后台 task：消费 `mpsc` channel → 写 SQLite → emit Tauri 事件
3. `traffic/token.rs` — 从三种 API 响应格式提取 token（流式必须等流完全结束后解析）
4. `traffic/clean.rs` — rollup_and_prune：每小时聚合 >24h 明细到 daily_rollups，清理 >7d 统计
5. `proxy/handler.rs`（修改）— 请求完成后 `log_tx.send(LogEntry)`，非阻塞 fire-and-forget
6. `TrafficPage` — 双 Tab（日志 + 统计）+ 双轨数据加载（command 初始拉取 + event 增量追加）

**关键数据结构：** `LogEntry` 含 UUID、timestamp、cli_id、provider_id、status_code、latency_ms、is_streaming、request_model、response_model、input/output_tokens、error_message。两张 SQLite 表：`request_logs`（24h 滚动明细，含 timestamp/provider_id 索引）+ `daily_rollups`（7d 聚合统计，复合主键）。

### Critical Pitfalls

1. **流式 SSE 响应链路中同步调用 SQLite（导致代理延迟增加）** — 所有 SQLite 写入必须在响应流完全结束后通过 `tokio::spawn` 派发，绝不在流传输链路中直接调用 `conn.execute()`；参考 cc-switch response_processor.rs 第 314/384 行的 spawn_log_usage 模式

2. **流式 token 在中途提取（导致 output_tokens 恒为 0）** — 必须等流完全结束后才解析：Anthropic SSE 的 input_tokens 在 `message_start`，output_tokens 在流末尾的 `message_delta`；需收集完整 events 集合后统一解析，不能逐 chunk 提取

3. **SQLite 放进 iCloud 同步目录（导致数据损坏）** — 使用 `app_local_data_dir()`（`~/Library/Application Support/`）而非 `data_dir()`；路径断言不含 `Mobile Documents` 或 `iCloud`；SQLite WAL 模式与 iCloud 最终一致性语义不兼容

4. **Tauri 事件在前端 webview 就绪前丢失（启动盲区）** — 事件仅作增量更新；页面挂载时主动通过 command 拉取历史 N 条作为初始数据；数据库是 source of truth，不依赖事件保证数据完整性

5. **`std::sync::Mutex` 持锁跨越 `.await` 点（编译 panic 或死锁）** — 使用 `Arc<std::sync::Mutex<Connection>>`（非 tokio Mutex），持锁期间只调用同步函数，不出现 `.await`；参考 cc-switch `lock_conn!` 宏模式

## Implications for Roadmap

基于依赖关系和风险分布，建议 5 阶段构建顺序（与 ARCHITECTURE.md 的 Suggested Build Order 对齐）：

### Phase 1: SQLite 基础设施
**Rationale:** 所有后端功能的零依赖基础；iCloud 路径和 WAL 配置必须第一步做对——一旦定错，恢复成本极高（PROJECT.md 明确记载的历史雷区）
**Delivers:** traffic.db 初始化、两张表 schema（含索引）、连接管理（`Arc<std::sync::Mutex<Connection>>`）、WAL + busy_timeout PRAGMA、db 路径验证（不含 iCloud）、rusqlite_migration 版本追踪
**Avoids:** Pitfall 3（iCloud 路径）、Pitfall 5（Mutex 模式确立）、Pitfall 6（schema 层 created_at 使用 INTEGER 类型）

### Phase 2: 非流式日志注入
**Rationale:** 先建立完整管道（handler → channel → logger task → SQLite → emit），用非流式路径验证所有集成点，排除集成风险后再处理技术难度最高的流式 SSE 路径
**Delivers:** ProxyState 扩展 `log_tx`、logger 后台 task（channel 消费 + SQLite 写入 + emit）、非流式 token 提取（response body usage 字段解析）、LogEntry 写入验证
**Uses:** tokio::sync::mpsc、AppHandle::emit()、rusqlite INSERT
**Avoids:** Pitfall 1（SQLite 写入在 spawn 内，不阻塞 handler）

### Phase 3: 流式 SSE Token 提取
**Rationale:** 技术难度最高的独立模块，单独一阶段便于专注；Claude Code 几乎全是流式请求，缺失会导致绝大部分日志 token 显示 0；三种协议格式差异显著，需要逐一验证
**Delivers:** 流式路径 token 提取（Anthropic 原生 SSE / OpenAI Chat Completions 转换 / OpenAI Responses API 转换），stream 完全结束后触发写入，三种协议各有集成测试覆盖
**Avoids:** Pitfall 2（等待 stream EOF 后统一解析，不在中途提取）

### Phase 4: 前端流量监控页面
**Rationale:** Phase 2 提供 Tauri 命令和事件后前端开发可完全展开；Phase 3 提供完整 token 数据让表格无空洞显示
**Delivers:** TrafficPage（独立 AppView，AppShell 扩展为三视图）、LogTable（分页 + provider 筛选）、统计摘要卡片、双轨数据加载（command 初始 + event 增量）
**Avoids:** Pitfall 4（双轨加载避免事件启动盲区）；UX Pitfalls（token 为 0 时显示"—"而非"0"，新条目追加到顶部不强制重排）

### Phase 5: 统计聚合 + 数据保留 + 可选图表
**Rationale:** 数据完整后再做聚合；rollup_and_prune SQL 基于 cc-switch usage_rollup.rs 已验证逻辑，实现风险低；recharts 趋势图为 P2，放最后不阻塞核心路径
**Delivers:** rollup_and_prune 定时任务（应用启动 + tokio::time::interval 每小时）、daily_rollups SAVEPOINT 原子聚合、StatsView 统计页（含 rollup 前后两来源 UNION 查询）、可选 recharts 趋势图
**Avoids:** Pitfall 6（定时触发而非每请求触发；created_at 用 INTEGER 类型；SAVEPOINT 保证原子性）

### Phase Ordering Rationale

- Phase 1 必须第一个：iCloud 路径、WAL 模式、Mutex 模型这三个决策一旦定错后续恢复成本均为 HIGH（无法安全在线迁移）
- Phase 2 先做非流式，Phase 3 再做流式：降低复杂度，建立可验证的完整管道后再处理 SSE 边界问题；避免在 handler 集成尚不稳定时同时调试 SSE 解析
- Phase 4 依赖 Phase 2 的 Tauri 命令和事件定义；但前端类型定义和组件骨架可在 Phase 2 同期并行
- Phase 5 依赖 Phase 1 的 schema + Phase 2/3 的数据写入；recharts 作为独立可选项不影响其他功能

### Research Flags

需要在规划阶段深化研究的 Phase：
- **Phase 3（流式 SSE Token 提取）:** 三种协议的 SSE 事件字段位置不同，且 OpenAI Responses API 可能包含 `input_tokens_details` 子对象——建议在 planning 阶段读取现有 `src-tauri/src/proxy/translate/stream.rs` 和 `responses_stream.rs` 确认转换后的 SSE event 格式，避免基于假设实现
- **Phase 5（统计来源合并）:** 统计查询需同时 UNION `request_logs`（最近 24h）和 `daily_rollups`（7d 历史）——rollup 前后统计卡片数字需保持一致，查询逻辑需要专项设计

标准模式可跳过深化研究的 Phase：
- **Phase 1:** rusqlite bundled 初始化是成熟模式，cc-switch schema.rs + dao/proxy.rs 有完整参考实现，直接复用
- **Phase 2:** channel + spawn 模式在现有代码库已有先例（proxy-mode-changed emit），参考 cc-switch response_processor.rs
- **Phase 4:** 双轨数据加载是 Tauri 标准模式，AppShell opacity 三视图扩展有现有代码可参考

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | cc-switch 同款技术（rusqlite bundled）代码级验证；recharts React 19 兼容已确认；版本号精确到最新稳定版；"不使用"列表有明确理由 |
| Features | HIGH | cc-switch 完整实现代码审阅 + CLIManager 现有代码结构逐文件分析；P1/P2/P3 优先级明确；anti-features 逻辑清晰且有 PROJECT.md 决策记录佐证 |
| Architecture | HIGH | 现有 ProxyState / handler.rs / AppShell 集成点逐一验证；LogEntry 字段结构与 cc-switch schema 对比确认；两张表 schema DDL 已设计完成含索引 |
| Pitfalls | HIGH | 6 个 Critical Pitfall 均有 cc-switch 代码行级别参考；每个 pitfall 有 warning signs、recovery cost 和 phase mapping；"Looks Done But Isn't" 检查清单 8 项 |

**整体置信度：HIGH**

### Gaps to Address

- **OpenAI Responses API 流式 token 字段确认：** PITFALLS.md 提示 `input_tokens_details` 子对象可能存在，需在 Phase 3 实现前读取 `src-tauri/src/proxy/translate/responses_stream.rs` 确认实际转换后的 SSE event 格式，避免假设导致提取遗漏
- **recharts Tailwind v4 颜色变量适配：** STACK.md 指出需用 `var(--chart-1)` 而非 `hsl(var(--chart-1))`，需在 Phase 5 引入时验证现有 shadcn/ui 主题变量是否已切换到 v4 格式（可能影响图表颜色正确性）
- **AppShell 三视图扩展布局：** 当前 opacity 双视图切换方案扩展为三视图时，需确认 Header 导航栏布局在小窗口尺寸下的表现（可能需要 icon-only 模式）

## Sources

### Primary（HIGH confidence）
- cc-switch `src-tauri/src/proxy/usage/logger.rs` — rusqlite INSERT 模式、RequestLog 字段结构
- cc-switch `src-tauri/src/database/schema.rs` — proxy_request_logs DDL、5 个索引定义、rusqlite 0.31 bundled 验证
- cc-switch `src-tauri/src/database/dao/usage_rollup.rs` — rollup_and_prune 算法、SAVEPOINT 原子性、INSERT OR REPLACE 合并、3 个单元测试
- cc-switch `src-tauri/src/proxy/response_processor.rs` — SseUsageCollector、spawn_log_usage 异步写入模式（第 314/384 行）
- cc-switch `src-tauri/src/proxy/usage/parser.rs` — 三协议 token 字段差异、from_claude_stream_events 完整 events 要求
- cc-switch `src-tauri/src/database/dao/proxy.rs` — `lock_conn!` 宏 + 同步 Mutex 用法 + 不跨 await 持锁模式
- cc-switch `src/components/usage/UsageTrendChart.tsx` — recharts AreaChart + 双 Y 轴 React 18 验证
- CLIManager `.planning/PROJECT.md` — Key Decisions：iCloud SQLite 雷区、startup 通知缓存队列设计
- [docs.rs/crate/rusqlite/latest](https://docs.rs/crate/rusqlite/latest) — 版本 0.39.0，内嵌 SQLite 3.51.3
- [docs.rs/rusqlite_migration/latest](https://docs.rs/rusqlite_migration/latest/rusqlite_migration/) — 版本 2.4.1，API 设计
- [v2.tauri.app/develop/calling-frontend/](https://v2.tauri.app/develop/calling-frontend/) — Events vs Channels 官方说明

### Secondary（MEDIUM confidence）
- [github.com/recharts/recharts/releases](https://github.com/recharts/recharts/releases) — 3.8.0（2025-03-06），React 19 兼容确认
- CLIManager 现有代码：`src-tauri/src/proxy/handler.rs`、`state.rs`、`mod.rs` — 集成点分析、ProxyState 结构
- CLIManager 现有代码：`src/components/layout/AppShell.tsx` — AppView 扩展点、opacity 切换模式

---
*Research completed: 2026-03-17*
*Ready for roadmap: yes*
