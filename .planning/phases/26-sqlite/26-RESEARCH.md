# Phase 26: SQLite 基础设施 - Research

**Researched:** 2026-03-18
**Domain:** Rust + rusqlite + rusqlite_migration + Tauri 托管状态
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **不记录请求/响应内容**（零隐私风险，零存储开销）
- **stop_reason/finish_reason** 保留为元数据（COLLECT-04 要求）
- 额外增加 `upstream_model` 列（模型映射后的实际上游模型名）
- 额外增加 `protocol_type` 列（Anthropic/OpenAiChatCompletions/OpenAiResponses）
- `request_logs` 用 `provider_name TEXT` 存储名称快照，无需 UUID 引用
- CLI 字段按端口号推断（15800=claude-code, 15801=codex），存为 TEXT
- **TTFB (ttfb_ms)**：代理向上游发出请求 → 收到第一个字节，毫秒 INTEGER/i64
- **Duration (duration_ms)**：handler 全生命周期，毫秒 INTEGER/i64
- token/sec 前端实时计算，不存 DB
- `cache_creation_tokens` 和 `cache_read_tokens` 两列预留
- `request_logs.created_at`：Unix epoch 毫秒 INTEGER/i64
- `daily_rollups.rollup_date`：TEXT YYYY-MM-DD，UNIQUE(provider_name, rollup_date)
- rollup 粒度：按 Provider+天，10 个聚合字段（request_count, success_count, total_input_tokens, total_output_tokens, total_cache_creation_tokens, total_cache_read_tokens, cache_triggered_count, cache_hit_count, sum_ttfb_ms, sum_duration_ms）
- **DB 损坏恢复**：静默删除并重建空表
- **DB 初始化失败**：降级运行，代理正常工作但不记录流量，记录日志警告

### Claude's Discretion

- rusqlite_migration 具体版本和用法
- 索引设计（哪些列需要索引）
- Arc<std::sync::Mutex<Connection>> 的具体封装方式
- DB 文件命名（traffic.db 或其他）
- 各协议缓存字段的具体提取位置（研究阶段确认）

### Deferred Ideas (OUT OF SCOPE)

- 供应商排行榜和缓存命中率排行榜的 UI 设计（Phase 29/30 前端页面）
- 流式 SSE 的缓存字段提取（Phase 28）
- 费用估算（cost_usd）— v2.7+ (ADV-01)
- first_token_ms 精确指标（区别于 TTFB）— v2.7+ (ADV-04)

</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| STORE-01 | 应用启动时自动初始化 SQLite 数据库（WAL 模式，路径在 app_data_dir，非 iCloud） | dirs::data_local_dir() → ~/Library/Application Support，不经 iCloud；rusqlite bundled + WAL pragma |
| STORE-02 | schema 迁移机制确保未来加字段安全（rusqlite_migration） | rusqlite_migration 2.4.1，user_version pragma，Migrations::from_slice + to_latest() |

</phase_requirements>

---

## Summary

Phase 26 需要为后续所有流量监控功能搭建 SQLite 基础设施：在非 iCloud 路径初始化 traffic.db，启用 WAL 模式，建立 request_logs 和 daily_rollups 两张表及其索引，并通过 rusqlite_migration 管理 schema 版本。

技术选型已在 CONTEXT.md 中锁定：rusqlite（bundled feature）+ rusqlite_migration。连接模型为单个 `std::sync::Mutex<Connection>`，通过 Tauri 的 `.manage()` 注入为全局状态。项目已有 `dirs` crate（v5.0，已在 Cargo.toml），`dirs::data_local_dir()` 在 macOS 返回 `~/Library/Application Support`，不走 iCloud。

DB 文件路径确定为 `~/Library/Application Support/com.climanager.app/traffic.db`（production，按 Tauri bundle identifier）或直接使用 `dirs::data_local_dir()` 手动拼接。对于模块组织，参考 `proxy/` 的按功能子目录拆分模式，新建 `traffic/` 模块。

**Primary recommendation:** 在 `lib.rs` setup 闭包中初始化 DB（proxy restore 之前），`TrafficDb` 结构体包裹 `Mutex<Connection>`，通过 `.manage()` 注入，`traffic/` 子模块按 schema/db 分文件组织。

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rusqlite | 0.38.0 | SQLite 绑定（bundled feature 内嵌 SQLite 3.51.1） | 40M+ 下载，项目已决策，bundled 消除系统 sqlite 版本依赖 |
| rusqlite_migration | 2.4.1 | Schema 版本管理，user_version pragma | 项目已决策，无 CLI 依赖，纯 Rust，API 极简 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| dirs | 5.0（已有） | 获取平台标准目录路径 | 拼接 DB 文件路径，替代 Tauri PathResolver（无需 AppHandle） |
| thiserror | 2.0（已有） | 统一错误类型 | DB 初始化/操作错误封装 |
| std::sync::Mutex | stdlib | 单连接保护 | rusqlite Connection 是 Send but !Sync，需要 Mutex 做内部可变性 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| dirs::data_local_dir() | Tauri app.path().app_local_data_dir() | Tauri 方式需要 AppHandle 才能调用，dirs 可在任何上下文使用；两者 macOS 结果相同（Application Support）。选 dirs 保持与现有代码一致 |
| std::sync::Mutex | tokio::sync::Mutex | rusqlite 是同步 API，std Mutex 更高效，无 async 过头；Tauri 文档推荐 std::sync::Mutex 用于 manage() |
| 单连接 Mutex<Connection> | r2d2 连接池 | < 10 req/s 无需连接池；连接池增加复杂度和依赖；CONTEXT.md 已决策单连接 |

**Installation:**

```toml
# 在 src-tauri/Cargo.toml [dependencies] 中添加：
rusqlite = { version = "0.38", features = ["bundled"] }
rusqlite_migration = "2.4"
```

## Architecture Patterns

### Recommended Project Structure

```
src-tauri/src/
├── traffic/
│   ├── mod.rs        # TrafficDb 结构体，pub use，初始化入口
│   ├── schema.rs     # MIGRATIONS 常量定义（M 切片），CREATE TABLE SQL
│   └── db.rs         # 路径解析 get_traffic_db_path()，open_traffic_db()
└── lib.rs            # setup 闭包中调用 traffic::init_traffic_db(app)
```

### Pattern 1: TrafficDb 托管状态

**What:** 将 `Mutex<Connection>` 包裹在 newtype 中，通过 `.manage()` 注入 Tauri 状态

**When to use:** Phase 27+ 的写入命令和代理 handler 通过 `State<TrafficDb>` 或 `app_handle.state::<TrafficDb>()` 访问

**Example:**

```rust
// traffic/mod.rs
use std::sync::Mutex;
use rusqlite::Connection;

/// DB 连接托管状态
pub struct TrafficDb {
    pub conn: Mutex<Connection>,
}

// lib.rs setup 闭包中注入
match traffic::init_traffic_db(&app) {
    Ok(db) => { app.manage(db); }
    Err(e) => {
        log::error!("traffic.db 初始化失败，降级运行: {}", e);
        // 不调用 manage()，代理正常工作，Phase 27 写入时检查 state 是否存在
    }
}
```

### Pattern 2: rusqlite_migration 迁移定义

**What:** 静态切片定义 SQL 迁移，`to_latest()` 在每次启动时幂等应用

**When to use:** 每次 DB 打开后调用，应用 schema 变更（新列、新表）

**Example:**

```rust
// traffic/schema.rs
// Source: https://docs.rs/rusqlite_migration/latest/rusqlite_migration/
use rusqlite_migration::{Migrations, M};

const MIGRATIONS_SLICE: &[M<'_>] = &[
    M::up("
        CREATE TABLE IF NOT EXISTS request_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at INTEGER NOT NULL,
            provider_name TEXT NOT NULL,
            cli_id TEXT NOT NULL,
            method TEXT NOT NULL,
            path TEXT NOT NULL,
            status_code INTEGER,
            is_streaming INTEGER NOT NULL DEFAULT 0,
            request_model TEXT,
            upstream_model TEXT,
            protocol_type TEXT NOT NULL,
            input_tokens INTEGER,
            output_tokens INTEGER,
            cache_creation_tokens INTEGER,
            cache_read_tokens INTEGER,
            ttfb_ms INTEGER,
            duration_ms INTEGER,
            stop_reason TEXT,
            error_message TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_request_logs_created_at
            ON request_logs (created_at);
        CREATE INDEX IF NOT EXISTS idx_request_logs_provider_name
            ON request_logs (provider_name);
        CREATE TABLE IF NOT EXISTS daily_rollups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_name TEXT NOT NULL,
            rollup_date TEXT NOT NULL,
            request_count INTEGER NOT NULL DEFAULT 0,
            success_count INTEGER NOT NULL DEFAULT 0,
            total_input_tokens INTEGER NOT NULL DEFAULT 0,
            total_output_tokens INTEGER NOT NULL DEFAULT 0,
            total_cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
            total_cache_read_tokens INTEGER NOT NULL DEFAULT 0,
            cache_triggered_count INTEGER NOT NULL DEFAULT 0,
            cache_hit_count INTEGER NOT NULL DEFAULT 0,
            sum_ttfb_ms INTEGER NOT NULL DEFAULT 0,
            sum_duration_ms INTEGER NOT NULL DEFAULT 0,
            UNIQUE(provider_name, rollup_date)
        );
        CREATE INDEX IF NOT EXISTS idx_daily_rollups_date
            ON daily_rollups (rollup_date);
    "),
];

pub const MIGRATIONS: Migrations<'static> = Migrations::from_slice(MIGRATIONS_SLICE);
```

### Pattern 3: WAL 模式 + busy_timeout Pragma

**What:** 打开连接后立即执行 pragma 配置

**When to use:** 每次 `Connection::open()` 之后，迁移之前

**Example:**

```rust
// traffic/db.rs
// Source: https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html
use rusqlite::Connection;

pub fn configure_connection(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode=WAL;
        PRAGMA busy_timeout=5000;
        PRAGMA synchronous=NORMAL;
        PRAGMA foreign_keys=ON;
    ")
}
```

注意：`journal_mode=WAL` 持久化存储在 DB 文件，只需设一次；`busy_timeout` 是连接级别参数，每次连接都必须设置。

### Pattern 4: DB 路径解析

**What:** 使用 `dirs::data_local_dir()` 获取 `~/Library/Application Support`，拼接 app bundle identifier 子目录

**When to use:** DB 初始化路径确定，保证非 iCloud

**Example:**

```rust
// traffic/db.rs
use std::path::PathBuf;

pub fn get_traffic_db_path() -> PathBuf {
    let base = dirs::data_local_dir()
        .expect("无法获取本地数据目录");
    // macOS: ~/Library/Application Support/com.climanager.app/traffic.db
    base.join("com.climanager.app").join("traffic.db")
}
```

### Pattern 5: 损坏恢复

**What:** 打开失败或迁移失败时删除并重建

**When to use:** CONTEXT.md 决策：损坏/schema 不兼容时静默重建

**Example:**

```rust
pub fn open_traffic_db(path: &PathBuf) -> Result<Connection, rusqlite::Error> {
    // 尝试打开并迁移
    let result = try_open_and_migrate(path);
    if result.is_err() {
        log::warn!("traffic.db 损坏或不兼容，删除重建: {:?}", path);
        let _ = std::fs::remove_file(path);
        try_open_and_migrate(path)
    } else {
        result
    }
}
```

### Anti-Patterns to Avoid

- **直接存 `Connection` 而不用 Mutex**：`Connection` 是 `Send` 但非 `Sync`，多线程访问需要 `Mutex`
- **在 setup 闭包内用 `?` 传播 DB 初始化错误**：会导致应用启动失败；改为降级运行（记录 error 日志，不 manage）
- **每次写入重新打开连接**：性能差，WAL 优势消失；复用单连接
- **tokio::sync::Mutex 用于 rusqlite**：过度使用 async mutex；rusqlite 是同步 API，用 std::sync::Mutex
- **不设 busy_timeout**：默认 0ms，首次锁竞争立即返回 `SQLITE_BUSY` 错误

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Schema 版本追踪 | 自定义 migrations 表 | rusqlite_migration | 正确处理多并发版本、回滚、部分迁移；user_version pragma 更轻量无需 SQL 表 |
| WAL 锁等待 | 自定义重试逻辑 | `PRAGMA busy_timeout=5000` | SQLite 内置，毫秒精度，正确处理 SQLITE_BUSY_RECOVERY |
| 损坏检测 | PRAGMA integrity_check 手动 | 尝试打开+迁移，失败则重建 | CONTEXT.md 已决策此策略，简单可靠 |

**Key insight:** rusqlite_migration 的 user_version 方案比自定义 migrations 表更轻（仅整数读写），且无需解析 SQL 表。

## Common Pitfalls

### Pitfall 1: DB 路径含 iCloud/Mobile Documents

**What goes wrong:** `~/Library/Mobile Documents/` 路径 + WAL 文件 = iCloud 最终一致性冲突，DB 损坏
**Why it happens:** iCloud Drive 会同步 WAL 文件但不保证原子性
**How to avoid:** 使用 `dirs::data_local_dir()`（返回 `~/Library/Application Support`），不使用 icloud.rs 中的路径
**Warning signs:** 路径字符串包含 "Mobile Documents" 或 "CloudDocs"

### Pitfall 2: busy_timeout 未设置导致 SQLITE_BUSY

**What goes wrong:** Phase 27 后台写入 task 和 Tauri 命令同时访问 DB，概率性 `database is locked` 错误
**Why it happens:** busy_timeout 是连接级别参数，默认 0ms，不会等待锁释放
**How to avoid:** 每次 `Connection::open()` 之后立即调用 `PRAGMA busy_timeout=5000`
**Warning signs:** 测试中偶发 `SqliteFailure { code: DatabaseBusy }` 错误

### Pitfall 3: setup 闭包 ? 传播 DB 错误导致启动失败

**What goes wrong:** DB 初始化错误（磁盘写保护、权限不足）导致整个 Tauri 应用 panic
**Why it happens:** `setup` 返回 `Result<(), Box<dyn Error>>`，`?` 会传播错误
**How to avoid:** 用 `match` 捕获 DB 初始化错误，降级运行（记录 warn 日志，跳过 `.manage()`）
**Warning signs:** 应用在某些用户机器上完全无法启动

### Pitfall 4: journal_mode WAL 与 busy_timeout 混淆持久性

**What goes wrong:** 认为 busy_timeout 和 journal_mode 一样持久化，每次打开后只设一次
**Why it happens:** journal_mode=WAL 确实持久化在 DB 文件，但 busy_timeout 是连接级参数，每次 open 后需重新设置
**How to avoid:** 在 `configure_connection()` 函数中始终同时设置两者
**Warning signs:** 重启后 busy_timeout 失效（默认回到 0ms）

### Pitfall 5: rusqlite_migration MIGRATIONS 常量跨 crate 生命周期问题

**What goes wrong:** `Migrations::from_slice()` 接受 `&[M<'_>]`，若切片生命周期不正确，编译报错
**Why it happens:** 常量初始化的生命周期推导
**How to avoid:** 使用 `const MIGRATIONS_SLICE: &[M<'static>]` 和 `const MIGRATIONS: Migrations<'static>`
**Warning signs:** 编译器报 `lifetime may not live long enough` 错误

## Code Examples

验证自官方文档的完整初始化流程：

### DB 初始化完整流程

```rust
// traffic/db.rs
// Source: https://docs.rs/rusqlite/latest/rusqlite/
// Source: https://docs.rs/rusqlite_migration/latest/rusqlite_migration/
use std::path::PathBuf;
use rusqlite::Connection;
use crate::traffic::schema::MIGRATIONS;

pub fn get_traffic_db_path() -> PathBuf {
    let base = dirs::data_local_dir().expect("无法获取本地数据目录");
    base.join("com.climanager.app").join("traffic.db")
}

fn configure_connection(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode=WAL;
        PRAGMA busy_timeout=5000;
        PRAGMA synchronous=NORMAL;
        PRAGMA foreign_keys=ON;
    ")
}

fn try_open_and_migrate(path: &PathBuf) -> Result<Connection, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(path.parent().unwrap())?;
    let mut conn = Connection::open(path)?;
    configure_connection(&conn)?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(conn)
}

pub fn open_traffic_db() -> Option<Connection> {
    let path = get_traffic_db_path();
    match try_open_and_migrate(&path) {
        Ok(conn) => Some(conn),
        Err(e) => {
            log::warn!("traffic.db 初始化失败，尝试重建: {}", e);
            let _ = std::fs::remove_file(&path);
            match try_open_and_migrate(&path) {
                Ok(conn) => Some(conn),
                Err(e2) => {
                    log::error!("traffic.db 重建失败，降级运行（不记录流量）: {}", e2);
                    None
                }
            }
        }
    }
}
```

### lib.rs setup 集成

```rust
// lib.rs setup 闭包（DB 初始化插入位置：watcher 之后，proxy restore 之前）
// Source: 现有 lib.rs setup 闭包结构，新增 DB 初始化

// 初始化 traffic DB
match traffic::open_traffic_db() {
    Some(conn) => {
        app.manage(traffic::TrafficDb {
            conn: std::sync::Mutex::new(conn),
        });
        log::info!("traffic.db 初始化成功");
    }
    None => {
        log::warn!("traffic.db 不可用，代理将正常工作但不记录流量");
    }
}
```

### 迁移验证测试

```rust
// traffic/schema.rs 或 traffic/mod.rs 的 tests 模块
// Source: https://docs.rs/rusqlite_migration/latest/rusqlite_migration/
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        assert!(MIGRATIONS.validate().is_ok());
    }

    #[test]
    fn migrations_create_expected_tables() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        MIGRATIONS.to_latest(&mut conn).unwrap();

        // 验证 request_logs 表存在
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='request_logs'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // 验证 daily_rollups 表存在
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='daily_rollups'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| rusqlite 0.31.x | rusqlite 0.38.0（内嵌 SQLite 3.51.1） | 2025-12 | 更新 SQLite 版本，API 无破坏性变更 |
| 自定义 migrations 表 | rusqlite_migration user_version pragma | N/A | 更轻量，无需 SQL 表解析 |
| tokio::sync::Mutex 用于 DB | std::sync::Mutex | Tauri 2 最佳实践 | 性能更好，适合同步 API |

**Deprecated/outdated:**
- `tokio::sync::Mutex<Connection>`：rusqlite 是同步 API，async mutex 是过度工程，可能导致 deadlock

## Open Questions

1. **DB 文件路径：`dirs::data_local_dir()` vs Tauri `app.path().app_local_data_dir()`**
   - What we know：两者在 macOS 都返回 `~/Library/Application Support`；Tauri 版本自动追加 bundle identifier 子目录（`com.climanager.app`）；dirs 版本需要手动拼接
   - What's unclear：Tauri 版本在 setup 闭包中可用（有 `app` 引用），但在 `traffic/db.rs` 独立函数中需要传入 AppHandle
   - Recommendation：在 setup 闭包中获取路径传给 `open_traffic_db(path)` 函数，或用 dirs 拼接（保持与现有 storage 模块一致的 `dirs` 风格）

2. **`TrafficDb` 降级时，Phase 27 如何安全访问状态**
   - What we know：Tauri `State<T>` 在 manage() 未调用时会 panic
   - What's unclear：Phase 27 handler 如何区分"DB 不可用"和"DB 正常"
   - Recommendation：使用 `Option<TrafficDb>` 包装，或检查 `app_handle.try_state::<TrafficDb>()`（Tauri 2 提供 try_state 方法避免 panic）

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust 内置 `#[cfg(test)]` + `cargo test` |
| Config file | 无独立配置，Cargo.toml `[dev-dependencies]` |
| Quick run command | `cargo test -p cli-manager-lib traffic -- --nocapture` |
| Full suite command | `cargo test -p cli-manager-lib` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STORE-01 | DB 文件在 Application Support 创建，路径不含 iCloud | unit | `cargo test -p cli-manager-lib traffic::db::tests` | Wave 0 |
| STORE-01 | WAL 模式启用（journal_mode = wal） | unit | `cargo test -p cli-manager-lib traffic::db::tests::test_wal_mode` | Wave 0 |
| STORE-01 | busy_timeout 已设置（连接可重复使用） | unit | `cargo test -p cli-manager-lib traffic::db::tests::test_pragma_config` | Wave 0 |
| STORE-02 | MIGRATIONS 对象通过 validate() 检验 | unit | `cargo test -p cli-manager-lib traffic::schema::tests::migrations_are_valid` | Wave 0 |
| STORE-02 | 重复执行 to_latest() 不报错（幂等性） | unit | `cargo test -p cli-manager-lib traffic::schema::tests::migrations_are_idempotent` | Wave 0 |
| STORE-02 | request_logs 和 daily_rollups 两表及索引均已创建 | unit | `cargo test -p cli-manager-lib traffic::schema::tests::migrations_create_expected_tables` | Wave 0 |
| STORE-01 | 路径不含 "Mobile Documents" 或 "CloudDocs" | unit | `cargo test -p cli-manager-lib traffic::db::tests::test_db_path_not_icloud` | Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p cli-manager-lib traffic`
- **Per wave merge:** `cargo test -p cli-manager-lib`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src-tauri/src/traffic/mod.rs` — TrafficDb 结构体，covers STORE-01
- [ ] `src-tauri/src/traffic/schema.rs` — MIGRATIONS 常量，covers STORE-02
- [ ] `src-tauri/src/traffic/db.rs` — 路径解析和 open 函数，covers STORE-01
- [ ] 依赖安装：`rusqlite = { version = "0.38", features = ["bundled"] }` 和 `rusqlite_migration = "2.4"` 加入 Cargo.toml

## Sources

### Primary (HIGH confidence)

- [docs.rs/rusqlite/latest](https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html) — Connection::open、pragma_update、threading 模型
- [docs.rs/rusqlite_migration/latest](https://docs.rs/rusqlite_migration/latest/rusqlite_migration/) — Migrations::from_slice、to_latest、validate API
- [v2.tauri.app/develop/state-management](https://v2.tauri.app/develop/state-management/) — manage() 和 State<T> 模式

### Secondary (MEDIUM confidence)

- [tauri path mapping on macOS](https://michaelcharl.es/aubrey/en/code/tauri-2-mac-paths) — app_local_data_dir = ~/Library/Application Support（已与 dirs crate 行为交叉验证）
- [SQLite concurrent writes and "database is locked" errors](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/) — busy_timeout 最佳实践

### Tertiary (LOW confidence)

- 无

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — rusqlite 0.38.0、rusqlite_migration 2.4.1 版本经 docs.rs 确认；API 用法从官方文档验证
- Architecture: HIGH — Tauri manage() 模式经官方文档验证；路径选择经 dirs crate 文档和 macOS 路径映射文档交叉验证
- Pitfalls: HIGH — WAL/busy_timeout 持久性区别、iCloud 路径危险来自官方 SQLite 文档和项目已有的 CONTEXT.md 设计决策

**Research date:** 2026-03-18
**Valid until:** 2026-06-18（rusqlite 版本号稳定，可能小版本更新但 API 兼容）
