# Feature Research

**Domain:** 本地 HTTP 代理流量监控（Tauri 2 桌面应用扩展模块）
**Researched:** 2026-03-17
**Confidence:** HIGH（基于 cc-switch 参考实现完整代码审阅 + 现有 CLIManager 代码库分析）

---

## Feature Landscape

### Table Stakes（用户默认期待的功能）

用户打开"流量监控"页面时，如果缺少以下功能，产品会感觉残缺。

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **实时请求日志表格** | 代理工具的核心可观测性，用户排查请求时第一诉求 | MEDIUM | 每次请求完成后推送到前端；含 provider、model、状态码、延迟列 |
| **token 用量列（input/output）** | AI API 用量核心指标；Claude Code 用户关心 token 消耗 | MEDIUM | 非流式从响应 `usage` 字段提取；流式 SSE 从 `message_start`/`message_delta` 两事件累加 |
| **请求状态显示（成功/失败/错误码）** | 调试首要需求，快速确认请求是否正常 | LOW | 记录 HTTP status_code + error_message；前端按 2xx/4xx/5xx 染色 |
| **Provider 筛选** | 多 Provider 切换场景下快速定位某个 provider 的请求 | LOW | 下拉过滤，前端状态过滤即可（或带参数查询 SQLite） |
| **时间戳列** | 每条日志的基本元数据 | LOW | unix timestamp 存储，前端格式化为本地时间 |
| **延迟指标（latency_ms / first_token_ms）** | 评估 provider 响应速度的标准指标 | LOW | handler.rs 在请求发出前后打时间戳即可 |
| **滚动保留策略** | 防止 SQLite 数据库无限增长 | MEDIUM | 24h 明细保留 + 7d 日聚合；定时任务执行 rollup_and_prune |
| **统计摘要卡片（总请求数、总 tokens、成功率）** | 监控页头部的全局感知，一眼掌握整体情况 | LOW | 按时间范围聚合查询 usage_daily_rollups + 最近 24h 的 proxy_request_logs |
| **流式/非流式标识** | Claude Code 几乎总是流式；区分有助于调试特定问题 | LOW | 记录 is_streaming 字段，从请求体 `stream: true` 或响应 Content-Type 判断 |

### Differentiators（差异化竞争力）

本项目核心价值在"多 Provider 快速切换"场景，监控功能对这一场景有专项增强价值。

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **按 Provider 聚合统计** | 帮助用户比较不同 provider 的性能/用量，支撑 Provider 选择决策 | MEDIUM | 查询 usage_daily_rollups 按 provider_id 分组；表格形式展示 provider stats |
| **按时间聚合趋势图（token/请求量）** | 让用户感知 token 用量趋势，判断有无异常突发 | MEDIUM | 基于 usage_daily_rollups；使用 recharts 显示折线图或柱状图 |
| **Tauri emit 实时推送（无需轮询）** | 请求完成即刻显示在表格中；比定时轮询体验好 | MEDIUM | proxy_handler 请求处理完后 emit `proxy-request-logged` 事件；前端 listen 事件 prepend 到本地状态 |
| **CLI 分组显示（claude vs codex）** | 与现有 Provider 按 CLI 分组设计一致；用户明确知道哪个 CLI 产生了哪些流量 | LOW | 日志表格增加 cli_id/app_type 列；筛选器支持按 CLI 过滤 |
| **原始模型名 vs 映射模型名** | 代理做了模型映射时，同时显示客户端原始请求模型名和实际上游模型名，帮用户验证映射是否生效 | LOW | 记录 model（上游实际值）和 request_model（客户端原始值）两字段；handler.rs 各分支均已有 request_model 变量 |
| **错误详情字段** | 上游返回 4xx/5xx 时保存 error body 摘要；排错比只看状态码更高效 | LOW | 非流式错误响应读取 body 存入 error_message；流式连接失败记录原因 |

### Anti-Features（看起来有用但应明确不做的）

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **费用估算（cost_usd 计算）** | cc-switch 有完整计费体系 | v2.6 目标是基础监控；价格表需要持续维护且精度存疑；增加大量实现复杂度 | 只记录 token 数字；v2.7+ 可选加入费用列 |
| **实时告警（用量超限推送通知）** | 防止意外高消费 | 需要阈值配置 UI + macOS 通知权限；超出 v2.6 范围 | 仅展示统计数字；超限告警为 v2.7 候选 |
| **请求 body / response body 完整记录** | 调试时想看完整内容 | 隐私风险（可能含 API key、用户数据）；存储开销巨大（一次 Claude 对话可达数十 KB） | 只记录摘要字段（model、token 数、状态码、error message） |
| **导出报表（CSV/JSON 导出）** | 离线分析 | 文件选择对话框 + 格式化逻辑复杂；v2.6 聚焦查看 | 后续里程碑可做；用户可自行查 SQLite 文件 |
| **SQLite 放 iCloud Drive** | 保持设备间同步 | iCloud 对 SQLite WAL 文件语义不兼容，导致数据损坏；PROJECT.md Key Decisions 明确记载此雷区 | SQLite 存本地 `~/.cli-manager/`，不同步 |
| **无限日志保留（无清理策略）** | 想看完整历史 | 磁盘占用无上界；活跃用户每天数十次请求无限积累不可接受 | 24h 明细 + 7d 日聚合滚动保留；覆盖主要使用场景 |
| **前端无分页无限滚动列表** | 更流畅感觉 | 全量日志常驻内存会导致内存压力和渲染卡顿；SQLite 分页查询更可控 | 分页加载（page/pageSize），默认 20 条/页；新事件 prepend 到顶部 |
| **代理 Failover / Circuit Breaker 状态集成** | cc-switch 有 circuit breaker | 超出流量监控范畴；是独立的 v3.0 网关里程碑功能 | 仅在日志表格中显示失败率；不接入 circuit breaker 状态机 |

---

## Feature Dependencies

```
[SQLite 持久化层（proxy_request_logs + usage_daily_rollups）]
    └──requires──> [rusqlite crate 引入（bundled feature）]
    └──requires──> [数据库模块 storage/db.rs 新增]
    └──requires──> [Schema 初始化（CREATE TABLE IF NOT EXISTS）]
    └──requires──> [滚动清理任务（rollup_and_prune，启动时 + 定时执行）]

[token 用量提取]
    └──requires──> [非流式分支：响应 body 解析 usage 字段]
    └──requires──> [流式分支：SSE 事件收集（message_start + message_delta 累加）]
    └──both require──> [handler.rs 改造]

[请求日志写入]
    └──requires──> [token 用量提取]
    └──requires──> [SQLite 持久化层]
    └──requires──> [ProxyState 扩展（注入 log channel sender）]
    └──requires──> [handler.rs 改造（在响应完成后写日志）]

[Tauri emit 实时推送]
    └──requires──> [请求日志写入]
    └──requires──> [AppHandle 可达（已有先例：proxy-mode-changed emit）]
    └──enhances──> [前端实时日志表格]

[流量监控前端页面（TrafficPage）]
    └──requires──> [Tauri 命令：get_request_logs（分页查询）]
    └──requires──> [Tauri 命令：get_traffic_stats（聚合统计）]
    └──enhances via──> [实时 event listener（proxy-request-logged）]
    └──requires──> [AppShell AppView 类型扩展（添加 "traffic" 视图）]
    └──requires──> [Header 导航栏新增流量监控入口]

[统计趋势图]
    └──requires──> [usage_daily_rollups 表有数据]
    └──requires──> [recharts 或 shadcn/ui charts 依赖]
    └──enhances──> [流量监控前端页面]

[Provider 筛选]
    └──requires──> [日志记录中含 provider_id 字段]
    └──can be──> [纯前端状态过滤，无需额外查询]
```

### Dependency Notes

- **handler.rs 改造**是所有后端日志功能的关键路径：在 `proxy_handler` 中注入写日志能力。推荐方案：`ProxyState` 扩展 `log_tx: Option<tokio::sync::mpsc::UnboundedSender<LogEntry>>`。handler send 到 channel（非阻塞），独立 tokio task 接收并写入 SQLite，避免阻塞请求响应路径。
- **流式 SSE token 收集**是技术难度最高的点：需在 SSE 流的末尾事件（`message_delta` 含 output_tokens；`message_start` 含 input_tokens）中累加 token 数，再在流结束后异步触发日志写入。cc-switch 使用全量事件收集器模式（`Vec<Value>`），CLIManager 应采用更轻量的做法：只提取关键字段，不存储全部事件。
- **rusqlite bundled**：Cargo.toml 目前无 SQLite 依赖，需新增 `rusqlite = { version = "0.32", features = ["bundled"] }`。bundled feature 避免 macOS 系统 sqlite3 版本差异，产物自包含。
- **SQLite 文件位置**：存放在 `~/.cli-manager/traffic.db`（或 Tauri `app_data_dir()`），与 iCloud 同步目录完全隔离，避免 iCloud 同步损坏风险（cc-switch 踩坑记录）。

---

## MVP Definition

### v2.6 交付范围（Launch With）

最小可用的流量监控，验证核心价值：

- [ ] **SQLite 数据库初始化**（proxy_request_logs + usage_daily_rollups 两表）— 基础设施，其他所有功能依赖它
- [ ] **handler.rs 日志注入**（在 proxy_handler 请求完成后异步写日志）— 核心数据采集点；不阻塞请求响应
- [ ] **token 用量提取（非流式）**（解析 response body 中的 usage 字段）— 监控核心指标
- [ ] **token 用量提取（流式 SSE）**（从 SSE 末尾事件累加 input/output tokens）— Claude Code 几乎全是流式请求，缺失会导致大部分日志 token 显示为 0
- [ ] **Tauri emit 实时推送**（`proxy-request-logged` 事件）— 实时性体验核心
- [ ] **独立流量监控页面（TrafficPage）**（AppShell 新增第三个 AppView）— 功能入口
- [ ] **实时日志表格**（provider/model/status/tokens/latency/timestamp 列，支持 provider 筛选，分页 20 条）— 核心用户需求
- [ ] **统计摘要卡片**（总请求、总 tokens in/out、成功率）— 头部概览
- [ ] **滚动清理定时任务**（应用启动时 + 定时每小时执行 rollup_and_prune，24h 明细 → 7d 日聚合）— 防数据膨胀

### 验证后加入（v2.6.x 补丁）

功能核心验证后、如有余力：

- [ ] **按 Provider 聚合统计表格**（触发条件：用户反馈需要比较 provider 性能）
- [ ] **趋势折线图/柱状图**（触发条件：用户有日均分析需求；需引入 recharts）

### 后续里程碑考虑（v2.7+）

- [ ] **费用估算（cost_usd）**— 需要维护价格表；v2.7 候选
- [ ] **实时告警**— 需要阈值配置 UI；v2.7 候选
- [ ] **导出报表**— v2.7 候选

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| SQLite 数据库初始化 | HIGH | LOW | P1 |
| handler.rs 日志注入基础架构 | HIGH | MEDIUM | P1 |
| token 用量提取（非流式） | HIGH | LOW | P1 |
| token 用量提取（流式 SSE） | HIGH | HIGH | P1（关键路径，技术难度高）|
| Tauri emit 实时推送 | HIGH | LOW | P1 |
| 实时日志表格（分页+筛选） | HIGH | MEDIUM | P1 |
| 统计摘要卡片 | MEDIUM | LOW | P1 |
| 滚动清理定时任务 | HIGH | MEDIUM | P1 |
| 按 Provider 聚合统计 | MEDIUM | LOW | P2 |
| 趋势图（recharts） | MEDIUM | MEDIUM | P2 |
| 费用估算 | LOW | HIGH | P3 |
| 导出报表 | LOW | MEDIUM | P3 |
| 实时告警 | MEDIUM | HIGH | P3 |

**Priority key:**
- P1: v2.6 必须交付
- P2: v2.6 余力加入 / v2.6.x 补丁
- P3: 明确 defer 到 v2.7+

---

## Competitor Feature Analysis

cc-switch 是本项目主要参考，其流量监控实现有以下特点与教训：

| Feature | cc-switch 实现 | CLIManager v2.6 方案 |
|---------|---------------|---------------------|
| 日志存储 | SQLite（proxy_request_logs 表，字段完整，含 cost_usd）| 相同表结构，简化版（不含 cost_usd；无 cost_multiplier） |
| 数据库位置 | 应用数据目录（不在 iCloud）| 相同，`~/.cli-manager/traffic.db` |
| token 提取 | 独立 usage/parser.rs 模块，支持 Claude/OpenRouter/Codex/Gemini | 只需支持 Anthropic + OpenAI 两格式（handler.rs 现有三分支对应不同格式） |
| 费用计算 | 完整 calculator.rs + model_pricing 表 + rust_decimal | **明确 defer**，v2.6 不做费用计算 |
| 前端组件 | UsageDashboard + RequestLogTable + ProviderStatsTable + ModelStatsTable + UsageTrendChart | 简化版：单页面 + 日志表格 + 基础摘要卡片；趋势图为 P2 |
| 实时推送 | 无（依赖定时轮询 refetchInterval，cc-switch 前端需轮询）| **改进**：Tauri emit 事件推送，体验更好 |
| 日聚合 | usage_daily_rollups + rollup_and_prune | 直接复用相同逻辑和 SQL |
| 数据库迁移 | user_version 版本迁移系统（6 个版本） | 简单 CREATE TABLE IF NOT EXISTS；v2.6 是 SQLite 首次引入，无需迁移系统 |
| 数据库依赖 | rusqlite + rust_decimal | rusqlite（bundled）；不引入 rust_decimal（无费用计算需求） |

cc-switch 的 usage 模块过度复杂（完整费用计算、多 API 格式、价格表管理），CLIManager v2.6 只需其中约 30% 的功能。核心复用价值：schema 设计（proxy_request_logs / usage_daily_rollups 两表结构）和 rollup_and_prune 算法（SQL 已验证正确）。

---

## 技术依赖说明

### 新增 Rust 依赖（Cargo.toml）

| Crate | 作用 | 版本建议 |
|-------|------|---------|
| `rusqlite` | SQLite 访问，bundled 自包含 | `0.32`，features = ["bundled"] |

其余现有依赖满足需求：chrono（时间戳）、uuid（request_id）、serde/serde_json（序列化）、tokio（异步写日志 channel）。

### 前端依赖

P1 功能：日志表格使用现有 shadcn/ui Table 组件，无需额外依赖。

P2 趋势图需要图表库：
- `recharts`（cc-switch 在用，React 生态主流，4.9 版本）— 推荐，成熟稳定
- 或 shadcn/ui charts（基于 recharts 封装）— 与设计体系高度一致

### handler.rs 注入方案

推荐 channel 解耦方案：扩展 `ProxyState`，新增 `log_tx: Option<tokio::sync::mpsc::UnboundedSender<LogEntry>>`。

handler 写日志时向 channel 做非阻塞 send（不阻塞请求响应路径），独立 tokio task 消费 channel 并批量写入 SQLite（串行化，避免并发写冲突）。AppHandle 通过同一 task 在写入后 emit 事件到前端。

### 流式 token 收集方案

在 stream.rs 和 responses_stream.rs 的转换 stream 中，在流结束时向 `log_tx` 发送包含累积 token 数的 `LogEntry`。累积策略：
- Anthropic 格式：`message_start` 事件提取 `input_tokens`；`message_delta` 事件提取 `output_tokens`
- OpenAI Chat Completions 格式：最后一个含 `usage` 字段的 SSE chunk 提取 `prompt_tokens` / `completion_tokens`
- OpenAI Responses API 格式：`response.completed` 事件提取 `usage`

---

## Sources

- cc-switch 参考代码：`cc-switch/src-tauri/src/database/schema.rs`（proxy_request_logs + usage_daily_rollups 表结构 — 完整字段审阅）
- cc-switch 参考代码：`cc-switch/src-tauri/src/database/dao/usage_rollup.rs`（rollup_and_prune 算法 + 完整测试覆盖）
- cc-switch 参考代码：`cc-switch/src-tauri/src/proxy/usage/parser.rs`（token 提取逻辑 — Claude/OpenRouter 格式）
- cc-switch 参考代码：`cc-switch/src-tauri/src/proxy/usage/logger.rs`（RequestLog 结构体 + 写入逻辑）
- cc-switch 参考代码：`cc-switch/src/types/usage.ts`（前端类型定义）
- cc-switch 参考代码：`cc-switch/src/lib/api/usage.ts`（Tauri invoke 命令列表）
- cc-switch 参考代码：`cc-switch/src/components/usage/UsageDashboard.tsx`（前端组件结构）
- cc-switch 参考代码：`cc-switch/src/components/usage/RequestLogTable.tsx`（日志表格组件）
- 现有代码：`src-tauri/src/proxy/handler.rs`（日志注入点分析，三分支 protocol 路由）
- 现有代码：`src-tauri/src/proxy/state.rs`（ProxyState 扩展点）
- 现有代码：`src-tauri/src/proxy/mod.rs`（ProxyService 架构）
- 现有代码：`src/components/layout/AppShell.tsx`（AppView 类型扩展点）
- 项目上下文：`.planning/PROJECT.md`（v2.6 目标定义 + 约束条件）

---
*Feature research for: 本地 HTTP 代理流量监控（v2.6 milestone）*
*Researched: 2026-03-17*
