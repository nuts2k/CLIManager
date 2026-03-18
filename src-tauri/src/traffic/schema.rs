use rusqlite_migration::{Migrations, M};

// 迁移切片：当前仅含 v1（初始建表）
// 未来加字段时追加新的 M::up(...)，rusqlite_migration 通过 user_version pragma 跟踪版本
const MIGRATIONS_SLICE: &[M<'static>] = &[M::up(
    "
    CREATE TABLE IF NOT EXISTS request_logs (
        id                      INTEGER PRIMARY KEY AUTOINCREMENT,
        created_at              INTEGER NOT NULL,
        provider_name           TEXT NOT NULL,
        cli_id                  TEXT NOT NULL,
        method                  TEXT NOT NULL,
        path                    TEXT NOT NULL,
        status_code             INTEGER,
        is_streaming            INTEGER NOT NULL DEFAULT 0,
        request_model           TEXT,
        upstream_model          TEXT,
        protocol_type           TEXT NOT NULL,
        input_tokens            INTEGER,
        output_tokens           INTEGER,
        cache_creation_tokens   INTEGER,
        cache_read_tokens       INTEGER,
        ttfb_ms                 INTEGER,
        duration_ms             INTEGER,
        stop_reason             TEXT,
        error_message           TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_request_logs_created_at
        ON request_logs (created_at);
    CREATE INDEX IF NOT EXISTS idx_request_logs_provider_name
        ON request_logs (provider_name);
    CREATE TABLE IF NOT EXISTS daily_rollups (
        id                          INTEGER PRIMARY KEY AUTOINCREMENT,
        provider_name               TEXT NOT NULL,
        rollup_date                 TEXT NOT NULL,
        request_count               INTEGER NOT NULL DEFAULT 0,
        success_count               INTEGER NOT NULL DEFAULT 0,
        total_input_tokens          INTEGER NOT NULL DEFAULT 0,
        total_output_tokens         INTEGER NOT NULL DEFAULT 0,
        total_cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
        total_cache_read_tokens     INTEGER NOT NULL DEFAULT 0,
        cache_triggered_count       INTEGER NOT NULL DEFAULT 0,
        cache_hit_count             INTEGER NOT NULL DEFAULT 0,
        sum_ttfb_ms                 INTEGER NOT NULL DEFAULT 0,
        sum_duration_ms             INTEGER NOT NULL DEFAULT 0,
        UNIQUE(provider_name, rollup_date)
    );
    CREATE INDEX IF NOT EXISTS idx_daily_rollups_date
        ON daily_rollups (rollup_date);
    ",
)];

/// 全局迁移常量，每次 DB 打开后调用 to_latest() 幂等应用
pub const MIGRATIONS: Migrations<'static> = Migrations::from_slice(MIGRATIONS_SLICE);

#[cfg(test)]
mod tests {
    use super::*;

    /// MIGRATIONS 对象通过 validate() 验证（SQL 语法正确）
    #[test]
    fn migrations_are_valid() {
        assert!(
            MIGRATIONS.validate().is_ok(),
            "MIGRATIONS validate 失败：SQL 语法或迁移顺序有误"
        );
    }

    /// to_latest() 后 request_logs 和 daily_rollups 两张表及索引均已创建
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
        assert_eq!(count, 1, "request_logs 表不存在");

        // 验证 request_logs 列数（19 列）
        let col_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('request_logs')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(col_count, 19, "request_logs 应有 19 列，实际 {col_count}");

        // 验证 daily_rollups 表存在
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='daily_rollups'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "daily_rollups 表不存在");

        // 验证 daily_rollups 列数（14 列：id + provider_name + rollup_date + 10 聚合字段 + UNIQUE）
        // UNIQUE 约束不算列，实际为 13 列
        let col_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('daily_rollups')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(col_count, 13, "daily_rollups 应有 13 列，实际 {col_count}");

        // 验证 idx_request_logs_created_at 索引存在
        let idx_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='index' AND name='idx_request_logs_created_at'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx_count, 1, "idx_request_logs_created_at 索引不存在");

        // 验证 idx_request_logs_provider_name 索引存在
        let idx_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='index' AND name='idx_request_logs_provider_name'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx_count, 1, "idx_request_logs_provider_name 索引不存在");

        // 验证 idx_daily_rollups_date 索引存在
        let idx_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='index' AND name='idx_daily_rollups_date'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx_count, 1, "idx_daily_rollups_date 索引不存在");
    }

    /// 重复执行 to_latest() 不报错（幂等性）
    #[test]
    fn migrations_are_idempotent() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        MIGRATIONS.to_latest(&mut conn).expect("第一次迁移失败");
        MIGRATIONS.to_latest(&mut conn).expect("第二次迁移失败（幂等性验证）");
        MIGRATIONS.to_latest(&mut conn).expect("第三次迁移失败（幂等性验证）");
    }
}
