# Phase 26: SQLite 基础设施 - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

初始化 traffic.db（WAL 模式，路径在 app_local_data_dir，非 iCloud），建立 request_logs 和 daily_rollups 两张表 schema，提供连接管理和 schema 迁移机制。后续所有流量数据读写操作的基础设施。

</domain>

<decisions>
## Implementation Decisions

### 摘要字段内容
- 不记录请求/响应内容（零隐私风险，零存储开销）
- 保留 stop_reason/finish_reason 作为元数据（COLLECT-04 要求）
- 额外增加 upstream_model 列（模型映射后的实际上游模型名，区别于 request_model）
- 额外增加 protocol_type 列（Anthropic/OpenAiChatCompletions/OpenAiResponses，便于按协议统计）

### Provider 引用方式
- request_logs 中用 provider_name TEXT 存储名称快照（记录时刻的名称，不随 Provider 改名/删除而变化）
- 无需 UUID 引用，查询无需 join，前端直接显示
- CLI 字段按端口号推断（15800=claude-code, 15801=codex），存为 TEXT

### 耗时指标定义
- TTFB (ttfb_ms): 代理向上游发出 reqwest 请求开始 → 收到上游响应第一个字节。反映 Provider 响应速度，不含客户端→代理网络延迟
- Duration (duration_ms): handler 全生命周期（从收到客户端请求到响应完全发送完毕，含流式 stream 全部传输）
- 精度：毫秒 (INTEGER/i64)
- token/sec: 前端实时计算 output_tokens / (duration_ms - ttfb_ms) * 1000，不存 DB

### 缓存指标字段
- request_logs 预留 cache_creation_tokens (INTEGER) 和 cache_read_tokens (INTEGER) 两列
- Anthropic: 直接填充 cache_creation_input_tokens 和 cache_read_input_tokens
- OpenAI: cached_tokens 填入 cache_read_tokens（creation=0）
- 缓存触发判定：cache_creation_tokens > 0 OR cache_read_tokens > 0
- 缓存命中判定：cache_read_tokens > 0
- 各协议具体字段位置由研究/规划阶段确认

### 时间戳存储格式
- request_logs.created_at: Unix epoch 毫秒 (INTEGER/i64)，无时区歧义，范围查询高效
- daily_rollups.rollup_date: TEXT YYYY-MM-DD，人类可读，UNIQUE(provider_name, rollup_date) 约束简洁

### rollup 聚合设计
- 粒度：按 Provider+天 聚合，每个 Provider 每天一行
- 聚合字段（10 列）：
  - request_count (INTEGER)
  - success_count (INTEGER) — 用于成功率计算
  - total_input_tokens (INTEGER)
  - total_output_tokens (INTEGER)
  - total_cache_creation_tokens (INTEGER)
  - total_cache_read_tokens (INTEGER)
  - cache_triggered_count (INTEGER) — cache 字段非零的请求数
  - cache_hit_count (INTEGER) — cache_read > 0 的请求数
  - sum_ttfb_ms (INTEGER) — 用于加权平均 TTFB
  - sum_duration_ms (INTEGER) — 用于加权平均 TPS
- 所有排行榜指标可从 rollup 组合计算（已验证可组合性）
- 平均 TPS 用加权平均：SUM(output_tokens) / (SUM(duration_ms) - SUM(ttfb_ms)) * 1000

### DB 损坏恢复策略
- 启动时发现 traffic.db 损坏或 schema 不兼容：静默删除并重建空表
- DB 初始化失败（磁盘写保护、权限不足等）：降级运行，代理正常工作但不记录流量，记录日志警告

### Claude's Discretion
- rusqlite_migration 具体版本和用法
- 索引设计（哪些列需要索引）
- Arc<std::sync::Mutex<Connection>> 的具体封装方式
- DB 文件命名（traffic.db 或其他）
- 各协议缓存字段的具体提取位置（研究阶段确认）

</decisions>

<specifics>
## Specific Ideas

- 需要支持两个排行榜视图：
  1. 供应商排行榜（请求数、Token 数、成功率、平均 TTFB、平均 TPS，任意字段升降序排序）
  2. 供应商缓存命中率排行榜（缓存触发请求数、缓存命中率、缓存读取 token 数、总 token 数，命中率降序）
  3. 两个排行榜均支持滚动 24 小时和 7 天两个时间维度
- 24 小时维度直接从 request_logs 查询，7 天维度从 daily_rollups 聚合查询

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `proxy::state::ProxyState` — 持有 upstream target 信息，可从中获取 provider_name/protocol_type/upstream_model
- `proxy::handler::proxy_handler` — 请求处理入口，日志采集点在此函数内（Phase 27）
- `proxy::handler::ResponseTranslationMode` — 携带 request_model 字段，可提供模型映射信息
- `storage::local` — 本地存储路径管理，traffic.db 路径可参考此模块的路径获取方式

### Established Patterns
- Tauri `.manage()` 注入共享状态 — DB 连接可同样通过 manage 注入为全局状态
- `tokio::sync::RwLock` 用于代理状态共享 — DB 用 `std::sync::Mutex` 因为 rusqlite 是同步 API
- 模块组织：`proxy/` 子目录按功能拆分（state, server, handler, translate/）— traffic 模块可类似组织

### Integration Points
- `lib.rs` setup 闭包：DB 初始化应在此处，在 watcher 和 proxy 恢复之间
- `Cargo.toml`：需添加 rusqlite（bundled feature）和 rusqlite_migration 依赖
- `proxy::handler`：Phase 27 将在此采集数据，Phase 26 只需确保 DB 连接可从 handler 访问

</code_context>

<deferred>
## Deferred Ideas

- 供应商排行榜和缓存命中率排行榜的 UI 设计 — Phase 29/30 前端页面
- 流式 SSE 的缓存字段提取 — Phase 28
- 费用估算（cost_usd）— v2.7+ (ADV-01)
- first_token_ms 精确指标（区别于 TTFB） — v2.7+ (ADV-04)

</deferred>

---

*Phase: 26-sqlite*
*Context gathered: 2026-03-18*
