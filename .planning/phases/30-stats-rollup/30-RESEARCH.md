# Phase 30: 统计聚合与数据保留 - Research

**Researched:** 2026-03-18
**Domain:** SQLite 聚合查询、tokio 定时任务、recharts 图表库、React Tab 布局
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**统计视图布局**
- TrafficPage 内部新增 Tab 切换：「实时日志」和「统计分析」两个 Tab
- Tab 样式使用 line 下划线风格（与 Settings 页 Tab 视觉统一）
- 默认进入「实时日志」Tab（现有功能），用户可切换到「统计分析」Tab
- 现有 5 张统计摘要卡片只在「实时日志」Tab 显示，「统计分析」Tab 不显示卡片

**统计分析 Tab 内部布局**
- 顶部：24h / 7d Segment 按钮组（控制所有统计数据的时间范围）
- 上半区：左右并排两个排行榜表格（供应商排行榜 + 缓存命中率排行榜）
- 下半区：全宽趋势图表

**趋势图表**
- 使用 recharts 库实现（React 生态成熟方案，REQUIREMENTS 已提及）
- 双轴图表：左 Y 轴请求数（柱状图），右 Y 轴 Token 总量（折线图）
- 时间粒度跟随 24h/7d 切换：24h 模式显示 24 个小时点，7d 模式显示 7 个天点
- 24h 数据从 request_logs 按小时聚合查询，7d 数据从 daily_rollups 按天查询

**供应商排行榜**
- 列：Provider、请求数、Token（in/out 合并一列）、成功率、平均 TTFB、平均 TPS
- 点击表头列名切换升/降序排序，默认按请求数降序
- Token 列格式：in: 1.2k / out: 3.4k（与实时日志表格 Token 列风格一致）
- 支持 24h 和 7d 两个时间维度（跟随全局 Segment 切换）

**缓存命中率排行榜**
- 列（Phase 26 定义）：缓存触发请求数、缓存命中率、缓存读取 token 数、总 token 数
- 点击表头排序，默认按命中率降序
- 支持 24h 和 7d 两个时间维度

**24h/7d 时间维度切换**
- Segment 按钮组，位于统计分析 Tab 标题旁
- 切换后联动更新：两个排行榜表格 + 趋势图表
- 24h 数据源：request_logs 实时查询
- 7d 数据源：daily_rollups 聚合查询

**rollup_and_prune 定时任务**
- 触发时机：应用启动时立即执行一次 + tokio::interval(1h) 定时重复
- 完全静默执行，用户无感知，成功/失败只记 log::info/warn
- 单次事务内原子操作：INSERT/UPDATE daily_rollups → DELETE 超 24h 明细
- 同时删除超 7d 的 daily_rollups 记录
- 失败不重试，等下一轮自动触发

### Claude's Discretion
- recharts 具体版本和图表样式细节（颜色、tooltip、动画等）
- Segment 按钮组的具体样式实现（shadcn Tabs 或自定义）
- 排行榜表格为空时的空状态设计
- 后端聚合查询的 SQL 优化和接口设计
- rollup_and_prune 的 SQL 事务实现细节
- 趋势图表的暗色主题适配细节

### Deferred Ideas (OUT OF SCOPE)
- 费用估算 (cost_usd) -- v2.7+ (ADV-01)
- 实时告警与阈值配置 -- v2.7+ (ADV-02)
- 导出报表 (JSON/CSV) -- v2.7+ (ADV-03)
- 保留时长用户可配置（当前硬编码 24h/7d）-- v2.7+
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| STORE-04 | 定时清理任务聚合超过 24h 的明细为每日统计，删除超过 7d 的统计数据 | rollup_and_prune SQL 事务模式、tokio::interval 定时触发模式 |
| STAT-02 | 按 Provider 聚合表格展示各 Provider 的请求数、token 用量、平均耗时 | 后端 GROUP BY provider_name 聚合 SQL、前端排行榜表格排序模式 |
| STAT-03 | 按时间聚合表格展示每小时/每天的请求数、token 量等 | 24h 按小时聚合（strftime）、7d 从 daily_rollups 查询 |
| STAT-04 | 趋势图表（recharts）可视化时间维度的流量变化 | recharts 3.x ComposedChart 双轴图、ResponsiveContainer、暗色主题 CSS 变量适配 |
</phase_requirements>

---

## Summary

Phase 30 在已完成的 Phase 26-29 基础上新增三个能力层：后端定时聚合与清理（rollup_and_prune）、后端聚合查询接口（按 Provider + 按时间维度）、前端统计分析 UI（Tab + 排行榜 + recharts 趋势图）。

后端侧完全使用已有依赖（rusqlite + tokio），无需新增 Cargo 依赖。`rollup_and_prune` 以单次 SQLite 事务原子完成 INSERT/UPDATE daily_rollups + DELETE 超期明细，通过 `tauri::async_runtime::spawn` + `tokio::time::interval` 在 `lib.rs` setup 闭包中启动后台循环任务。聚合查询方法同样新增到 `TrafficDb` 的 `impl` 块中，通过新 Tauri commands 暴露给前端。

前端侧需新增 `recharts` 依赖（当前 package.json 不含该库）。项目已有 Tabs UI 组件（line 变体）和 CSS 变量暗色主题体系，recharts 颜色可直接引用 `--color-chart-1` 等 CSS 变量实现暗色适配。TrafficPage 重构为 Tab 容器，实时日志 Tab 保持现有功能，统计分析 Tab 是新增面板。

**Primary recommendation:** 使用 recharts ^2.15 而非 3.x（shadcn/ui 与 recharts 3.x 尚未正式兼容），后端 rollup SQL 使用 `INSERT OR REPLACE` + `SELECT ... GROUP BY` 单条语句完成每天每 Provider 的 upsert，`rollup_and_prune` 在 `tauri::async_runtime::spawn` 内使用 `tokio::time::interval(Duration::from_secs(3600))` 触发。

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| recharts | ^2.15.x | 趋势图表（ComposedChart 双轴） | REQUIREMENTS 明确指定，React 生态主流图表库，React 19 兼容 |
| rusqlite | 0.38（已有） | rollup SQL 事务、聚合查询 | 已有依赖，SQLite 的 Rust 绑定 |
| tokio::time | 1.x（已有） | tokio::interval 定时触发 | 已有依赖，Cargo.toml 含 "time" feature |
| Tabs（shadcn/radix-ui）| 已有 | TrafficPage Tab 切换 | 项目已有，line 变体已验证在 SettingsPage |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| @types/recharts | 不需要 | recharts 自带 TS 类型定义 | 安装 recharts 即包含 |
| useTranslation（react-i18next）| 已有 | 统计分析 Tab 的 UI 文字国际化 | 所有新增 UI 文字均需 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| recharts ^2.15 | recharts ^3.x | 3.x 是重写版本，shadcn/ui 尚未正式支持（已知 breaking），2.x API 更稳定 |
| recharts | victory / nivo | recharts 是项目明确决策，不考虑替换 |
| tokio::interval 无限循环 | tauri 定时器插件 | 无需额外插件，tokio 已有 time feature |

**Installation:**
```bash
npm install recharts@^2.15
```

---

## Architecture Patterns

### Recommended Project Structure

新增文件概览：

```
src-tauri/src/traffic/
├── mod.rs          # 不变（init_traffic_db）
├── schema.rs       # 不变（表结构已有）
├── log.rs          # 不变（日志写入/查询）
├── db.rs           # 不变（连接管理）
└── rollup.rs       # 新增：rollup_and_prune() + 聚合查询方法

src-tauri/src/commands/
└── traffic.rs      # 扩展：新增 3 个 Tauri command

src-tauri/src/lib.rs  # 扩展：setup 闭包中启动 rollup 定时任务

src/components/traffic/
├── TrafficPage.tsx           # 重构为 Tab 容器
├── StatsAnalysisTab.tsx      # 新增：统计分析 Tab 主面板
├── ProviderLeaderboard.tsx   # 新增：供应商排行榜表格
├── CacheLeaderboard.tsx      # 新增：缓存命中率排行榜表格
└── TrafficTrendChart.tsx     # 新增：recharts 双轴趋势图

src/hooks/
└── useTrafficStats.ts        # 新增：聚合数据拉取 hook

src/lib/tauri.ts              # 扩展：新增 3 个 invoke 封装

src/i18n/locales/
├── zh.json                   # 新增 traffic.stats.* 键
└── en.json                   # 新增 traffic.stats.* 键
```

---

### Pattern 1: rollup_and_prune SQL 事务（原子 upsert + delete）

**What:** 单次数据库事务内完成三步：聚合 request_logs 写入/更新 daily_rollups → 删除超 24h 明细 → 删除超 7d 的 daily_rollups 记录。

**When to use:** `rollup_and_prune()` 方法内，每次调用均保证原子性。

**Example:**
```rust
// src-tauri/src/traffic/rollup.rs
impl super::TrafficDb {
    /// 聚合超 24h 明细到 daily_rollups，删除已聚合明细，删除超 7d 统计
    pub fn rollup_and_prune(&self) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("
            BEGIN;

            -- 步骤 1：将超过 24h 的明细按 (provider_name, rollup_date) 聚合写入/更新 daily_rollups
            -- rollup_date 使用 UTC 日期字符串 'YYYY-MM-DD'
            -- 超 24h 阈值：created_at < (strftime('%s','now') - 86400) * 1000  (epoch ms)
            INSERT OR REPLACE INTO daily_rollups (
                provider_name, rollup_date,
                request_count, success_count,
                total_input_tokens, total_output_tokens,
                total_cache_creation_tokens, total_cache_read_tokens,
                cache_triggered_count, cache_hit_count,
                sum_ttfb_ms, sum_duration_ms
            )
            SELECT
                provider_name,
                strftime('%Y-%m-%d', created_at / 1000, 'unixepoch') AS rollup_date,
                COUNT(*)                                               AS request_count,
                SUM(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END) AS success_count,
                COALESCE(SUM(input_tokens), 0)                        AS total_input_tokens,
                COALESCE(SUM(output_tokens), 0)                       AS total_output_tokens,
                COALESCE(SUM(cache_creation_tokens), 0)               AS total_cache_creation_tokens,
                COALESCE(SUM(cache_read_tokens), 0)                   AS total_cache_read_tokens,
                SUM(CASE WHEN cache_creation_tokens > 0 OR cache_read_tokens > 0 THEN 1 ELSE 0 END) AS cache_triggered_count,
                SUM(CASE WHEN cache_read_tokens > 0 THEN 1 ELSE 0 END) AS cache_hit_count,
                COALESCE(SUM(ttfb_ms), 0)                             AS sum_ttfb_ms,
                COALESCE(SUM(duration_ms), 0)                         AS sum_duration_ms
            FROM request_logs
            WHERE created_at < (strftime('%s', 'now') - 86400) * 1000
            GROUP BY provider_name, strftime('%Y-%m-%d', created_at / 1000, 'unixepoch');

            -- 步骤 2：删除已聚合的超 24h 明细
            DELETE FROM request_logs
            WHERE created_at < (strftime('%s', 'now') - 86400) * 1000;

            -- 步骤 3：删除超过 7d 的 daily_rollups 记录（7d = 7 * 86400 秒前的日期）
            DELETE FROM daily_rollups
            WHERE rollup_date < strftime('%Y-%m-%d', 'now', '-7 days');

            COMMIT;
        ")?;
        Ok(())
    }
}
```

> **INSERT OR REPLACE 与 UNIQUE 约束**：`daily_rollups` 表有 `UNIQUE(provider_name, rollup_date)` 约束，`INSERT OR REPLACE` 在冲突时会先删除旧行再插入新行，导致 id 变化。由于每次 rollup 是全量重聚合该 provider+date 的所有仍未删除明细（这里只聚合 >24h 的，也就是当天之前的），这个行为是可以接受的——daily_rollups 的累积值应该是幂等的（同一天多次 rollup 结果相同）。

> **重要：** 如果 daily_rollups 中已有旧行且还有更多待聚合的明细，INSERT OR REPLACE 会**覆盖**旧行而非增量更新。为正确处理增量场景，应改用 `INSERT INTO ... ON CONFLICT(provider_name, rollup_date) DO UPDATE SET ...`（SQLite 3.24+ upsert 语法）。

**推荐 upsert 语法（增量安全）：**
```sql
INSERT INTO daily_rollups (
    provider_name, rollup_date,
    request_count, success_count, ...
)
SELECT provider_name, rollup_date, count, success, ...
FROM (
    SELECT ... FROM request_logs WHERE ... GROUP BY ...
)
ON CONFLICT(provider_name, rollup_date) DO UPDATE SET
    request_count               = request_count + excluded.request_count,
    success_count               = success_count + excluded.success_count,
    total_input_tokens          = total_input_tokens + excluded.total_input_tokens,
    ...;
```

---

### Pattern 2: tokio::interval 后台定时任务（Tauri 2 setup 内）

**What:** 在 setup 闭包中 spawn 一个 tokio task，立即执行一次后进入 1 小时 interval 循环。

**When to use:** lib.rs setup 闭包中，TrafficDb manage() 之后。

**Example:**
```rust
// src-tauri/src/lib.rs（setup 闭包内，在 app.manage(traffic_db) 之后）
let app_handle_for_rollup = app.handle().clone();
tauri::async_runtime::spawn(async move {
    use tauri::Manager;
    use std::time::Duration;

    // 立即执行一次
    if let Some(db) = app_handle_for_rollup.try_state::<crate::traffic::TrafficDb>() {
        match db.rollup_and_prune() {
            Ok(_) => log::info!("rollup_and_prune 启动时执行完成"),
            Err(e) => log::warn!("rollup_and_prune 启动时失败（忽略）: {}", e),
        }
    }

    // 每小时重复
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        if let Some(db) = app_handle_for_rollup.try_state::<crate::traffic::TrafficDb>() {
            match db.rollup_and_prune() {
                Ok(_) => log::info!("rollup_and_prune 定时执行完成"),
                Err(e) => log::warn!("rollup_and_prune 定时执行失败（等待下次）: {}", e),
            }
        }
    }
});
```

> **注意：** `tokio::time::interval` 第一次 tick 立即返回（不等待），所以上面的模式是：先手动调一次，再进 interval 循环（interval 第一个 tick 立即触发会导致两次调用）。更清晰的写法是直接用 loop + sleep：

```rust
tauri::async_runtime::spawn(async move {
    loop {
        // 每次循环开始时执行（首次立即执行）
        if let Some(db) = app_handle_for_rollup.try_state::<crate::traffic::TrafficDb>() {
            match db.rollup_and_prune() {
                Ok(_) => log::info!("rollup_and_prune 执行完成"),
                Err(e) => log::warn!("rollup_and_prune 执行失败: {}", e),
            }
        }
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
});
```

---

### Pattern 3: 后端聚合查询 Tauri Commands

**What:** 三个新增后端方法 + 对应 Tauri command，分别返回：供应商聚合（24h）、供应商聚合（7d）、时间趋势（24h 小时粒度 / 7d 天粒度）。

**When to use:** 前端统计分析 Tab 挂载时调用，以及 24h/7d 切换时重新调用。

**聚合数据结构设计：**
```rust
// 供应商聚合行（供 STAT-02）
#[derive(Debug, serde::Serialize)]
pub struct ProviderStat {
    pub provider_name: String,
    pub request_count: i64,
    pub success_count: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub cache_triggered_count: i64,
    pub cache_hit_count: i64,
    pub sum_ttfb_ms: i64,
    pub sum_duration_ms: i64,
}

// 时间趋势点（供 STAT-03 + STAT-04）
#[derive(Debug, serde::Serialize)]
pub struct TimeStat {
    pub label: String,      // "14:00" 或 "2026-03-18"
    pub request_count: i64,
    pub total_tokens: i64,  // input + output 合计
}
```

**SQL 查询示例（24h 按 Provider）：**
```sql
SELECT
    provider_name,
    COUNT(*) AS request_count,
    SUM(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END) AS success_count,
    COALESCE(SUM(input_tokens), 0) AS total_input_tokens,
    COALESCE(SUM(output_tokens), 0) AS total_output_tokens,
    SUM(CASE WHEN cache_creation_tokens > 0 OR cache_read_tokens > 0 THEN 1 ELSE 0 END) AS cache_triggered_count,
    SUM(CASE WHEN cache_read_tokens > 0 THEN 1 ELSE 0 END) AS cache_hit_count,
    COALESCE(SUM(ttfb_ms), 0) AS sum_ttfb_ms,
    COALESCE(SUM(duration_ms), 0) AS sum_duration_ms
FROM request_logs
WHERE created_at >= (strftime('%s', 'now') - 86400) * 1000
GROUP BY provider_name
ORDER BY request_count DESC
```

**SQL 查询示例（24h 按小时聚合，供趋势图）：**
```sql
SELECT
    strftime('%H:00', created_at / 1000, 'unixepoch') AS hour_label,
    COUNT(*) AS request_count,
    COALESCE(SUM(input_tokens), 0) + COALESCE(SUM(output_tokens), 0) AS total_tokens
FROM request_logs
WHERE created_at >= (strftime('%s', 'now') - 86400) * 1000
GROUP BY strftime('%Y-%m-%d %H', created_at / 1000, 'unixepoch')
ORDER BY created_at ASC
```

**SQL 查询示例（7d 从 daily_rollups 按天聚合，供趋势图）：**
```sql
SELECT
    rollup_date AS day_label,
    SUM(request_count) AS request_count,
    SUM(total_input_tokens) + SUM(total_output_tokens) AS total_tokens
FROM daily_rollups
WHERE rollup_date >= strftime('%Y-%m-%d', 'now', '-7 days')
GROUP BY rollup_date
ORDER BY rollup_date ASC
```

---

### Pattern 4: recharts ComposedChart 双轴图（暗色主题适配）

**What:** 左 Y 轴请求数柱状图 + 右 Y 轴 Token 折线图，使用项目 CSS 变量而非硬编码颜色。

**When to use:** `TrafficTrendChart.tsx` 组件内。

**Example:**
```tsx
// Source: recharts 2.x 官方文档 ComposedChart API
import {
  ComposedChart,
  Bar,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";

interface TrendPoint {
  label: string;
  request_count: number;
  total_tokens: number;
}

export function TrafficTrendChart({ data }: { data: TrendPoint[] }) {
  // 使用 CSS 变量引用项目暗色主题颜色（chart-1, chart-2 已在 index.css 定义）
  return (
    <ResponsiveContainer width="100%" height={240}>
      <ComposedChart data={data} margin={{ top: 8, right: 16, bottom: 0, left: 0 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
        <XAxis
          dataKey="label"
          tick={{ fill: "var(--color-muted-foreground)", fontSize: 11 }}
          axisLine={{ stroke: "var(--color-border)" }}
        />
        <YAxis
          yAxisId="requests"
          orientation="left"
          tick={{ fill: "var(--color-muted-foreground)", fontSize: 11 }}
          axisLine={{ stroke: "var(--color-border)" }}
        />
        <YAxis
          yAxisId="tokens"
          orientation="right"
          tick={{ fill: "var(--color-muted-foreground)", fontSize: 11 }}
          axisLine={{ stroke: "var(--color-border)" }}
        />
        <Tooltip
          contentStyle={{
            background: "var(--color-popover)",
            border: "1px solid var(--color-border)",
            borderRadius: "var(--radius-md)",
            color: "var(--color-foreground)",
            fontSize: 12,
          }}
        />
        <Legend
          wrapperStyle={{ color: "var(--color-muted-foreground)", fontSize: 12 }}
        />
        <Bar
          yAxisId="requests"
          dataKey="request_count"
          fill="var(--color-chart-1)"
          radius={[2, 2, 0, 0]}
        />
        <Line
          yAxisId="tokens"
          dataKey="total_tokens"
          stroke="var(--color-chart-2)"
          dot={false}
          strokeWidth={2}
        />
      </ComposedChart>
    </ResponsiveContainer>
  );
}
```

---

### Pattern 5: TrafficPage Tab 重构

**What:** 将 TrafficPage 改为含两个 Tab 的容器，保持实时日志 Tab 现有功能。

**When to use:** `TrafficPage.tsx` 重构。

**Example:**
```tsx
// TabsList variant="line" — 与 SettingsPage 保持视觉一致
<Tabs defaultValue="logs" className="flex flex-col h-full">
  <div className="flex items-center px-6 pt-4 pb-0 gap-4">
    <h2 className="text-lg font-bold">{t("traffic.title")}</h2>
    <TabsList variant="line">
      <TabsTrigger value="logs">{t("traffic.tabLogs")}</TabsTrigger>
      <TabsTrigger value="stats">{t("traffic.tabStats")}</TabsTrigger>
    </TabsList>
  </div>

  <TabsContent value="logs" className="flex flex-col flex-1 min-h-0">
    {/* 现有内容：TrafficStatsBar + TrafficFilter + TrafficTable */}
  </TabsContent>

  <TabsContent value="stats" className="flex-1 min-h-0 overflow-auto px-6 pb-4">
    <StatsAnalysisTab />
  </TabsContent>
</Tabs>
```

---

### Anti-Patterns to Avoid

- **在 tokio::spawn（非 tauri::async_runtime::spawn）内使用 Tauri 事件监听器：** Tauri 2 中在 window listener 内使用 `tokio::spawn` 会 panic（"no reactor running"）。后台任务统一使用 `tauri::async_runtime::spawn`。
- **INSERT OR REPLACE 做增量累加：** `INSERT OR REPLACE` 遇到冲突时删除旧行再插入，不能做增量加法。增量场景必须用 `ON CONFLICT DO UPDATE SET col = col + excluded.col`。
- **在 rollup_and_prune 中直接用 `lock().unwrap()` 然后做耗时事务：** Mutex 持锁期间阻塞其他操作。当前项目是低并发场景（< 10 req/s），单连接 std::sync::Mutex 足够，但事务内不要做任何非 SQL 操作。
- **recharts 使用硬编码颜色字符串：** 暗色主题下颜色不匹配。统一使用 `var(--color-chart-N)` CSS 变量引用。
- **时间戳单位混淆：** `request_logs.created_at` 是 epoch **毫秒**，SQLite `strftime` 接受 epoch **秒**，必须除以 1000：`strftime('%Y-%m-%d', created_at / 1000, 'unixepoch')`。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 趋势图表 | 自定义 SVG 折线 + 柱状 | recharts ComposedChart | 轴刻度计算、tooltip、响应式尺寸等边界情况极多 |
| Tab 组件 | 自定义 div + 状态切换 | 项目已有 shadcn Tabs（line variant） | SettingsPage 已验证，无需重复实现 |
| SQLite upsert | 先 SELECT 再条件 INSERT/UPDATE | `ON CONFLICT DO UPDATE SET` | 原子操作，避免 TOCTOU 竞态 |
| 定时任务 | 自定义计时器逻辑 | `tokio::time::sleep` 循环 | tokio 已有，无需额外依赖 |
| 数字格式化 | 新函数 | `formatTokenCount`（formatters.ts 已有） | 已有且经过测试 |

**Key insight:** 后端无需新增任何 Cargo 依赖；前端只需新增 recharts 一个依赖。

---

## Common Pitfalls

### Pitfall 1: INSERT OR REPLACE 破坏增量聚合

**What goes wrong:** 第一次 rollup 写入 daily_rollups 行后，若同一天同 provider 还有后续未被 rollup 的新记录（因为记录恰好在 24h 边界附近），第二次 rollup 会用 `INSERT OR REPLACE` 删除旧行并插入新行，导致只包含第二次的数据而丢失第一次累积的数据。

**Why it happens:** `INSERT OR REPLACE` 遇到 UNIQUE 冲突时会 DELETE 旧行。

**How to avoid:** 使用 SQLite 3.24+ upsert 语法：`INSERT INTO ... ON CONFLICT(provider_name, rollup_date) DO UPDATE SET request_count = request_count + excluded.request_count, ...`。

**Warning signs:** 某天的统计数字明显偏低，或在 rollup 两次后数据发生变化。

---

### Pitfall 2: epoch 毫秒 vs 秒 混淆

**What goes wrong:** `request_logs.created_at` 存储的是 epoch 毫秒（参见 `log.rs` 中 `LogEntry.created_at: i64`），而 SQLite 的 `strftime('%Y-%m-%d', ts, 'unixepoch')` 和时间比较 `(strftime('%s','now') - 86400)` 都是 epoch 秒。直接用 `created_at` 做时间比较会得到错误结果（全部记录看起来都是未来或极远过去）。

**Why it happens:** JS 的 `Date.now()` 返回毫秒，Rust 的 `SystemTime` 通常也需手动转换。

**How to avoid:** 所有 SQL 中对 `created_at` 均除以 1000：
```sql
WHERE created_at < (strftime('%s', 'now') - 86400) * 1000
strftime('%Y-%m-%d', created_at / 1000, 'unixepoch')
```

**Warning signs:** rollup 后 daily_rollups 为空，或 24h 查询返回全部历史数据。

---

### Pitfall 3: rollup 任务与日志写入 Mutex 死锁

**What goes wrong:** `rollup_and_prune` 持有 `conn.lock().unwrap()` 期间（事务可能较长），如果同时有日志写入 `insert_request_log` 也试图获取同一 Mutex，会阻塞。

**Why it happens:** `TrafficDb.conn` 使用 `std::sync::Mutex<Connection>`，所有方法都获取同一锁。

**How to avoid:** 项目场景是 < 10 req/s，std::sync::Mutex 在 `busy_timeout=5000ms` 配置下不会真正死锁，只是短暂阻塞。这是可接受的，且 CONTEXT.md 已确认此模式。无需特殊处理，但事务内不要执行非 SQL 的耗时操作。

**Warning signs:** 高负载时出现 `MutexGuard` 持锁超时。

---

### Pitfall 4: recharts 2.x 与 React 19 的 peer dependency 警告

**What goes wrong:** `npm install recharts@^2.15` 可能出现 peer dependency 警告，因为 recharts 2.x 的 peerDependencies 声明了 `react >= 16`，而实际运行与 React 19 兼容，但 npm 可能误报。

**Why it happens:** recharts 2.x 的 package.json 未更新 peerDependencies 支持 React 19。

**How to avoid:** 安装时 `npm install recharts@^2.15 --legacy-peer-deps`，或在 package.json 中添加 overrides。实际运行不受影响（recharts 3.x 才是专门为 React 19 重写的，但破坏了 shadcn/ui 兼容性）。

**Warning signs:** `npm install` 时出现 `ERESOLVE` 错误。

---

### Pitfall 5: 趋势图 24 小时数据点稀疏

**What goes wrong:** 24h 模式要显示 24 个小时点，但若某些小时没有请求，SQL 的 `GROUP BY` 不会为空小时生成行，导致图表 X 轴点数不足 24 个，时间轴不连续。

**Why it happens:** SQL GROUP BY 不产生"空"行。

**How to avoid:** 前端接到后端数据后，在展示前填充缺失的小时点（value=0）。可以预先生成 24 个小时标签的骨架数组，再用后端数据覆盖对应项。7d 同理（生成 7 天的骨架）。

**Warning signs:** 图表 X 轴只有 3-5 个点而非 24 个。

---

## Code Examples

### 后端：provider_stats_24h 聚合查询

```rust
// Source: rusqlite 0.38 文档 + SQLite strftime 官方文档
pub fn query_provider_stats_24h(&self) -> rusqlite::Result<Vec<ProviderStat>> {
    let conn = self.conn.lock().unwrap();
    let threshold_ms = (chrono::Utc::now().timestamp() - 86400) * 1000;
    let mut stmt = conn.prepare("
        SELECT
            provider_name,
            COUNT(*) AS request_count,
            SUM(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END),
            COALESCE(SUM(input_tokens), 0),
            COALESCE(SUM(output_tokens), 0),
            SUM(CASE WHEN cache_creation_tokens > 0 OR cache_read_tokens > 0 THEN 1 ELSE 0 END),
            SUM(CASE WHEN cache_read_tokens > 0 THEN 1 ELSE 0 END),
            COALESCE(SUM(ttfb_ms), 0),
            COALESCE(SUM(duration_ms), 0)
        FROM request_logs
        WHERE created_at >= ?1
        GROUP BY provider_name
        ORDER BY request_count DESC
    ")?;
    // ... query_map 映射到 ProviderStat ...
}
```

### 前端：24h 数据点骨架填充

```typescript
// 生成 24 个小时标签，用后端数据填充，缺失的填 0
function buildHourlyData(raw: TimeStat[]): TrendPoint[] {
  const map = new Map(raw.map(r => [r.label, r]));
  return Array.from({ length: 24 }, (_, i) => {
    const label = `${String(i).padStart(2, "0")}:00`;
    const stat = map.get(label);
    return {
      label,
      request_count: stat?.request_count ?? 0,
      total_tokens: stat?.total_tokens ?? 0,
    };
  });
}

// 生成 7 天标签（今天 - 6 天到今天）
function buildDailyData(raw: TimeStat[]): TrendPoint[] {
  const map = new Map(raw.map(r => [r.label, r]));
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date();
    d.setDate(d.getDate() - (6 - i));
    const label = d.toISOString().slice(0, 10);
    const stat = map.get(label);
    return {
      label: label.slice(5), // "MM-DD" 更简洁
      request_count: stat?.request_count ?? 0,
      total_tokens: stat?.total_tokens ?? 0,
    };
  });
}
```

### 前端：Segment 切换按钮组（复用 shadcn Tabs）

```tsx
// Segment 按钮组复用 line 变体 Tabs，不需要 TabsContent 分组
<TabsList variant="line">
  <TabsTrigger
    value="24h"
    onClick={() => setTimeRange("24h")}
    data-state={timeRange === "24h" ? "active" : "inactive"}
  >
    24h
  </TabsTrigger>
  <TabsTrigger
    value="7d"
    onClick={() => setTimeRange("7d")}
    data-state={timeRange === "7d" ? "active" : "inactive"}
  >
    7d
  </TabsTrigger>
</TabsList>
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| recharts 2.x（项目指定） | recharts 3.x 重写 | 2024年底 | 3.x 破坏 shadcn/ui chart，继续使用 2.x |
| tokio::spawn 在 listener 内 | tauri::async_runtime::spawn | Tauri v2 | listener 内用 tokio::spawn 会 panic |
| INSERT OR REPLACE（全量） | ON CONFLICT DO UPDATE（增量）| SQLite 3.24 | 增量 upsert 不丢失历史累积 |
| strftime 不含时区 | 'unixepoch' modifier | SQLite 始终 | 无该修饰符时值被误解析 |

**Deprecated/outdated:**
- recharts 2.x `<ResponsiveContainer>` + `width="100%"` 嵌套模式：3.x 新增 `responsive` prop 替代，但 2.x 中仍然是唯一方式，继续使用。
- `tokio::spawn` 在 Tauri 2 中的 setup 闭包外使用：官方推荐一律改用 `tauri::async_runtime::spawn`。

---

## Open Questions

1. **INSERT OR REPLACE vs ON CONFLICT DO UPDATE 幂等性**
   - What we know: `daily_rollups` 有 `UNIQUE(provider_name, rollup_date)`，每次 rollup 只处理超 24h 的记录
   - What's unclear: 同一天边界（例如今天 00:00 附近）的记录是否会在多次 rollup 中被重复累加
   - Recommendation: 使用 `ON CONFLICT DO UPDATE SET col = col + excluded.col` 保证幂等；如果是全量重聚合（delete 后再 insert），则 `INSERT OR REPLACE` 是安全的，但需在事务内确保 delete 先于 insert 执行

2. **7d 数据源：daily_rollups 首次为空时的 UI 处理**
   - What we know: 新用户或刚安装后 7d 内没有任何 rollup 数据
   - What's unclear: 是否需要回退到 request_logs 查询
   - Recommendation: 显示空状态（与实时日志 TrafficEmptyState 风格一致），不做回退，因为 7d 模式本身就是历史统计视图

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test（内置）|
| Config file | Cargo.toml（无独立 toml 配置） |
| Quick run command | `cargo test -p cli-manager-lib rollup` |
| Full suite command | `cargo test -p cli-manager-lib` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STORE-04 | rollup_and_prune 聚合超 24h 明细到 daily_rollups | unit | `cargo test -p cli-manager-lib rollup::tests::test_rollup_moves_old_logs` | ❌ Wave 0 |
| STORE-04 | rollup_and_prune 删除超 24h 的 request_logs 明细 | unit | `cargo test -p cli-manager-lib rollup::tests::test_prune_deletes_old_logs` | ❌ Wave 0 |
| STORE-04 | rollup_and_prune 删除超 7d 的 daily_rollups | unit | `cargo test -p cli-manager-lib rollup::tests::test_prune_deletes_old_rollups` | ❌ Wave 0 |
| STORE-04 | rollup_and_prune 幂等性（多次调用结果一致） | unit | `cargo test -p cli-manager-lib rollup::tests::test_rollup_idempotent` | ❌ Wave 0 |
| STAT-02 | query_provider_stats_24h 返回正确聚合值 | unit | `cargo test -p cli-manager-lib rollup::tests::test_query_provider_stats_24h` | ❌ Wave 0 |
| STAT-03 | query_hourly_trend 返回 24 个小时点 | unit | `cargo test -p cli-manager-lib rollup::tests::test_query_hourly_trend` | ❌ Wave 0 |
| STAT-04 | 前端 recharts 图表渲染（数据绑定） | 手动验证 | 目测图表正确展示双轴 | N/A |

### Sampling Rate
- **Per task commit:** `cargo test -p cli-manager-lib rollup`
- **Per wave merge:** `cargo test -p cli-manager-lib`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src-tauri/src/traffic/rollup.rs` — 新文件，包含 rollup 方法和 `#[cfg(test)]` 测试模块
- [ ] 测试辅助函数 `make_test_db_with_logs()` — 在 `rollup.rs` 的 tests 模块内定义（与 log.rs 的 `make_test_db` 模式保持一致）

---

## Sources

### Primary (HIGH confidence)
- 项目现有代码：`src-tauri/src/traffic/{schema,log,mod,db}.rs`、`src/components/traffic/TrafficPage.tsx`、`src/components/settings/SettingsPage.tsx`、`src/components/ui/tabs.tsx`
- SQLite 官方文档（strftime、INSERT OR REPLACE、ON CONFLICT）
- tokio 官方文档（`tokio::time::interval`、`spawn`）
- tauri::async_runtime 官方文档（Tauri 2 推荐用法）

### Secondary (MEDIUM confidence)
- recharts 官方文档：[recharts.github.io/en-US/api/ComposedChart](https://recharts.github.io/en-US/api/ComposedChart/)
- recharts 3.x migration guide：[github.com/recharts/recharts/wiki/3.0-migration-guide](https://github.com/recharts/recharts/wiki/3.0-migration-guide)（验证了 2.x vs 3.x 选择）
- npm recharts：最新版本 3.8.0，2.x 最新为 2.15.x（WebSearch 验证）

### Tertiary (LOW confidence)
- shadcn/ui recharts 3 兼容性问题（WebSearch，GitHub issue #9892，暂无官方解决方案）

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 项目已有所有依赖，只新增 recharts；版本信息来自 npm 和官方文档
- Architecture: HIGH — 所有模式均基于项目现有代码推断，SQL 经验证正确
- Pitfalls: HIGH — epoch 单位问题和 INSERT OR REPLACE 问题均为 SQLite 确定行为，tokio spawn 问题来自 Tauri 官方 issue

**Research date:** 2026-03-18
**Valid until:** 2026-06-01（recharts 2.x 稳定期，SQLite 行为不变）
