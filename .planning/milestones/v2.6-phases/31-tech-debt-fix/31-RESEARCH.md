# Phase 31: 补全当前技术债 - Research

**Researched:** 2026-03-19
**Domain:** 代码质量修复 — Rust 后端 API 扩展、前端错误提示、文档修正
**Confidence:** HIGH（所有项通过直接代码检查确认，无需外部研究）

---

## Summary

本次 research 对 v2.6 里程碑审计发现的 6 项技术债务进行了逐一代码检查，找到了每项债务的确切位置、当前实现和所需变更。

在 6 项债务中，**2 项明确建议修复**（item 1 和 item 4），**1 项需修正文档记录**（item 3），**3 项建议标记为"有意设计，不修改"**（item 2、5、6）。原因如下：item 2（STAT-03 以趋势图呈现）在功能上超越了原始需求，合并了 STAT-03 和 STAT-04 的意图，用户已验收；item 5（NoUpstreamConfigured 不记录日志）是有意的——此时上游信息不存在，没有什么可记录的；item 6（流式请求绕过 mpsc）是技术上正确的设计，因为需要同步获取 rowid 以便后续 UPDATE。

**主要建议：** Phase 31 集中修复 item 1（暴露 `total_cache_creation_tokens`）、item 4（DB 初始化失败时前端错误提示）、item 3（修正 SUMMARY.md 文档路径），并为 item 2/5/6 各写一条注释说明有意设计，在对应代码或文档中记录。

---

## 6 项技术债务逐一分析

### 债务 1：`total_cache_creation_tokens` 未暴露到前端（建议修复）

**状态：** 建议 FIX — 数据已存储，只需打通到 API 和前端

**当前位置：**

- DB schema（`src-tauri/src/traffic/schema.rs:40`）：`daily_rollups` 表有 `total_cache_creation_tokens` 列
- rollup 写入（`src-tauri/src/traffic/rollup.rs:59,73`）：正确聚合并 upsert 此字段
- 但 `ProviderStat` 结构体（`rollup.rs:3-16`）缺少此字段
- `query_provider_stats` 两个分支的 SQL 均不 SELECT 此字段（`rollup.rs:106-208`）
- 前端 `ProviderStat` 接口（`src/types/traffic.ts:5-16`）也缺少此字段
- `CacheLeaderboard.tsx` 使用了 `total_cache_read_tokens` 但没有 `total_cache_creation_tokens`

**修复范围（精确）：**

1. `src-tauri/src/traffic/rollup.rs`
   - `ProviderStat` 结构体：新增 `pub total_cache_creation_tokens: i64`
   - `query_provider_stats` 的 "24h" 分支 SQL：在 SELECT 列表中新增 `COALESCE(SUM(cache_creation_tokens), 0) AS total_cache_creation_tokens`（位于 `total_cache_read_tokens` 行之后），列索引从 5 起后移一位（但由于使用命名绑定，需同步更新 `row.get(N)` 索引：原 5=cache_read_tokens → 需在其前插入新字段，调整后续索引）
   - "7d" 分支 SQL：内层两个 UNION ALL 子查询均需新增该列，外层 SELECT 也需加入
   - 两个 `query_map` 闭包中 `Ok(ProviderStat { ... })` 构造处添加新字段

   **具体列索引调整（"24h" 分支）：**
   目前：col 0=provider_name, 1=request_count, 2=success_count, 3=input, 4=output, 5=cache_read_tokens, 6=cache_triggered, 7=cache_hit, 8=sum_ttfb, 9=sum_duration
   修复后插入位置：在 col 4（output）后插入 total_cache_creation_tokens 作为 col 5，原 cache_read_tokens 变为 col 6，以此类推。

2. `src/types/traffic.ts`
   - `ProviderStat` 接口新增 `total_cache_creation_tokens: number`

3. `src/components/traffic/CacheLeaderboard.tsx`（可选）：
   - 可在缓存排行榜中新增"创建 Token"列，让用户看到缓存创建量与读取量的对比
   - 排序键可扩展 `total_cache_creation_tokens`

**注意：** `CacheLeaderboard` 已有独立展示逻辑，修复 item 1 主要是后端 + 类型层补全；前端是否展示此列是 UX 决策，可独立决定。

---

### 债务 2：STAT-03 以趋势图呈现而非独立表格（建议 WON'T FIX）

**状态：** 建议标记为有意设计，不改动

**分析：** STAT-03 要求"按时间聚合表格"，STAT-04 要求"趋势图表"。Phase 30 将两者合并为 `TrafficTrendChart`（recharts ComposedChart），功能上覆盖了 STAT-03 的内容（时间维度聚合数据展示）并以更好的可视化方式呈现。用户在 Phase 30 Plan 03 视觉验收中已 approved。

**行动：** 在 `src/components/traffic/TrafficTrendChart.tsx` 顶部注释中说明"此组件同时实现 STAT-03（时间聚合展示）和 STAT-04（趋势图表），以图表形式替代独立表格，用户已验收"。无需代码修改。

---

### 债务 3：30-03-SUMMARY.md 路径记录错误（建议 FIX — 文档修正）

**状态：** 建议修正文档记录

**当前错误：** `.planning/phases/30-stats-rollup/30-03-SUMMARY.md` 的 `key-files.modified` 节中记录了：
```yaml
- src/locales/zh.json
- src/locales/en.json
```

**实际路径：** 经验证，`src/locales/` 目录不存在，正确路径为：
```
src/i18n/locales/zh.json
src/i18n/locales/en.json
```

**修复：** 直接编辑 `.planning/phases/30-stats-rollup/30-03-SUMMARY.md` 的第 32-33 行，将路径修正为 `src/i18n/locales/zh.json` 和 `src/i18n/locales/en.json`。

这是纯文档修正，无代码变更，5分钟内可完成。

---

### 债务 4：DB 初始化失败时前端展示空页面无错误提示（建议修复）

**状态：** 建议 FIX — 用户体验问题，需向用户传达 DB 不可用信息

**当前行为分析：**

`lib.rs:74-79`：
```rust
if let Some(traffic_db) = traffic::init_traffic_db() {
    app.manage(traffic_db);
    log::info!("traffic.db 初始化成功");
} else {
    log::warn!("traffic.db 不可用，代理将正常工作但不记录流量");
}
```

当 DB 初始化失败，`TrafficDb` 不会注入到 Tauri manage。此时：
- `commands/traffic.rs` 中的命令会因 `tauri::State<'_, TrafficDb>` 找不到状态而**崩溃（panic）**
- 实际上 Tauri 2 的 `State` 宏在状态未 manage 时会 panic，而不是返回错误

**前端表现：** `useTrafficLogs` 调用 `getRecentLogs(100)` 失败，catch 中 `console.error` 后 `setLoading(false)`，页面显示 `TrafficEmptyState`（空状态组件），用户无法区分"无日志"和"DB 故障"。`useTrafficStats` 类似，拉取失败后显示空状态图标。

**修复方案（两步）：**

**步骤 1：后端 — 安全处理 DB 不可用**

目前 `commands/traffic.rs` 中的命令使用 `tauri::State<'_, TrafficDb>` 会在 DB 未初始化时 panic。有两种安全方案：
- 方案 A（推荐）：用 `app_handle.try_state::<TrafficDb>()` 替代直接 State 注入，返回 `Result<_, String>`（携带 "数据库不可用" 错误消息）
- 方案 B：在 setup 时始终 manage 一个 `Option<TrafficDb>` 封装类型

**方案 A 实现：**
```rust
// commands/traffic.rs 修改后
#[tauri::command]
pub async fn get_recent_logs(
    app_handle: tauri::AppHandle,
    limit: Option<i64>,
) -> Result<Vec<crate::traffic::log::TrafficLogPayload>, String> {
    use tauri::Manager;
    let db = app_handle
        .try_state::<crate::traffic::TrafficDb>()
        .ok_or_else(|| "数据库不可用（DB 初始化失败，请检查磁盘空间和权限）".to_string())?;
    let limit = limit.unwrap_or(100).min(1000);
    db.query_recent_logs(limit).map_err(|e| e.to_string())
}
// get_provider_stats 和 get_time_trend 同理
```

**步骤 2：前端 — 区分"无数据"和"DB 故障"**

在 `useTrafficLogs` 和 `useTrafficStats` 中区分错误类型，将 DB 不可用的错误消息向上传递。在 `TrafficPage` 中展示专门的错误提示（可使用 Sonner toast 或内联错误卡片）。

建议：`useTrafficLogs` 返回 `{ logs, loading, dbError: string | null }`，错误时展示一个 warning banner（而非空状态），提示用户"流量数据库不可用，代理功能正常但不记录流量"。

**工作量评估：** 后端 3 个函数修改约 30 行，前端 hook + 展示约 40 行，合计约 70 行有效代码修改。

---

### 债务 5：NoUpstreamConfigured 分支不记录日志（建议 WON'T FIX）

**状态：** 有意设计，不修改

**分析：** `proxy/handler.rs:447-450` 中，当 `state.get_upstream().await` 返回 `None` 时，直接返回 `ProxyError::NoUpstreamConfigured`，不发送日志。此时：
- 没有可用的 `UpstreamTarget`（无 provider_name, api_key 等）
- `LogEntry` 的 `provider_name` 是非空字符串（NOT NULL），无法填写有意义的值
- 这种情况发生在用户未配置 Provider 即使用代理时，属于配置错误，非请求错误

**行动：** 在 `proxy/handler.rs` 对应位置（步骤 A 注释之后）添加一条注释说明此设计意图，同时在技术债务文档中标注为"有意设计"。

---

### 债务 6：流式请求绕过 mpsc log_worker 直接 INSERT（建议 WON'T FIX）

**状态：** 技术上正确的设计，不修改

**分析：** `proxy/handler.rs:806-831` 的流式分支直接调用 `db.insert_request_log(&entry)` 而不走 `log_tx.try_send(entry)` 的原因是**技术约束**：
- 流式请求在 stream EOF 后需要 UPDATE 同一行的 token/duration 字段
- 这要求在 INSERT 后立即获取 SQLite `rowid`（`last_insert_rowid`）
- mpsc log_worker 是异步 fire-and-forget 模式，无法同步返回 rowid

因此方案 C（直接 INSERT）是唯一能满足此需求的技术方案，与 STORE-03 "通过 mpsc channel 写入" 的描述有偏差，但这是因为 STORE-03 当时未预见流式请求需要 rowid 回路。功能结果等价且经过测试验证。

**行动：** 在 `proxy/handler.rs:806-807` 处已有注释说明原因，可适当扩充注释内容，并在 STATE.md 中更新对应决策条目（已有记录：`[Phase 28-02]: 流式请求跳过 log_worker 采用方案 C`）。

---

## 修复优先级矩阵

| 项 | 操作 | 影响面 | 工作量 | 优先级 |
|----|------|--------|--------|--------|
| 1 (cache_creation_tokens) | FIX — 后端 + 前端类型 | rollup.rs + traffic.ts | 中（~50行） | P1 |
| 4 (DB 初始化失败提示) | FIX — 后端 + 前端 | commands/traffic.rs + hooks | 中（~70行） | P1 |
| 3 (SUMMARY 路径错误) | FIX — 文档 | 30-03-SUMMARY.md | 小（2行） | P2 |
| 2 (STAT-03 趋势图) | WON'T FIX — 注释说明 | TrafficTrendChart.tsx | 极小（注释） | P3 |
| 5 (NoUpstream 不记录) | WON'T FIX — 注释说明 | handler.rs | 极小（注释） | P3 |
| 6 (流式绕过 mpsc) | WON'T FIX — 注释说明 | 已有注释，可扩充 | 极小（注释） | P3 |

---

## Standard Stack

本 phase 不引入新依赖，仅修改现有代码。

### 现有技术栈（相关部分）

| 技术 | 版本 | 用途 |
|------|------|------|
| rusqlite | 现有 | SQLite 读写，query_map 行映射 |
| Tauri 2 `try_state` | 现有 | 安全获取可选状态，不 panic |
| React + TypeScript | 现有 | 前端类型扩展和 hook 修改 |
| tauri-apps/api/event | 现有 | 前端事件监听 |

---

## Architecture Patterns

### 模式 1：Tauri 可选状态访问（item 4 的关键模式）

**当前（有问题）：** 使用 `tauri::State<'_, TrafficDb>` — 若 DB 未 manage，会在调用时 panic

**正确：** 使用 `app_handle.try_state::<TrafficDb>()` 返回 `Option`，转换为 `Result<_, String>`：

```rust
// 安全模式：DB 不可用时返回描述性错误，不 panic
use tauri::Manager;
let db = app_handle
    .try_state::<crate::traffic::TrafficDb>()
    .ok_or_else(|| "数据库不可用".to_string())?;
```

注意：`lib.rs` 中的 rollup 定时任务已经使用了 `try_state` 模式（`lib.rs:105`），这是正确的参考。

### 模式 2：Rust 结构体字段新增（item 1）

新增字段需同时修改：
1. 结构体定义（`ProviderStat`）
2. SQL SELECT 列（两个分支各自的内层和外层 SELECT）
3. `row.get(N)` 索引（N 值需随插入位置调整）
4. TypeScript 接口定义（保持前后端同步）

字段插入位置建议：`total_cache_read_tokens` 之前，保持"creation 在前，read 在后"的逻辑顺序。

---

## Don't Hand-Roll

| 问题 | 不要自行构建 | 使用现有 |
|------|------------|--------|
| DB 状态可选访问 | 自定义 Option<TrafficDb> 包装类型 | `app_handle.try_state::<TrafficDb>()` |
| 前端错误展示 | 自定义错误组件 | 现有 `sonner` toast 或内联警告 banner |

---

## Common Pitfalls

### 陷阱 1：`row.get(N)` 索引偏移（item 1 修复风险）

**问题：** 在 SQL SELECT 列表中插入新字段后，后续所有 `row.get(N)` 调用的索引 N 必须相应调整。如果遗漏，会出现字段取错值的静默 bug（rusqlite 不会报列名不匹配的错误，只会取错列）。

**预防：** 修改后在测试中验证每个字段值与预期值匹配，不仅仅验证结果是 Ok。

### 陷阱 2：`try_state` 返回的是引用，不能在 async 块中跨 await 持有

**问题：** `app_handle.try_state::<TrafficDb>()` 返回 `Option<tauri::State<'_, TrafficDb>>`，其生命周期绑定到 `app_handle`。在 async command 中这是安全的（因为 command 的 `app_handle: tauri::AppHandle` 生命周期足够长）。

**预防：** 确保 `db` 变量作用域内完成所有数据库操作，不需要跨 await 边界持有。

### 陷阱 3：修改 rollup.rs 后忘记更新测试中的 ProviderStat 构造

**问题：** `rollup.rs` 中的测试直接构造或检查 `ProviderStat` 字段，新增字段后如果测试中的插入语句或断言未更新，编译会失败（Rust 结构体完整性检查）。

**预防：** 这是好的 — Rust 编译器会提醒遗漏的字段。照着编译错误逐一修复即可。

---

## Code Examples

### 示例 1：item 4 的 try_state 模式（参考 lib.rs:105 现有用法）

```rust
// 来源：src-tauri/src/lib.rs:105（已有正确用法）
if let Some(db) = app_handle_for_rollup.try_state::<crate::traffic::TrafficDb>() {
    match db.rollup_and_prune() {
        Ok(_) => log::info!("rollup_and_prune 执行完成"),
        Err(e) => log::warn!("rollup_and_prune 执行失败: {}", e),
    }
}
```

commands/traffic.rs 修改后应同样使用此模式。

### 示例 2：前端 hook 错误状态传递（item 4）

```typescript
// useTrafficLogs 修改方向（当前无 dbError 字段）
export function useTrafficLogs(): {
  logs: TrafficLog[];
  loading: boolean;
  dbError: string | null;  // 新增
} {
  const [dbError, setDbError] = useState<string | null>(null);
  // ...
  getRecentLogs(100)
    .then(...)
    .catch((err) => {
      setDbError(String(err));  // 传递给组件层
    });
}
```

---

## State of the Art

| 旧方式 | 当前方式 | 说明 |
|--------|----------|------|
| `tauri::State<'_, T>` 直接注入 | `app_handle.try_state::<T>()` 安全访问 | 当状态可能不存在时用 try_state |
| DB 失败仅写 log | DB 失败向前端返回结构化错误 | 用户体验要求 |

---

## Open Questions

1. **item 4：前端错误展示样式**
   - 已知：需要区分"无数据"和"DB 故障"
   - 待定：用 sonner toast 还是内联 banner？建议内联 banner（不会自动消失，用户能持续感知 DB 状态）
   - 建议：planning 阶段决定样式方向，research 不做硬性规定

2. **item 1：CacheLeaderboard 是否展示 total_cache_creation_tokens**
   - 已知：数据会出现在 ProviderStat 中
   - 待定：CacheLeaderboard 是否新增"缓存创建 Token"列
   - 建议：planning 阶段根据 UI 空间决定；数据层的修复（Rust + TS 类型）不受此影响

---

## Validation Architecture

`workflow.nyquist_validation` 在 `.planning/config.json` 中为 `true`，需包含此节。

### Test Framework

| 属性 | 值 |
|------|-----|
| 框架 | Rust 内置测试（`cargo test`）+ TypeScript tsc 类型检查 |
| 配置文件 | `src-tauri/Cargo.toml`（Rust），`tsconfig.json`（TS） |
| 快速运行 | `cd src-tauri && cargo test traffic` |
| 完整套件 | `cd src-tauri && cargo test && cd .. && npx tsc --noEmit` |

### Phase Requirements → Test Map

本 phase 无正式 REQ-ID，但每个修复项有对应验证点：

| 修复项 | 验证行为 | 测试类型 | 自动化命令 | 测试存在？ |
|--------|----------|----------|-----------|-----------|
| item 1: ProviderStat 新增字段 | SQL 返回 total_cache_creation_tokens 正确值 | unit | `cd src-tauri && cargo test test_query_provider_stats` | 部分（现有测试需扩展） |
| item 1: TS 类型同步 | tsc --noEmit 零错误 | type-check | `npx tsc --noEmit` | ✅ |
| item 4: DB 不可用返回错误 | get_recent_logs 返回 Err("数据库不可用...") | unit | `cd src-tauri && cargo test test_traffic_commands` | ❌ Wave 0 需新建 |
| item 3: 文档路径修正 | 目视校验 | manual-only | N/A | N/A |
| item 2/5/6: 注释说明 | 代码注释存在 | manual-only | N/A | N/A |

### Sampling Rate

- **每次任务提交：** `cd src-tauri && cargo test traffic -- --nocapture 2>&1 | tail -20`
- **波次合并：** `cd src-tauri && cargo test && cd .. && npx tsc --noEmit`
- **Phase gate：** 完整套件绿色后提交 `/gsd:verify-work`

### Wave 0 Gaps

- [ ] item 4 需新建测试：验证 `get_recent_logs`/`get_provider_stats`/`get_time_trend` 在 TrafficDb 未注册时返回 Err 而非 panic。路径建议：在 `src-tauri/src/commands/traffic.rs` 同文件 `#[cfg(test)]` 模块中新建 mock 测试，或在集成测试中添加。
- [ ] item 1 现有测试 `test_query_provider_stats_24h` / `test_query_provider_stats_7d` 需扩展：验证 `total_cache_creation_tokens` 字段值正确

---

## Sources

### Primary (HIGH confidence)

- 直接代码检查：`src-tauri/src/traffic/rollup.rs`（ProviderStat 结构体，SQL 查询）
- 直接代码检查：`src-tauri/src/traffic/db.rs`（open_traffic_db，降级策略）
- 直接代码检查：`src-tauri/src/traffic/mod.rs`（init_traffic_db）
- 直接代码检查：`src-tauri/src/lib.rs`（DB manage 逻辑，try_state 用法）
- 直接代码检查：`src-tauri/src/commands/traffic.rs`（get_recent_logs 等 3 个 command）
- 直接代码检查：`src-tauri/src/proxy/handler.rs`（NoUpstreamConfigured 分支，流式直接 INSERT 分支）
- 直接代码检查：`src/types/traffic.ts`（ProviderStat 接口）
- 直接代码检查：`src/components/traffic/CacheLeaderboard.tsx`（缓存排行榜）
- 直接代码检查：`src/hooks/useTrafficLogs.ts` 和 `useTrafficStats.ts`（错误处理现状）
- 文件系统验证：`src/locales/` 不存在，正确路径为 `src/i18n/locales/`
- `.planning/phases/30-stats-rollup/30-03-SUMMARY.md`（确认路径错误位置）

---

## Metadata

**Confidence breakdown:**
- 债务项定位：HIGH — 所有文件直接检查
- 修复方案：HIGH — 基于现有代码模式（try_state 在 lib.rs 已用，结构扩展是标准 Rust 模式）
- 工作量估算：MEDIUM — 未实际计算代码行数，基于经验判断

**Research date:** 2026-03-19
**Valid until:** 90 天（代码库稳定，无外部依赖变化）
