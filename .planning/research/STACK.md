# Stack Research

**Domain:** 流量监控（代理请求日志 + SQLite 持久化 + 实时推送 + 统计图表）
**Researched:** 2026-03-17
**Confidence:** HIGH

---

## 里程碑范围说明

本文档只覆盖 v2.6 流量监控所需的**增量**栈变化。以下技术已在 v2.0-v2.5 验证，**不重复研究**：

- Tauri 2.10, React 19, Vite 7, shadcn/ui, Tailwind CSS v4, i18next, axum 0.8
- serde, serde_json (preserve_order), toml_edit, notify (FSEvents), uuid, chrono
- reqwest 0.12 (+stream), axum 0.8, tower-http 0.6, tokio (net,sync,time)
- bytes, futures, async-stream, tauri-plugin-updater, tauri-plugin-process

---

## 核心发现摘要

v2.6 需要引入 **3 个新 Rust crate + 1 个新前端库**，其余能力由现有栈覆盖：

| 新增 | 用途 | 理由 |
|------|------|------|
| `rusqlite` 0.39 (bundled) | SQLite 持久化存储 | 无外部依赖，bundled 特性编译内嵌 SQLite 3.51.3 |
| `rusqlite_migration` 2.4 | Schema 版本管理 | 轻量级，用 SQLite 自带 user_version 追踪，专为 rusqlite 设计 |
| `tokio::sync::broadcast` | 日志实时推送到前端 | **tokio 已有，无需新增**，broadcast channel 是多消费者日志扇出的标准模式 |
| `recharts` 3.8 (npm) | 统计趋势图表 | React 生态标准图表库，React 19 + Tailwind v4 已验证兼容，cc-switch 同款 |

**前端实时更新**：`app.emit()` (Tauri 事件系统) 足以推送日志条目；日志频率通常 < 10 req/s，无需 Channel API（Channel 是高频流式场景使用的低延迟替代方案）。

**统计聚合**：直接在 SQLite 用 GROUP BY 查询完成，不需要前端聚合库或额外后端服务。

---

## Recommended Stack

### Core Technologies（新增）

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `rusqlite` | `0.39` | SQLite 读写：日志明细表 + 统计快照表 | bundled feature 内嵌 SQLite 3.51.3，无系统依赖；cc-switch 同款验证（用 0.31 bundled）；Rust 生态最成熟的 SQLite 绑定（4000万+下载）；Mutex<Connection> 与现有 tokio 架构完全匹配 |
| `rusqlite_migration` | `2.4` | Schema 版本控制与增量迁移 | 专为 rusqlite 设计，用 SQLite user_version pragma 追踪版本（零额外表）；API 极简（`Migrations::from_slice(&[M::up("SQL")])`）；支持同步和 async 两种模式；v2.6 只需 1 次 initial migration |
| `recharts` | `^3.8` | 实时折线图 / 面积图（token 趋势、请求量趋势） | React 19 已验证兼容（v3.x 修复了 React 19 渲染问题）；Tailwind v4 颜色变量格式已适配（去除 hsl() 包裹直接用 var(--chart-1)）；cc-switch UsageTrendChart.tsx 完整验证了 AreaChart + 双 Y 轴方案；1480万周下载量，维护活跃 |

### Supporting Libraries（现有栈已覆盖，无需新增）

| 能力需求 | 现有满足方案 | 说明 |
|----------|------------|------|
| 日志实时推送前端 | `tauri::AppHandle::emit()` | Tauri 事件系统；日志场景数据量小（一个 JSON 对象/请求），`emit()` 足够；高频流媒体才需要 Channel API |
| 多路消费日志流 | `tokio::sync::broadcast` | tokio 已有；handler.rs 写入 SQLite 后同时广播 LogEntry 给 Tauri 事件发射器 |
| 统计 GROUP BY 聚合 | SQLite SQL | 用 `GROUP BY provider_id, DATE(created_at, 'unixepoch')` 直接在 DB 层聚合，前端无需额外处理 |
| 日志表格虚拟化 | shadcn/ui Table | v2.6 明细最多保留 24 小时（几百条），无需 TanStack Virtual；若数据量超 500 行再引入 |
| 时间格式化 | `chrono` (已有) | Rust 侧时间戳格式化；前端用 JS 原生 Date |
| UUID 请求 ID | `uuid` (已有) | 每个代理请求生成唯一 ID |

---

## Cargo.toml 变更（最小化）

```toml
# src-tauri/Cargo.toml —— 在现有 [dependencies] 中追加两行
rusqlite = { version = "0.39", features = ["bundled"] }
rusqlite_migration = "2.4"
```

---

## package.json 变更

```bash
# 前端
pnpm add recharts
```

版本说明：`recharts@^3.8`（当前最新 3.8.0，2025-03-06 发布）

---

## 架构集成点

### Rust 侧：日志写入流

```
proxy/handler.rs（现有）
  └─► 请求完成后
        └─► traffic/logger.rs（新增）
              ├─► rusqlite INSERT → proxy_request_logs 表
              └─► AppHandle::emit("traffic-log-entry", LogEntry)
                    └─► 前端 TrafficPage listen() 更新表格
```

**关键设计决策**：
- `AppHandle` 注入 `ProxyState`（通过 axum State 扩展，或 `tokio::sync::broadcast` 中转）
- 日志写入在请求处理完成后异步执行（`tokio::spawn`），不阻塞代理响应链路
- SQLite 连接用 `Arc<Mutex<Connection>>`，与现有 `ProxyState` 的 `Arc<RwLock<_>>` 模式一致

### 前端侧：实时更新流

```
Tauri listen("traffic-log-entry")
  └─► React state update（新条目追加到列表头部，最多保留 200 条内存记录）
        └─► 页面首次挂载：invoke("traffic_get_recent_logs") 加载历史记录
              └─► Provider 筛选：前端过滤 OR invoke("traffic_get_logs_filtered")
```

### 统计数据流

```
前端定时 invoke("traffic_get_stats", {period: "7d"})
  └─► Rust: SQLite GROUP BY 聚合查询
        └─► 返回 DailyStats[] / ProviderStats[]
              └─► recharts AreaChart 渲染趋势图
```

---

## Schema 设计（v2.6 初始版本）

```sql
-- 24 小时滚动明细日志
CREATE TABLE IF NOT EXISTS proxy_request_logs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id   TEXT NOT NULL,
    provider_id  TEXT NOT NULL,
    provider_name TEXT NOT NULL,
    cli_type     TEXT NOT NULL,          -- "claude" | "codex"
    model        TEXT NOT NULL,          -- 原始请求模型名
    upstream_model TEXT,                 -- 实际转发模型名（映射后）
    is_streaming INTEGER NOT NULL DEFAULT 0,
    status_code  INTEGER NOT NULL,
    error_message TEXT,
    input_tokens  INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    latency_ms   INTEGER NOT NULL,
    created_at   INTEGER NOT NULL        -- Unix timestamp (seconds)
);

-- 7 天统计快照（每日 rollup，避免明细过期后失去统计）
CREATE TABLE IF NOT EXISTS traffic_daily_stats (
    date         TEXT NOT NULL,          -- "YYYY-MM-DD"
    provider_id  TEXT NOT NULL,
    cli_type     TEXT NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    input_tokens  INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    total_latency_ms INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (date, provider_id, cli_type)
);
```

**数据保留策略**：
- `proxy_request_logs`：`DELETE WHERE created_at < unixepoch() - 86400`（24 小时滚动，每次写入后触发清理或定时任务）
- `traffic_daily_stats`：`DELETE WHERE date < date('now', '-7 days')`（7 天保留）

---

## 不使用

| 避免引入 | 原因 | 替代方案 |
|----------|------|---------|
| `sqlx` | async-first 设计，需要额外 runtime 配置；rusqlite + `tokio::task::spawn_blocking` 在 Tauri 中更简单直接 | rusqlite 0.39 |
| `r2d2` / `deadpool` 连接池 | v2.6 写入频率低（< 10 req/s），单连接 `Mutex<Connection>` 足够；cc-switch 也用单连接 + Mutex 模式 | `Arc<Mutex<rusqlite::Connection>>` |
| `tauri-plugin-sql` | Tauri 官方 SQLite 插件，但抽象层薄且版本历史有 breaking changes；不如直接用 rusqlite 控制 schema | rusqlite 直接操作 |
| `@tanstack/react-virtual` | v2.6 日志最多 24 小时 / 几百条，普通 shadcn Table 足够；超过 500 行时再引入 | shadcn/ui Table |
| `@tanstack/react-query` | 已有 `tauri invoke` + `useEffect` 模式，再引入 react-query 增加复杂度；v2.6 数据流简单 | 直接 invoke |
| `rust_decimal` | cc-switch 用于费用计算；v2.6 不做费用估算，token 数用 INTEGER 即可 | SQLite INTEGER 类型 |
| Chart.js / victory / visx | recharts 已被 cc-switch 验证，与 shadcn/ui 风格统一，不值得引入生态差异 | recharts 3.8 |

---

## 版本兼容性

| Package | 版本 | 兼容性说明 |
|---------|------|-----------|
| `rusqlite 0.39` | 2026-03-15 发布 | 内嵌 SQLite 3.51.3（libsqlite3-sys 0.37.0）；Rust edition 2021；与 tokio 1 / Tauri 2 完全兼容 |
| `rusqlite_migration 2.4` | 当前稳定版 | 依赖 rusqlite 0.31+，与 rusqlite 0.39 兼容 |
| `recharts ^3.8` | 3.8.0 (2025-03-06) | React 19 兼容（v3.x 修复）；Tailwind v4 颜色变量适配（直接用 `var(--chart-1)` 而非 `hsl(var(--chart-1))`） |

---

## cc-switch 参考实现验证

| 文件 | 行数 | 实现内容 | v2.6 参考价值 |
|------|------|---------|--------------|
| `src-tauri/src/proxy/usage/logger.rs` | 423 | `UsageLogger`：INSERT INTO proxy_request_logs；`log_request()`、`log_error()`、`log_error_with_context()` | Schema 字段选择、错误记录模式（HIGH） |
| `src-tauri/src/services/usage_stats.rs` | ~300 | `DailyStats`、`ProviderStats`、`UsageSummary`、`LogFilters`、`PaginatedLogs` 数据结构 | 统计数据结构设计参考（HIGH） |
| `src-tauri/src/database/schema.rs` | ~300 | `proxy_request_logs` 表 DDL、所有字段定义 | Schema 字段参考（HIGH，但 v2.6 去掉费用相关字段） |
| `src-tauri/src/database/dao/proxy.rs` | ~300 | `get_global_proxy_config`、WAL pragma 配置 | SQLite 配置模式参考（HIGH） |
| `src/components/usage/UsageTrendChart.tsx` | 233 | recharts AreaChart + 双 Y 轴 + 自定义 Tooltip | 图表实现参考（MEDIUM，v2.6 只显示 token 无费用） |

**cc-switch 超出 v2.6 范围（不参考）**：
- `rust_decimal`、`CostCalculator`、`ModelPricing`——费用计算是 v2.7+ 方向
- `tauri-plugin-store` 存储代理配置——v2.6 用 iCloud/local 现有存储层
- `rusqlite hooks`——v2.6 不需要 DB change notifications

---

## Alternatives Considered

| 推荐 | 替代 | 不选替代的原因 |
|------|------|---------------|
| rusqlite 0.39 bundled | sqlx | sqlx async-first，需要 migrate! 宏和独立 .sql 文件，配置比 rusqlite 复杂；v2.6 写入频率低，无需 async DB |
| rusqlite_migration 2.4 | 手写 PRAGMA user_version | 手写方案代码更少但更脆弱；rusqlite_migration 提供事务安全的顺序迁移，未来加字段更安全 |
| recharts 3.8 | shadcn/ui Charts (也基于 recharts) | shadcn/ui Charts 是 recharts 的包装，直接用 recharts 更灵活；双 Y 轴（token + 时间）是 recharts 原生特性 |
| Tauri emit() | Tauri Channel API | Channel 用于高频大量有序流（如下载进度）；日志推送数量级差异大（<10 req/s vs 60fps），emit() 足够且实现更简单 |

---

## Sources

- [docs.rs/crate/rusqlite/latest](https://docs.rs/crate/rusqlite/latest) — 确认当前版本 0.39.0，内嵌 SQLite 3.51.3（HIGH）
- [docs.rs/rusqlite_migration/latest](https://docs.rs/rusqlite_migration/latest/rusqlite_migration/) — 确认当前版本 2.4.1，API 设计（HIGH）
- [github.com/recharts/recharts/releases](https://github.com/recharts/recharts/releases) — 确认 3.8.0（2025-03-06），React 19 兼容（HIGH）
- [v2.tauri.app/develop/calling-frontend/](https://v2.tauri.app/develop/calling-frontend/) — Events vs Channels 官方说明；emit() 适用小数据/低频；Channel 适用流式高频（HIGH）
- cc-switch `src-tauri/src/proxy/usage/logger.rs` — rusqlite INSERT 模式 + RequestLog 字段结构（HIGH）
- cc-switch `src-tauri/src/database/schema.rs` — proxy_request_logs DDL，rusqlite 0.31 bundled 验证（HIGH）
- cc-switch `src/components/usage/UsageTrendChart.tsx` — recharts AreaChart + React 18 验证（MEDIUM，已升级到 React 19）
- cc-switch `src-tauri/Cargo.toml` — `rusqlite = { version = "0.31", features = ["bundled", "backup", "hooks"] }` 选型交叉验证（HIGH）

---

*Stack research for: v2.6 流量监控（SQLite 持久化 + 实时日志推送 + 统计图表）*
*Researched: 2026-03-17*
