use rusqlite::Result;

/// 按 Provider 聚合的统计数据（供应商排行榜）
#[derive(Debug, serde::Serialize)]
pub struct ProviderStat {
    pub provider_name: String,
    pub request_count: i64,
    pub success_count: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_creation_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub cache_triggered_count: i64,
    pub cache_hit_count: i64,
    pub sum_ttfb_ms: i64,
    pub sum_duration_ms: i64,
}

/// 时间维度趋势数据点
#[derive(Debug, serde::Serialize)]
pub struct TimeStat {
    pub label: String,       // "HH:00" 或 "YYYY-MM-DD"
    pub request_count: i64,
    pub total_tokens: i64,   // input + output 合计
}

impl super::TrafficDb {
    /// 聚合超 24h 的 request_logs 明细到 daily_rollups，删除已聚合明细，删除超 7d 统计
    ///
    /// 单次事务内原子完成三步：
    /// 1. INSERT INTO daily_rollups ... ON CONFLICT DO UPDATE SET（增量 upsert，不丢历史）
    /// 2. DELETE FROM request_logs WHERE created_at < 24h 前
    /// 3. DELETE FROM daily_rollups WHERE rollup_date < 7d 前
    ///
    /// 注意：created_at 是 epoch 毫秒，SQLite strftime('%s','now') 返回秒，需乘以 1000。
    pub fn rollup_and_prune(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let sql = format!(
            "
            BEGIN;

            -- 步骤 1：将超过 24h 的明细按 (provider_name, rollup_date) 聚合写入/增量更新 daily_rollups
            -- 使用 ON CONFLICT DO UPDATE SET（增量 upsert），不丢失历史累积数据（RESEARCH.md Pitfall 1）
            -- created_at 是 epoch 毫秒，strftime('%s','now') 是秒，阈值需乘以 1000（RESEARCH.md Pitfall 2）
            INSERT INTO daily_rollups (
                provider_name, rollup_date,
                request_count, success_count,
                total_input_tokens, total_output_tokens,
                total_cache_creation_tokens, total_cache_read_tokens,
                cache_triggered_count, cache_hit_count,
                sum_ttfb_ms, sum_duration_ms
            )
            SELECT
                provider_name,
                strftime('%Y-%m-%d', created_at / 1000, 'unixepoch', 'localtime') AS rollup_date,
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
              AND {}
            GROUP BY provider_name, strftime('%Y-%m-%d', created_at / 1000, 'unixepoch', 'localtime')
            ON CONFLICT(provider_name, rollup_date) DO UPDATE SET
                request_count               = request_count + excluded.request_count,
                success_count               = success_count + excluded.success_count,
                total_input_tokens          = total_input_tokens + excluded.total_input_tokens,
                total_output_tokens         = total_output_tokens + excluded.total_output_tokens,
                total_cache_creation_tokens = total_cache_creation_tokens + excluded.total_cache_creation_tokens,
                total_cache_read_tokens     = total_cache_read_tokens + excluded.total_cache_read_tokens,
                cache_triggered_count       = cache_triggered_count + excluded.cache_triggered_count,
                cache_hit_count             = cache_hit_count + excluded.cache_hit_count,
                sum_ttfb_ms                 = sum_ttfb_ms + excluded.sum_ttfb_ms,
                sum_duration_ms             = sum_duration_ms + excluded.sum_duration_ms;

            -- 步骤 2：删除已聚合的超 24h 明细
            DELETE FROM request_logs
            WHERE created_at < (strftime('%s', 'now') - 86400) * 1000;

            -- 步骤 3：删除超过 7d 的 daily_rollups 记录
            DELETE FROM daily_rollups
            WHERE rollup_date < strftime('%Y-%m-%d', 'now', 'localtime', '-7 days');

            COMMIT;
            ",
            super::HIDDEN_REQUESTS_SQL_FILTER
        );
        conn.execute_batch(&sql)?;
        Ok(())
    }

    /// 查询按 Provider 聚合的统计数据
    ///
    /// range:
    /// - "24h"：从 request_logs 查询最近 24h 数据，按 provider_name 分组
    /// - "7d"：合并 daily_rollups 历史聚合 + 最近 24h request_logs 明细，按 provider 汇总
    pub fn query_provider_stats(&self, range: &str) -> Result<Vec<ProviderStat>> {
        let conn = self.conn.lock().unwrap();

        match range {
            "24h" => {
                let threshold_ms = (chrono::Utc::now().timestamp() - 86400) * 1000;
                let query = format!(
                    "SELECT
                        provider_name,
                        COUNT(*) AS request_count,
                        SUM(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END) AS success_count,
                        COALESCE(SUM(input_tokens), 0) AS total_input_tokens,
                        COALESCE(SUM(output_tokens), 0) AS total_output_tokens,
                        COALESCE(SUM(cache_creation_tokens), 0) AS total_cache_creation_tokens,
                        COALESCE(SUM(cache_read_tokens), 0) AS total_cache_read_tokens,
                        SUM(CASE WHEN cache_creation_tokens > 0 OR cache_read_tokens > 0 THEN 1 ELSE 0 END) AS cache_triggered_count,
                        SUM(CASE WHEN cache_read_tokens > 0 THEN 1 ELSE 0 END) AS cache_hit_count,
                        COALESCE(SUM(ttfb_ms), 0) AS sum_ttfb_ms,
                        COALESCE(SUM(duration_ms), 0) AS sum_duration_ms
                    FROM request_logs
                    WHERE created_at >= ?1
                      AND {}
                    GROUP BY provider_name
                    ORDER BY request_count DESC",
                    super::HIDDEN_REQUESTS_SQL_FILTER
                );
                let mut stmt = conn.prepare(&query)?;
                // 列索引：0=provider_name, 1=request_count, 2=success_count,
                // 3=total_input_tokens, 4=total_output_tokens, 5=total_cache_creation_tokens,
                // 6=total_cache_read_tokens, 7=cache_triggered_count, 8=cache_hit_count,
                // 9=sum_ttfb_ms, 10=sum_duration_ms
                let rows = stmt.query_map(rusqlite::params![threshold_ms], |row| {
                    Ok(ProviderStat {
                        provider_name: row.get(0)?,
                        request_count: row.get(1)?,
                        success_count: row.get(2)?,
                        total_input_tokens: row.get(3)?,
                        total_output_tokens: row.get(4)?,
                        total_cache_creation_tokens: row.get(5)?,
                        total_cache_read_tokens: row.get(6)?,
                        cache_triggered_count: row.get(7)?,
                        cache_hit_count: row.get(8)?,
                        sum_ttfb_ms: row.get(9)?,
                        sum_duration_ms: row.get(10)?,
                    })
                })?;
                rows.collect()
            }
            "7d" => {
                let recent_threshold_ms = (chrono::Utc::now().timestamp() - 86400) * 1000;
                let threshold_date = (chrono::Local::now() - chrono::Duration::days(7))
                    .format("%Y-%m-%d")
                    .to_string();
                let query = format!(
                    "SELECT
                        provider_name,
                        SUM(request_count) AS request_count,
                        SUM(success_count) AS success_count,
                        SUM(total_input_tokens) AS total_input_tokens,
                        SUM(total_output_tokens) AS total_output_tokens,
                        SUM(total_cache_creation_tokens) AS total_cache_creation_tokens,
                        SUM(total_cache_read_tokens) AS total_cache_read_tokens,
                        SUM(cache_triggered_count) AS cache_triggered_count,
                        SUM(cache_hit_count) AS cache_hit_count,
                        SUM(sum_ttfb_ms) AS sum_ttfb_ms,
                        SUM(sum_duration_ms) AS sum_duration_ms
                    FROM (
                        SELECT
                            provider_name,
                            request_count,
                            success_count,
                            total_input_tokens,
                            total_output_tokens,
                            total_cache_creation_tokens,
                            total_cache_read_tokens,
                            cache_triggered_count,
                            cache_hit_count,
                            sum_ttfb_ms,
                            sum_duration_ms
                        FROM daily_rollups
                        WHERE rollup_date >= ?1

                        UNION ALL

                        SELECT
                            provider_name,
                            COUNT(*) AS request_count,
                            SUM(CASE WHEN status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END) AS success_count,
                            COALESCE(SUM(input_tokens), 0) AS total_input_tokens,
                            COALESCE(SUM(output_tokens), 0) AS total_output_tokens,
                            COALESCE(SUM(cache_creation_tokens), 0) AS total_cache_creation_tokens,
                            COALESCE(SUM(cache_read_tokens), 0) AS total_cache_read_tokens,
                            SUM(CASE WHEN cache_creation_tokens > 0 OR cache_read_tokens > 0 THEN 1 ELSE 0 END) AS cache_triggered_count,
                            SUM(CASE WHEN cache_read_tokens > 0 THEN 1 ELSE 0 END) AS cache_hit_count,
                            COALESCE(SUM(ttfb_ms), 0) AS sum_ttfb_ms,
                            COALESCE(SUM(duration_ms), 0) AS sum_duration_ms
                        FROM request_logs
                        WHERE created_at >= ?2
                          AND {}
                        GROUP BY provider_name
                    )
                    GROUP BY provider_name
                    ORDER BY request_count DESC",
                    super::HIDDEN_REQUESTS_SQL_FILTER
                );
                let mut stmt = conn.prepare(&query)?;
                // 列索引：0=provider_name, 1=request_count, 2=success_count,
                // 3=total_input_tokens, 4=total_output_tokens, 5=total_cache_creation_tokens,
                // 6=total_cache_read_tokens, 7=cache_triggered_count, 8=cache_hit_count,
                // 9=sum_ttfb_ms, 10=sum_duration_ms
                let rows = stmt.query_map(rusqlite::params![threshold_date, recent_threshold_ms], |row| {
                    Ok(ProviderStat {
                        provider_name: row.get(0)?,
                        request_count: row.get(1)?,
                        success_count: row.get(2)?,
                        total_input_tokens: row.get(3)?,
                        total_output_tokens: row.get(4)?,
                        total_cache_creation_tokens: row.get(5)?,
                        total_cache_read_tokens: row.get(6)?,
                        cache_triggered_count: row.get(7)?,
                        cache_hit_count: row.get(8)?,
                        sum_ttfb_ms: row.get(9)?,
                        sum_duration_ms: row.get(10)?,
                    })
                })?;
                rows.collect()
            }
            _ => Ok(vec![]),
        }
    }

    /// 查询按时间聚合的趋势数据
    ///
    /// range:
    /// - "24h"：从 request_logs 按小时分组（HH:00）
    /// - "7d"：合并 daily_rollups 历史聚合 + 最近 24h request_logs 明细，按天分组（YYYY-MM-DD）
    pub fn query_time_trend(&self, range: &str) -> Result<Vec<TimeStat>> {
        let conn = self.conn.lock().unwrap();

        match range {
            "24h" => {
                let threshold_ms = (chrono::Utc::now().timestamp() - 86400) * 1000;
                let query = format!(
                    "SELECT
                        strftime('%H:00', created_at / 1000, 'unixepoch', 'localtime') AS hour_label,
                        COUNT(*) AS request_count,
                        COALESCE(SUM(input_tokens), 0) + COALESCE(SUM(output_tokens), 0) AS total_tokens
                    FROM request_logs
                    WHERE created_at >= ?1
                      AND {}
                    GROUP BY strftime('%Y-%m-%d %H', created_at / 1000, 'unixepoch', 'localtime')
                    ORDER BY hour_label ASC",
                    super::HIDDEN_REQUESTS_SQL_FILTER
                );
                let mut stmt = conn.prepare(&query)?;
                let rows = stmt.query_map(rusqlite::params![threshold_ms], |row| {
                    Ok(TimeStat {
                        label: row.get(0)?,
                        request_count: row.get(1)?,
                        total_tokens: row.get(2)?,
                    })
                })?;
                rows.collect()
            }
            "7d" => {
                let recent_threshold_ms = (chrono::Utc::now().timestamp() - 86400) * 1000;
                let threshold_date = (chrono::Local::now() - chrono::Duration::days(7))
                    .format("%Y-%m-%d")
                    .to_string();
                let query = format!(
                    "SELECT
                        day_label,
                        SUM(request_count) AS request_count,
                        SUM(total_tokens) AS total_tokens
                    FROM (
                        SELECT
                            rollup_date AS day_label,
                            SUM(request_count) AS request_count,
                            SUM(total_input_tokens) + SUM(total_output_tokens) AS total_tokens
                        FROM daily_rollups
                        WHERE rollup_date >= ?1
                        GROUP BY rollup_date

                        UNION ALL

                        SELECT
                            strftime('%Y-%m-%d', created_at / 1000, 'unixepoch', 'localtime') AS day_label,
                            COUNT(*) AS request_count,
                            COALESCE(SUM(input_tokens), 0) + COALESCE(SUM(output_tokens), 0) AS total_tokens
                        FROM request_logs
                        WHERE created_at >= ?2
                          AND {}
                        GROUP BY strftime('%Y-%m-%d', created_at / 1000, 'unixepoch', 'localtime')
                    )
                    GROUP BY day_label
                    ORDER BY day_label ASC",
                    super::HIDDEN_REQUESTS_SQL_FILTER
                );
                let mut stmt = conn.prepare(&query)?;
                let rows = stmt.query_map(rusqlite::params![threshold_date, recent_threshold_ms], |row| {
                    Ok(TimeStat {
                        label: row.get(0)?,
                        request_count: row.get(1)?,
                        total_tokens: row.get(2)?,
                    })
                })?;
                rows.collect()
            }
            _ => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::traffic::TrafficDb;
    use std::sync::Mutex;

    /// 创建内存 DB（运行所有迁移）
    fn make_test_db() -> TrafficDb {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::traffic::schema::MIGRATIONS
            .to_latest(&mut conn)
            .unwrap();
        TrafficDb {
            conn: Mutex::new(conn),
        }
    }

    /// 插入一条 request_log，created_at 可指定
    fn insert_log(db: &TrafficDb, provider: &str, created_at: i64, status: i64, input: i64, output: i64) {
        insert_log_with_cache(db, provider, created_at, status, input, output, None, None);
    }

    /// 插入一条带缓存 token 的 request_log
    fn insert_log_with_cache(
        db: &TrafficDb,
        provider: &str,
        created_at: i64,
        status: i64,
        input: i64,
        output: i64,
        cache_creation: Option<i64>,
        cache_read: Option<i64>,
    ) {
        insert_log_with_path(
            db,
            provider,
            created_at,
            status,
            input,
            output,
            cache_creation,
            cache_read,
            "/v1/messages",
        );
    }

    fn insert_log_with_path(
        db: &TrafficDb,
        provider: &str,
        created_at: i64,
        status: i64,
        input: i64,
        output: i64,
        cache_creation: Option<i64>,
        cache_read: Option<i64>,
        path: &str,
    ) {
        use crate::traffic::log::LogEntry;
        let entry = LogEntry {
            created_at,
            provider_name: provider.to_string(),
            cli_id: "claude".to_string(),
            method: "POST".to_string(),
            path: path.to_string(),
            status_code: Some(status),
            is_streaming: 0,
            request_model: Some("claude-3-5-sonnet".to_string()),
            upstream_model: None,
            protocol_type: "anthropic".to_string(),
            input_tokens: Some(input),
            output_tokens: Some(output),
            cache_creation_tokens: cache_creation,
            cache_read_tokens: cache_read,
            ttfb_ms: Some(100),
            duration_ms: Some(500),
            stop_reason: Some("end_turn".to_string()),
            error_message: None,
        };
        db.insert_request_log(&entry).unwrap();
    }

    /// 超 24h 前的时间戳（epoch 毫秒）
    fn old_ts() -> i64 {
        (chrono::Utc::now().timestamp() - 86400 - 3600) * 1000
    }

    /// 最近 1h 内的时间戳（epoch 毫秒）
    fn recent_ts() -> i64 {
        (chrono::Utc::now().timestamp() - 1800) * 1000
    }

    /// 最近 N 小时前的时间戳（epoch 毫秒）
    fn hours_ago_ts(hours: i64) -> i64 {
        (chrono::Utc::now().timestamp() - hours * 3600) * 1000
    }

    // ====================================================================
    // rollup_and_prune 测试
    // ====================================================================

    /// Test: rollup_and_prune 将超过 24h 的 request_logs 聚合到 daily_rollups（10 个聚合字段正确）
    #[test]
    fn test_rollup_aggregates_old_logs() {
        let db = make_test_db();
        let ts = old_ts();
        // 插入 2 条超 24h 记录（同一 provider）
        insert_log(&db, "provider-a", ts, 200, 100, 50);
        insert_log(&db, "provider-a", ts, 200, 200, 100);

        db.rollup_and_prune().unwrap();

        // daily_rollups 应有 1 行，request_count = 2，token 正确聚合
        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM daily_rollups", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "daily_rollups 应有 1 行");

        let (req_count, total_input, total_output): (i64, i64, i64) = conn
            .query_row(
                "SELECT request_count, total_input_tokens, total_output_tokens FROM daily_rollups",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(req_count, 2, "request_count 应为 2");
        assert_eq!(total_input, 300, "total_input_tokens 应为 300 (100+200)");
        assert_eq!(total_output, 150, "total_output_tokens 应为 150 (50+100)");
    }

    /// Test: rollup_and_prune 删除超 24h 的 request_logs 行
    #[test]
    fn test_prune_deletes_old_logs() {
        let db = make_test_db();
        let old = old_ts();
        let new = recent_ts();

        insert_log(&db, "provider-a", old, 200, 100, 50);  // 超 24h，应被删
        insert_log(&db, "provider-a", new, 200, 100, 50);  // 近 1h，应保留

        db.rollup_and_prune().unwrap();

        let conn = db.conn.lock().unwrap();
        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM request_logs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(remaining, 1, "应保留 1 条最近的记录，超 24h 的应被删除");
    }

    /// Test: rollup_and_prune 删除 rollup_date < 7d 前的 daily_rollups 行
    #[test]
    fn test_prune_deletes_old_rollups() {
        let db = make_test_db();

        // 直接插入一条超 7d 的 daily_rollups 行
        let old_date = (chrono::Utc::now() - chrono::Duration::days(8))
            .format("%Y-%m-%d")
            .to_string();
        let recent_date = (chrono::Utc::now() - chrono::Duration::days(3))
            .format("%Y-%m-%d")
            .to_string();

        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO daily_rollups (provider_name, rollup_date, request_count, success_count,
                    total_input_tokens, total_output_tokens, total_cache_creation_tokens,
                    total_cache_read_tokens, cache_triggered_count, cache_hit_count,
                    sum_ttfb_ms, sum_duration_ms)
                 VALUES (?1, ?2, 10, 8, 1000, 500, 0, 0, 0, 0, 1000, 5000)",
                rusqlite::params!["provider-x", old_date],
            ).unwrap();
            conn.execute(
                "INSERT INTO daily_rollups (provider_name, rollup_date, request_count, success_count,
                    total_input_tokens, total_output_tokens, total_cache_creation_tokens,
                    total_cache_read_tokens, cache_triggered_count, cache_hit_count,
                    sum_ttfb_ms, sum_duration_ms)
                 VALUES (?1, ?2, 5, 4, 500, 200, 0, 0, 0, 0, 500, 2000)",
                rusqlite::params!["provider-x", recent_date],
            ).unwrap();
        }

        db.rollup_and_prune().unwrap();

        let conn = db.conn.lock().unwrap();
        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM daily_rollups", [], |r| r.get(0))
            .unwrap();
        // 超 7d 的应被删，最近 3d 的应保留
        assert_eq!(remaining, 1, "应保留 1 条近 7d 的 rollup 行，超 7d 的应被删除");

        let kept_date: String = conn
            .query_row("SELECT rollup_date FROM daily_rollups", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kept_date, recent_date, "保留的应是 recent_date 行");
    }

    /// Test: rollup_and_prune 幂等性（连续调用两次，daily_rollups 数据一致，不重复累加）
    #[test]
    fn test_rollup_idempotent() {
        let db = make_test_db();
        let ts = old_ts();
        // 插入 2 条超 24h 记录
        insert_log(&db, "provider-a", ts, 200, 100, 50);
        insert_log(&db, "provider-a", ts, 200, 200, 100);

        // 第一次 rollup（聚合数据到 daily_rollups，并删除 request_logs）
        db.rollup_and_prune().unwrap();

        let (req_count_1, total_input_1, total_output_1) = {
            let conn = db.conn.lock().unwrap();
            conn.query_row(
                "SELECT request_count, total_input_tokens, total_output_tokens FROM daily_rollups",
                [],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)),
            ).unwrap()
        };

        // 第二次 rollup（request_logs 已为空，daily_rollups 应不变）
        db.rollup_and_prune().unwrap();

        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM daily_rollups", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "第二次 rollup 后 daily_rollups 仍应有 1 行");

        let (req_count_2, total_input_2, total_output_2): (i64, i64, i64) = conn
            .query_row(
                "SELECT request_count, total_input_tokens, total_output_tokens FROM daily_rollups",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ).unwrap();

        assert_eq!(req_count_1, req_count_2, "两次 rollup request_count 应一致（幂等）");
        assert_eq!(total_input_1, total_input_2, "两次 rollup total_input_tokens 应一致（幂等）");
        assert_eq!(total_output_1, total_output_2, "两次 rollup total_output_tokens 应一致（幂等）");
    }

    /// Test: rollup_and_prune 不应将 token count 请求聚合到 daily_rollups
    #[test]
    fn test_rollup_excludes_token_count_requests() {
        let db = make_test_db();
        let ts = old_ts();

        insert_log(&db, "provider-a", ts, 200, 100, 50);
        insert_log_with_path(
            &db,
            "provider-a",
            ts,
            200,
            999,
            888,
            None,
            None,
            "/v1/token_count",
        );

        db.rollup_and_prune().unwrap();

        let conn = db.conn.lock().unwrap();
        let (req_count, total_input, total_output): (i64, i64, i64) = conn
            .query_row(
                "SELECT request_count, total_input_tokens, total_output_tokens FROM daily_rollups",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(req_count, 1, "rollup 不应聚合 token count 请求");
        assert_eq!(total_input, 100);
        assert_eq!(total_output, 50);
    }

    // ====================================================================
    // query_provider_stats 测试
    // ====================================================================

    /// Test: query_provider_stats("24h") 从 request_logs 按 provider 分组返回正确聚合值
    #[test]
    fn test_query_provider_stats_24h() {
        let db = make_test_db();
        let recent = recent_ts();

        // provider-a: 2 条成功（其中 1 条带 cache_creation_tokens=50），provider-b: 1 条失败
        insert_log(&db, "provider-a", recent, 200, 100, 50);
        insert_log_with_cache(&db, "provider-a", recent, 200, 200, 80, Some(50), None);
        insert_log(&db, "provider-b", recent, 500, 50, 0);

        let stats = db.query_provider_stats("24h").unwrap();
        assert_eq!(stats.len(), 2, "应有 2 个 provider 的统计");

        // provider-a 请求数更多，应排第一
        assert_eq!(stats[0].provider_name, "provider-a");
        assert_eq!(stats[0].request_count, 2);
        assert_eq!(stats[0].success_count, 2);
        assert_eq!(stats[0].total_input_tokens, 300, "input: 100+200");
        assert_eq!(stats[0].total_output_tokens, 130, "output: 50+80");
        assert_eq!(stats[0].total_cache_creation_tokens, 50, "cache_creation: 0+50");

        assert_eq!(stats[1].provider_name, "provider-b");
        assert_eq!(stats[1].request_count, 1);
        assert_eq!(stats[1].success_count, 0, "status 500 不算成功");
        assert_eq!(stats[1].total_cache_creation_tokens, 0, "provider-b 无缓存创建 token");
    }

    /// Test: query_provider_stats("7d") 合并 daily_rollups 与最近 24h request_logs 后返回正确聚合值
    #[test]
    fn test_query_provider_stats_7d() {
        let db = make_test_db();
        let recent = recent_ts();

        // 直接插入 daily_rollups 行（模拟已 rollup 的历史数据）
        let d1 = (chrono::Utc::now() - chrono::Duration::days(2))
            .format("%Y-%m-%d")
            .to_string();
        let d2 = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        {
            let conn = db.conn.lock().unwrap();
            // provider-a 两天数据（d1 带 cache_creation=100，d2 带 cache_creation=30）
            conn.execute(
                "INSERT INTO daily_rollups (provider_name, rollup_date, request_count, success_count,
                    total_input_tokens, total_output_tokens, total_cache_creation_tokens,
                    total_cache_read_tokens, cache_triggered_count, cache_hit_count,
                    sum_ttfb_ms, sum_duration_ms)
                 VALUES ('provider-a', ?1, 10, 9, 1000, 500, 100, 0, 0, 0, 1000, 5000)",
                rusqlite::params![d1],
            ).unwrap();
            conn.execute(
                "INSERT INTO daily_rollups (provider_name, rollup_date, request_count, success_count,
                    total_input_tokens, total_output_tokens, total_cache_creation_tokens,
                    total_cache_read_tokens, cache_triggered_count, cache_hit_count,
                    sum_ttfb_ms, sum_duration_ms)
                 VALUES ('provider-a', ?1, 5, 5, 500, 200, 30, 0, 0, 0, 500, 2000)",
                rusqlite::params![d2],
            ).unwrap();
            // provider-b 一天数据（无缓存）
            conn.execute(
                "INSERT INTO daily_rollups (provider_name, rollup_date, request_count, success_count,
                    total_input_tokens, total_output_tokens, total_cache_creation_tokens,
                    total_cache_read_tokens, cache_triggered_count, cache_hit_count,
                    sum_ttfb_ms, sum_duration_ms)
                 VALUES ('provider-b', ?1, 3, 2, 300, 100, 0, 0, 0, 0, 300, 1500)",
                rusqlite::params![d1],
            ).unwrap();
        }

        // 最近 24h 明细：provider-a 带 cache_creation=20，provider-c 无缓存
        insert_log_with_cache(&db, "provider-a", recent, 200, 120, 30, Some(20), None);
        insert_log(&db, "provider-c", recent, 200, 60, 40);

        let stats = db.query_provider_stats("7d").unwrap();
        assert_eq!(stats.len(), 3, "应有 3 个 provider（含最近 24h 明细）");

        // provider-a 合并两天 rollup + 最近 24h 明细：request_count = 16，排第一
        assert_eq!(stats[0].provider_name, "provider-a");
        assert_eq!(stats[0].request_count, 16, "provider-a 应包含最近 24h 的 1 条明细");
        assert_eq!(stats[0].success_count, 15, "provider-a success_count 应补上最近 24h 成功请求");
        assert_eq!(stats[0].total_input_tokens, 1620, "provider-a input 应补上最近 24h 的 120");
        assert_eq!(stats[0].total_output_tokens, 730, "provider-a output 应补上最近 24h 的 30");
        assert_eq!(
            stats[0].total_cache_creation_tokens, 150,
            "provider-a cache_creation 应为 100(d1)+30(d2)+20(24h 明细)"
        );

        assert_eq!(stats[2].provider_name, "provider-c");
        assert_eq!(stats[2].request_count, 1, "provider-c 仅来自最近 24h 明细");
        assert_eq!(stats[2].total_cache_creation_tokens, 0, "provider-c 无缓存创建 token");
    }

    /// Test: 统计查询应排除 token count 请求
    #[test]
    fn test_stats_queries_exclude_token_count_requests() {
        let db = make_test_db();
        let recent = recent_ts();

        insert_log(&db, "provider-a", recent, 200, 100, 50);
        insert_log_with_path(
            &db,
            "provider-a",
            recent,
            200,
            999,
            888,
            None,
            None,
            "/v1/token_count",
        );
        insert_log_with_path(
            &db,
            "provider-a",
            recent,
            200,
            777,
            666,
            None,
            None,
            "/v1/messages/count_tokens",
        );

        let provider_stats = db.query_provider_stats("24h").unwrap();
        assert_eq!(provider_stats.len(), 1);
        assert_eq!(provider_stats[0].request_count, 1, "provider 统计不应计入 token count 请求");
        assert_eq!(provider_stats[0].total_input_tokens, 100);
        assert_eq!(provider_stats[0].total_output_tokens, 50);

        let trend = db.query_time_trend("24h").unwrap();
        let total_req: i64 = trend.iter().map(|t| t.request_count).sum();
        let total_tokens: i64 = trend.iter().map(|t| t.total_tokens).sum();
        assert_eq!(total_req, 1, "时间趋势不应计入 token count 请求");
        assert_eq!(total_tokens, 150, "时间趋势 token 总量不应计入 token count 请求");
    }

    // ====================================================================
    // query_time_trend 测试
    // ====================================================================

    /// Test: query_time_trend("24h") 返回按小时分组的请求数和 token 总量
    #[test]
    fn test_query_time_trend_24h() {
        let db = make_test_db();
        let recent = recent_ts();

        // 插入 3 条近 24h 数据
        insert_log(&db, "provider-a", recent, 200, 100, 50);
        insert_log(&db, "provider-a", recent, 200, 200, 80);
        insert_log(&db, "provider-b", recent, 200, 50, 30);

        let trend = db.query_time_trend("24h").unwrap();
        // 3 条数据在同一小时内，应汇总为 1 个时间点
        assert!(!trend.is_empty(), "24h 趋势数据不应为空");

        let total_req: i64 = trend.iter().map(|t| t.request_count).sum();
        let total_tok: i64 = trend.iter().map(|t| t.total_tokens).sum();
        assert_eq!(total_req, 3, "总请求数应为 3");
        assert_eq!(total_tok, 510, "total_tokens = (100+50)+(200+80)+(50+30) = 510");

        // label 格式应为 "HH:00"
        for point in &trend {
            assert!(
                point.label.ends_with(":00") && point.label.len() == 5,
                "label 应为 HH:00 格式，实际: {}",
                point.label
            );
        }
    }

    /// Test: query_time_trend("7d") 合并 daily_rollups 与最近 24h request_logs 后返回按天分组结果
    ///
    /// 修复说明（[Rule 1 - Bug] 修复原始测试的时区边界竞态）：
    /// 原始测试使用 hours_ago_ts(23) 模拟 "昨天" 数据，在 UTC 凌晨时该时间点属于今天，
    /// 导致天点数与断言不符。修复方案：只断言 daily_rollups 的静态部分（d1），
    /// 以及两条数据源合并后的总量，彻底消除时区边界问题。
    #[test]
    fn test_query_time_trend_7d() {
        let db = make_test_db();
        let recent = recent_ts(); // 30min 前，始终在最近 24h 内

        let d1 = (chrono::Utc::now() - chrono::Duration::days(2))
            .format("%Y-%m-%d")
            .to_string();
        let d2 = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO daily_rollups (provider_name, rollup_date, request_count, success_count,
                    total_input_tokens, total_output_tokens, total_cache_creation_tokens,
                    total_cache_read_tokens, cache_triggered_count, cache_hit_count,
                    sum_ttfb_ms, sum_duration_ms)
                 VALUES ('provider-a', ?1, 10, 8, 1000, 500, 0, 0, 0, 0, 1000, 5000)",
                rusqlite::params![d1],
            ).unwrap();
            conn.execute(
                "INSERT INTO daily_rollups (provider_name, rollup_date, request_count, success_count,
                    total_input_tokens, total_output_tokens, total_cache_creation_tokens,
                    total_cache_read_tokens, cache_triggered_count, cache_hit_count,
                    sum_ttfb_ms, sum_duration_ms)
                 VALUES ('provider-b', ?1, 5, 4, 500, 200, 0, 0, 0, 0, 500, 2000)",
                rusqlite::params![d1],
            ).unwrap();
            conn.execute(
                "INSERT INTO daily_rollups (provider_name, rollup_date, request_count, success_count,
                    total_input_tokens, total_output_tokens, total_cache_creation_tokens,
                    total_cache_read_tokens, cache_triggered_count, cache_hit_count,
                    sum_ttfb_ms, sum_duration_ms)
                 VALUES ('provider-a', ?1, 7, 6, 700, 300, 0, 0, 0, 0, 700, 3500)",
                rusqlite::params![d2],
            ).unwrap();
        }

        // 最近 24h 明细：1 条，30min 前（始终在范围内）
        insert_log(&db, "provider-a", recent, 200, 50, 10);

        let trend = db.query_time_trend("7d").unwrap();

        // 至少应有 d1、d2 两个历史天点（来自 daily_rollups），加最近 24h 至少 1 个天点
        assert!(trend.len() >= 2, "至少应有 2 个天点（含 daily_rollups）");

        // d1 应排在最前（ASC 排序），且聚合值正确
        assert_eq!(trend[0].label, d1, "第一个时间点应为 d1");
        assert_eq!(trend[0].request_count, 15, "d1 合并两 provider: 10+5=15");
        assert_eq!(trend[0].total_tokens, 2200, "d1 total_tokens = 1000+500+500+200 = 2200");

        // d2 应在结果中，包含 rollup 的 7 条请求（最近 24h 明细可能合并到 d2 或今天，取决于执行时刻）
        let d2_point = trend.iter().find(|t| t.label == d2);
        assert!(d2_point.is_some(), "应有昨天(d2)天点");
        assert!(
            d2_point.unwrap().request_count >= 7,
            "d2 至少有 rollup 的 7 条请求，实际: {}",
            d2_point.unwrap().request_count
        );

        // 总请求数 = daily_rollups (10+5+7=22) + 最近 24h 明细 (1) = 23
        let total_req: i64 = trend.iter().map(|t| t.request_count).sum();
        assert_eq!(total_req, 23, "总请求数应为 22(daily_rollups)+1(最近明细)=23");

        // 所有日期标签格式应为 YYYY-MM-DD（10 字符）
        for point in &trend {
            assert_eq!(point.label.len(), 10, "label 应为 YYYY-MM-DD 格式: {}", point.label);
        }
    }
}
