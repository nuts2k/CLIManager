# Architecture Research

**Domain:** 流量监控集成到现有 Tauri 2 代理应用
**Researched:** 2026-03-17
**Confidence:** HIGH

## System Overview

### v2.6 新增组件

```
┌──────────────────────────────────────────────────────────┐
│                     Frontend (React 19)                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Provider │  │ Traffic  │  │ Settings │  │ Updater  │  │
│  │ Page     │  │ Page NEW │  │ Page     │  │ Dialog   │  │
│  │          │  │ ├Log Tab │  │          │  │          │  │
│  │          │  │ └Stat Tab│  │          │  │          │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │
├──────────────────────────────────────────────────────────┤
│          Tauri Commands / Events                          │
│   get_request_logs  │  get_traffic_stats                  │
│   emit("traffic-log") NEW                                │
├──────────────────────────────────────────────────────────┤
│                     Rust Backend                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │  traffic/ NEW                                     │    │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  │    │
│  │  │ db.rs  │  │logger  │  │ token  │  │ clean  │  │    │
│  │  │SQLite  │  │.rs     │  │.rs     │  │.rs     │  │    │
│  │  │schema  │  │write   │  │extract │  │rollup  │  │    │
│  │  │query   │  │+emit   │  │parse   │  │prune   │  │    │
│  │  └────────┘  └────────┘  └────────┘  └────────┘  │    │
│  └──────────────────────────────────────────────────┘    │
│                                                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │  proxy/handler.rs (MODIFIED)                      │    │
│  │  请求完成后 -> log_tx.send(LogEntry)               │    │
│  └──────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | New/Modified |
|-----------|----------------|--------------|
| `traffic/db.rs` | SQLite 连接管理、schema 迁移、读写查询 | **新增** |
| `traffic/logger.rs` | 从 channel 接收 LogEntry，写 SQLite + emit 到前端 | **新增** |
| `traffic/token.rs` | 从 API 响应中提取 token 用量（三种协议格式） | **新增** |
| `traffic/clean.rs` | 滚动保留清理（24h 明细 + 7d 统计聚合） | **新增** |
| `commands/traffic.rs` | Tauri 命令：get_request_logs, get_traffic_stats | **新增** |
| `proxy/handler.rs` | 请求完成后发送 LogEntry 到 log channel | **修改** |
| `proxy/state.rs` | ProxyState 新增 log_tx 字段 | **修改** |
| `TrafficPage` | 流量监控独立页面（日志 + 统计两个 Tab） | **新增** |
| `AppShell` | 导航新增流量监控入口 | **修改** |

## Recommended Project Structure

### Rust 后端新增

```
src-tauri/src/
├── traffic/               # NEW：流量监控模块
│   ├── mod.rs             # 模块导出 + LogEntry 定义
│   ├── db.rs              # SQLite 连接池、schema 迁移、CRUD 查询
│   ├── logger.rs          # 后台写入 task（channel -> SQLite + emit）
│   ├── token.rs           # token 用量提取（三种协议）
│   └── clean.rs           # 滚动保留 + 统计聚合
├── commands/
│   └── traffic.rs         # NEW：Tauri 命令
├── proxy/
│   ├── handler.rs         # MODIFIED：请求完成后 log_tx.send()
│   └── state.rs           # MODIFIED：ProxyState 新增 log_tx
└── lib.rs                 # MODIFIED：注册 traffic 模块 + Tauri 命令
```

### React 前端新增

```
src/
├── components/
│   ├── traffic/           # NEW：流量监控组件
│   │   ├── TrafficPage.tsx       # 顶级页面容器（Tab 切换）
│   │   ├── LogTable.tsx          # 实时日志表格
│   │   ├── LogFilter.tsx         # Provider 筛选
│   │   ├── StatsView.tsx         # 统计数据展示
│   │   └── StatsChart.tsx        # 图表组件（recharts）
│   └── layout/
│       └── AppShell.tsx   # MODIFIED：新增导航入口
└── types/
    └── traffic.ts         # NEW：类型定义
```

## Architectural Patterns

### Pattern 1: 非阻塞日志管道 (Channel -> Background Task)

**What:** handler.rs 请求完成后通过 `tokio::sync::mpsc::UnboundedSender` 非阻塞发送 LogEntry，独立后台 task 消费并写入 SQLite + emit 到前端。
**When to use:** 日志写入不应影响代理请求延迟。
**Trade-offs:** + 零延迟影响 + 批量写入可优化 | - 异常时日志可能丢失（可接受）

```rust
// handler.rs 末尾（请求完成后）
if let Some(log_tx) = &state.log_tx {
    let _ = log_tx.send(LogEntry { ... }); // 非阻塞，失败忽略
}

// logger.rs 后台 task
async fn log_writer_task(mut rx: UnboundedReceiver<LogEntry>, db: TrafficDb, app: AppHandle) {
    while let Some(entry) = rx.recv().await {
        db.insert_log(&entry);                     // 同步 SQLite 写入
        let _ = app.emit("traffic-log", &entry);   // 推送到前端
    }
}
```

### Pattern 2: 双轨数据加载 (Command 初始 + Event 增量)

**What:** 前端页面挂载时通过 Tauri command 拉取历史日志，之后通过 event listener 接收实时增量。
**When to use:** 避免 Tauri 事件在 webview 未就绪时丢失。
**Trade-offs:** + 不丢数据 + 首次加载完整 | - 需要去重逻辑（用 ID 判断）

```typescript
// TrafficPage.tsx
useEffect(() => {
  // 1. 初始加载
  invoke('get_request_logs', { limit: 200 }).then(setLogs);
  // 2. 增量监听
  const unlisten = listen('traffic-log', (event) => {
    setLogs(prev => [event.payload, ...prev].slice(0, MAX_DISPLAY));
  });
  return () => { unlisten.then(fn => fn()); };
}, []);
```

### Pattern 3: Token 提取策略模式

**What:** 根据 ResponseTranslationMode（已有枚举）选择不同的 token 提取逻辑。
**When to use:** 三种协议的 token 字段位置不同。

```
Anthropic 透传:     response.usage.input_tokens / output_tokens
  流式:             message_start(input) + message_delta(output)
OpenAI ChatCompl:   response.usage.prompt_tokens / completion_tokens
  流式:             末尾 chunk.usage
OpenAI Responses:   response.usage.input_tokens / output_tokens
  流式:             response.completed 事件
```

## Data Flow

### 请求日志数据流

```
CLI 请求
    |
proxy/handler.rs (proxy_handler)
    | 请求转发 + 响应接收
    | 提取 metadata + token 用量
    |
log_tx.send(LogEntry)       <- 非阻塞，不影响响应返回
    |
traffic/logger.rs (后台 task)
    |-> db.insert_log()          -> SQLite (traffic.db)
    +-> app.emit("traffic-log")  -> Frontend
                                      |
                                TrafficPage
                                |- LogTable (实时更新)
                                +- StatsView (定时刷新)
```

### 统计查询数据流

```
用户切换到统计 Tab
    |
invoke('get_traffic_stats', { range: '24h', group_by: 'provider' })
    |
commands/traffic.rs
    |
traffic/db.rs -- SQL 聚合查询
    |
返回统计数据 -> StatsView 渲染表格 + StatsChart 渲染图表
```

### 滚动清理数据流

```
tokio::time::interval(1 hour)
    |
traffic/clean.rs
    |-> 聚合 >24h 明细 -> daily_rollups 表
    +-> 删除 >7d 的 rollup 记录
```

### LogEntry 结构

```rust
pub struct LogEntry {
    pub id: String,              // UUID
    pub timestamp: i64,          // Unix timestamp
    pub cli_id: String,          // "claude" | "codex"
    pub provider_id: String,     // Provider UUID
    pub provider_name: String,   // 显示名
    pub method: String,          // "POST"
    pub path: String,            // "/v1/messages"
    pub status_code: u16,        // 200, 502, etc.
    pub latency_ms: u64,         // 请求耗时
    pub is_streaming: bool,      // 是否流式
    pub request_model: String,   // 请求的模型名
    pub response_model: Option<String>,  // 响应中的模型名
    pub input_tokens: Option<i64>,       // 输入 token 数
    pub output_tokens: Option<i64>,      // 输出 token 数
    pub error_message: Option<String>,   // 错误信息（失败时）
    pub stop_reason: Option<String>,     // stop_reason / finish_reason
}
```

### SQLite Schema

```sql
-- 请求明细日志（滚动保留 24 小时）
CREATE TABLE request_logs (
    id TEXT PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    cli_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    provider_name TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    latency_ms INTEGER NOT NULL,
    is_streaming INTEGER NOT NULL DEFAULT 0,
    request_model TEXT NOT NULL DEFAULT '',
    response_model TEXT DEFAULT NULL,
    input_tokens INTEGER DEFAULT NULL,
    output_tokens INTEGER DEFAULT NULL,
    error_message TEXT DEFAULT NULL,
    stop_reason TEXT DEFAULT NULL
);

CREATE INDEX idx_request_logs_timestamp ON request_logs(timestamp);
CREATE INDEX idx_request_logs_provider ON request_logs(provider_id);

-- 每日聚合统计（滚动保留 7 天）
CREATE TABLE daily_rollups (
    date TEXT NOT NULL,           -- YYYY-MM-DD
    cli_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    provider_name TEXT NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    total_input_tokens INTEGER NOT NULL DEFAULT 0,
    total_output_tokens INTEGER NOT NULL DEFAULT 0,
    avg_latency_ms INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (date, cli_id, provider_id)
);
```

## Integration Points

### handler.rs 修改点

handler.rs `proxy_handler` 函数需要在以下位置注入日志：

1. **请求开始**：记录 `start_time = Instant::now()`
2. **请求结束**（所有分支汇合处）：构造 LogEntry 并 `log_tx.send()`
3. **非流式请求**：从响应 body 直接提取 token（已有 body 解析）
4. **流式请求**：需要在 stream 消费完毕后提取（需要 wrapper stream）

### ProxyState 扩展

```rust
pub struct ProxyState {
    upstream: Arc<RwLock<Option<UpstreamTarget>>>,
    pub http_client: reqwest::Client,
    pub log_tx: Option<UnboundedSender<LogEntry>>,  // NEW
}
```

`log_tx` 为 `Option` 是因为：
- 代理模式关闭时不需要日志
- 测试中可以不注入 logger
- 与现有 ProxyServer::new() 签名兼容

### 前端 AppShell 导航扩展

当前 AppShell 用 opacity 双视图切换（Providers / Settings）。新增 Traffic 页面需要扩展为三视图，沿用现有 opacity 模式。

### 已有模式复用

| 已有模式 | 复用于 |
|----------|--------|
| `overlay_path_override` 注入模式 | SQLite db 路径测试注入 |
| `ProtocolType` 枚举 | token 提取路由 |
| `ResponseTranslationMode` 枚举 | 判断流式/非流式 + 协议类型 |
| `ProxyService` 生命周期管理 | logger task 的启停跟随代理 |

## Suggested Build Order

| Phase | Focus | Dependencies |
|-------|-------|-------------|
| 1 | SQLite 基础设施：db.rs + schema + 连接管理 + logger 后台 task | 无 |
| 2 | handler.rs 日志注入 + 非流式 token 提取 | Phase 1 |
| 3 | 流式 token 提取（三种 SSE 格式） | Phase 2 |
| 4 | Tauri 命令 + 前端 TrafficPage + 日志表格 | Phase 1-2 |
| 5 | 统计聚合 + 滚动清理 + 统计页面 + 图表 | Phase 1-4 |

## Anti-Patterns

### Anti-Pattern 1: 同步 SQLite 写入阻塞 handler

**What people do:** 在 proxy_handler 里直接调用 `db.insert_log()`
**Why it's wrong:** rusqlite 是同步阻塞 API，会阻塞 tokio runtime
**Do this instead:** 用 channel 异步发送，后台 task 消费写入

### Anti-Pattern 2: 流式请求中途提取 token

**What people do:** 在 SSE 流传输过程中解析 token
**Why it's wrong:** token 数分布在不同事件中，中途提取得到残缺数据
**Do this instead:** 等流完全结束后从最终事件中提取

### Anti-Pattern 3: 纯事件驱动前端（无初始加载）

**What people do:** 前端只监听 `traffic-log` 事件
**Why it's wrong:** webview 就绪前的事件全部丢失；页面切换回来也没有历史
**Do this instead:** 双轨模式 -- command 拉取初始数据 + event 接收增量

### Anti-Pattern 4: SQLite 放在 iCloud 目录

**What people do:** 数据库文件放在同步目录
**Why it's wrong:** SQLite WAL 模式 + iCloud 最终一致性 = 数据损坏
**Do this instead:** 放在 `app_data_dir()`（~/Library/Application Support/）

## Sources

- CLIManager 现有代码：proxy/handler.rs, proxy/state.rs, proxy/server.rs, proxy/mod.rs
- cc-switch 参考：proxy_request_logs schema, usage_rollup 逻辑
- Tauri 2 官方文档：events, commands
- STACK.md / FEATURES.md / PITFALLS.md 研究输出

---
*Architecture research for: v2.6 流量监控*
*Researched: 2026-03-17*
