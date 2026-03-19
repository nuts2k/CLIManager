# Phase 27: 日志写入管道 - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

每个代理请求完成后，非阻塞地将元数据（含非流式 token 用量、错误信息）写入 SQLite 并实时推送到前端。建立 mpsc channel 写入管道，handler 中采集元数据并通过 channel 发送，后台 task 写入 SQLite 并 emit Tauri 事件。Phase 28 将在此管道基础上补充流式 token 提取。

</domain>

<decisions>
## Implementation Decisions

### 流式请求日志策略
- Phase 27 为所有请求（含流式）写入基础元数据，流式请求的 token 字段（input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens）和耗时字段（duration_ms, ttfb_ms）留 null
- Phase 28 在 stream EOF 后 UPDATE 同一行，填充 token 和耗时字段
- 流式请求写入基础日志后立即 emit 到前端（token/duration 为 null），Phase 28 补充后再 emit 一次更新事件
- 统一使用 `traffic-log` 事件名，payload 含 `type` 字段区分 `"new"` 和 `"update"`，前端用同一个 listener 根据 type 决定 append 还是 update row

### 非流式 Token + 缓存 Token 提取
- 非流式响应在 handler 中直接解析 usage 字段，提取 input_tokens、output_tokens、cache_creation_tokens、cache_read_tokens（不通过 mpsc 传输 body）
- 三种协议的非流式 token 提取全部在 Phase 27 实现：
  - Anthropic: usage.input_tokens, usage.output_tokens, usage.cache_creation_input_tokens, usage.cache_read_input_tokens
  - OpenAI Chat Completions: usage.prompt_tokens, usage.completion_tokens, usage.prompt_tokens_details.cached_tokens
  - OpenAI Responses: usage.input_tokens, usage.output_tokens（缓存字段位置由研究阶段确认）
- stop_reason/finish_reason 存原始值，不做跨协议映射（Anthropic 存 end_turn/max_tokens 等，OpenAI 存 stop/length 等），保留协议差异便于调试

### 事件 Payload 设计
- traffic-log 事件 payload 携带 request_logs 表完整 19 列 + type 字段（"new"/"update"）
- 时间戳保持 epoch 毫秒（与 DB 一致），前端用 new Date(ts) 转换显示
- 前端初始加载通过 Tauri command 查询最近 100 条日志（双轨策略：command 初始拉取 + event 增量追加）
- Phase 27 提供基础查询 command（按 created_at 降序，LIMIT 参数）

### Claude's Discretion
- mpsc channel buffer 大小
- handler 中计时逻辑的具体实现方式（Instant::now 位置）
- TrafficDb/AppHandle/mpsc sender 在 ProxyState 中的注入方式
- 错误日志的 error_message 格式化方式
- 查询 command 的具体函数签名和过滤参数

</decisions>

<specifics>
## Specific Ideas

- Phase 26 STATE.md 中已确定双轨策略（command 初始拉取 + event 增量追加），事件不作 source of truth
- 流式请求的两次 emit（new + update）让前端能立即看到请求开始，然后看到 token 填充，类似"进行中"→"完成"的状态变化

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `traffic::TrafficDb` (traffic/mod.rs): 已通过 `app.manage()` 注入，持有 `Mutex<Connection>`
- `proxy::handler::proxy_handler`: 请求处理入口，非流式 body 已在 handler 中读取用于协议转换
- `proxy::handler::ResponseTranslationMode`: 携带 request_model 字段，可提供模型映射信息
- `proxy::state::ProxyState` + `UpstreamTarget`: 持有 api_key, base_url, protocol_type, upstream_model

### Established Patterns
- Tauri emit: watcher/mod.rs 中 `app_handle.emit("event-name", &payload)` 模式
- async_runtime::spawn: watcher 中的后台异步操作 spawn 模式
- 降级运行: traffic/mod.rs 中 `init_traffic_db() -> Option<TrafficDb>` + `if let Some` 模式
- Tauri manage 注入: 通过 `app.manage(state)` 注入，handler 中通过 State extractor 获取

### Integration Points
- handler.rs: 计时测量（start/end）、元数据采集、非流式 body 解析 token、通过 mpsc sender 发送日志
- lib.rs setup 闭包: 创建 mpsc channel、启动后台写入 task、将 sender 注入到 proxy 状态
- ProxyState: 需扩展以包含 mpsc sender 和/或 AppHandle（供 emit 使用）
- traffic 模块: 需新增 insert_request_log 方法和查询 command

</code_context>

<deferred>
## Deferred Ideas

- 流式 SSE 的 token + 缓存字段提取 -- Phase 28
- 流式请求的 duration_ms 和 ttfb_ms 计算 -- Phase 28
- 前端流量监控页面 -- Phase 29
- 统计聚合与数据保留 -- Phase 30
- 费用估算 (cost_usd) -- v2.7+ (ADV-01)

</deferred>

---

*Phase: 27-log-pipeline*
*Context gathered: 2026-03-18*
