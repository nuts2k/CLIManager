use tokio::sync::mpsc;

/// 日志条目：对应 request_logs 表的 18 个数据列（不含 id）
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub created_at: i64,      // epoch ms
    pub provider_name: String,
    pub cli_id: String,
    pub method: String,
    pub path: String,
    pub status_code: Option<i64>,
    pub is_streaming: i64,    // 0 或 1
    pub request_model: Option<String>,
    pub upstream_model: Option<String>,
    pub protocol_type: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub ttfb_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub stop_reason: Option<String>,
    pub error_message: Option<String>,
}

/// 流量日志事件 Payload：用于 Tauri emit，含 id + type + 19 列
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrafficLogPayload {
    #[serde(rename = "type")]
    pub event_type: String, // "new" / "update" / "history"
    pub id: i64,
    pub created_at: i64,
    pub provider_name: String,
    pub cli_id: String,
    pub method: String,
    pub path: String,
    pub status_code: Option<i64>,
    pub is_streaming: i64,
    pub request_model: Option<String>,
    pub upstream_model: Option<String>,
    pub protocol_type: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub ttfb_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub stop_reason: Option<String>,
    pub error_message: Option<String>,
}

impl TrafficLogPayload {
    /// 从 LogEntry + id + event_type 构建 Payload
    pub fn from_entry(id: i64, entry: &LogEntry, event_type: &str) -> Self {
        Self {
            event_type: event_type.to_string(),
            id,
            created_at: entry.created_at,
            provider_name: entry.provider_name.clone(),
            cli_id: entry.cli_id.clone(),
            method: entry.method.clone(),
            path: entry.path.clone(),
            status_code: entry.status_code,
            is_streaming: entry.is_streaming,
            request_model: entry.request_model.clone(),
            upstream_model: entry.upstream_model.clone(),
            protocol_type: entry.protocol_type.clone(),
            input_tokens: entry.input_tokens,
            output_tokens: entry.output_tokens,
            cache_creation_tokens: entry.cache_creation_tokens,
            cache_read_tokens: entry.cache_read_tokens,
            ttfb_ms: entry.ttfb_ms,
            duration_ms: entry.duration_ms,
            stop_reason: entry.stop_reason.clone(),
            error_message: entry.error_message.clone(),
        }
    }
}

impl super::TrafficDb {
    /// 将 LogEntry 写入 request_logs 表，返回自增 id
    pub fn insert_request_log(&self, entry: &LogEntry) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO request_logs (
                created_at, provider_name, cli_id, method, path,
                status_code, is_streaming, request_model, upstream_model, protocol_type,
                input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens,
                ttfb_ms, duration_ms, stop_reason, error_message
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14,
                ?15, ?16, ?17, ?18
            )",
            rusqlite::params![
                entry.created_at,
                entry.provider_name,
                entry.cli_id,
                entry.method,
                entry.path,
                entry.status_code,
                entry.is_streaming,
                entry.request_model,
                entry.upstream_model,
                entry.protocol_type,
                entry.input_tokens,
                entry.output_tokens,
                entry.cache_creation_tokens,
                entry.cache_read_tokens,
                entry.ttfb_ms,
                entry.duration_ms,
                entry.stop_reason,
                entry.error_message,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 查询最近 N 条日志，按 created_at 降序（最新在前）
    pub fn query_recent_logs(&self, limit: i64) -> rusqlite::Result<Vec<TrafficLogPayload>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, created_at, provider_name, cli_id, method, path,
                    status_code, is_streaming, request_model, upstream_model, protocol_type,
                    input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens,
                    ttfb_ms, duration_ms, stop_reason, error_message
             FROM request_logs
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(rusqlite::params![limit], |row| {
            Ok(TrafficLogPayload {
                event_type: "history".to_string(),
                id: row.get(0)?,
                created_at: row.get(1)?,
                provider_name: row.get(2)?,
                cli_id: row.get(3)?,
                method: row.get(4)?,
                path: row.get(5)?,
                status_code: row.get(6)?,
                is_streaming: row.get(7)?,
                request_model: row.get(8)?,
                upstream_model: row.get(9)?,
                protocol_type: row.get(10)?,
                input_tokens: row.get(11)?,
                output_tokens: row.get(12)?,
                cache_creation_tokens: row.get(13)?,
                cache_read_tokens: row.get(14)?,
                ttfb_ms: row.get(15)?,
                duration_ms: row.get(16)?,
                stop_reason: row.get(17)?,
                error_message: row.get(18)?,
            })
        })?;

        rows.collect()
    }
}

/// 后台日志写入 worker：消费 mpsc channel 中的 LogEntry，写入 SQLite 并 emit Tauri 事件
pub async fn log_worker(mut rx: mpsc::Receiver<LogEntry>, app_handle: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    while let Some(entry) = rx.recv().await {
        if let Some(db) = app_handle.try_state::<super::TrafficDb>() {
            match db.insert_request_log(&entry) {
                Ok(id) => {
                    let payload = TrafficLogPayload::from_entry(id, &entry, "new");
                    if let Err(e) = app_handle.emit("traffic-log", &payload) {
                        log::warn!("emit traffic-log 失败: {}", e);
                    }
                }
                Err(e) => log::warn!("写入 request_logs 失败: {}", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traffic::TrafficDb;
    use std::sync::Mutex;

    /// 创建内存 DB 用于测试
    fn make_test_db() -> TrafficDb {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::traffic::schema::MIGRATIONS
            .to_latest(&mut conn)
            .unwrap();
        TrafficDb {
            conn: Mutex::new(conn),
        }
    }

    /// 创建一个完整的测试 LogEntry（所有字段均有值）
    fn make_full_entry() -> LogEntry {
        LogEntry {
            created_at: 1_700_000_000_000,
            provider_name: "test-provider".to_string(),
            cli_id: "claude".to_string(),
            method: "POST".to_string(),
            path: "/v1/messages".to_string(),
            status_code: Some(200),
            is_streaming: 0,
            request_model: Some("claude-3-5-sonnet-20241022".to_string()),
            upstream_model: Some("gpt-4o".to_string()),
            protocol_type: "anthropic".to_string(),
            input_tokens: Some(100),
            output_tokens: Some(50),
            cache_creation_tokens: Some(10),
            cache_read_tokens: Some(5),
            ttfb_ms: Some(120),
            duration_ms: Some(500),
            stop_reason: Some("end_turn".to_string()),
            error_message: None,
        }
    }

    /// Test: insert_request_log 写入 LogEntry 后 last_insert_rowid > 0
    #[test]
    fn test_insert_returns_positive_id() {
        let db = make_test_db();
        let entry = make_full_entry();
        let id = db.insert_request_log(&entry).unwrap();
        assert!(id > 0, "插入后 id 应大于 0，实际: {}", id);
    }

    /// Test: insert_request_log 写入后 query_recent_logs 能查到该记录，字段值匹配
    #[test]
    fn test_insert_then_query_fields_match() {
        let db = make_test_db();
        let entry = make_full_entry();
        let id = db.insert_request_log(&entry).unwrap();

        let logs = db.query_recent_logs(10).unwrap();
        assert_eq!(logs.len(), 1);

        let log = &logs[0];
        assert_eq!(log.id, id);
        assert_eq!(log.created_at, entry.created_at);
        assert_eq!(log.provider_name, entry.provider_name);
        assert_eq!(log.cli_id, entry.cli_id);
        assert_eq!(log.method, entry.method);
        assert_eq!(log.path, entry.path);
        assert_eq!(log.status_code, entry.status_code);
        assert_eq!(log.is_streaming, entry.is_streaming);
        assert_eq!(log.request_model, entry.request_model);
        assert_eq!(log.upstream_model, entry.upstream_model);
        assert_eq!(log.protocol_type, entry.protocol_type);
        assert_eq!(log.input_tokens, entry.input_tokens);
        assert_eq!(log.output_tokens, entry.output_tokens);
        assert_eq!(log.cache_creation_tokens, entry.cache_creation_tokens);
        assert_eq!(log.cache_read_tokens, entry.cache_read_tokens);
        assert_eq!(log.ttfb_ms, entry.ttfb_ms);
        assert_eq!(log.duration_ms, entry.duration_ms);
        assert_eq!(log.stop_reason, entry.stop_reason);
        assert_eq!(log.error_message, entry.error_message);
        assert_eq!(log.event_type, "history");
    }

    /// Test: query_recent_logs limit=1 只返回 1 条（多条数据时）
    #[test]
    fn test_query_limit_respected() {
        let db = make_test_db();
        let entry = make_full_entry();
        db.insert_request_log(&entry).unwrap();
        db.insert_request_log(&entry).unwrap();
        db.insert_request_log(&entry).unwrap();

        let logs = db.query_recent_logs(1).unwrap();
        assert_eq!(logs.len(), 1, "limit=1 时应只返回 1 条");
    }

    /// Test: query_recent_logs 按 created_at 降序排列（最新的在前）
    #[test]
    fn test_query_descending_order() {
        let db = make_test_db();

        // 插入三条不同时间的记录
        let mut entry1 = make_full_entry();
        entry1.created_at = 1_000_000;
        db.insert_request_log(&entry1).unwrap();

        let mut entry2 = make_full_entry();
        entry2.created_at = 3_000_000;
        db.insert_request_log(&entry2).unwrap();

        let mut entry3 = make_full_entry();
        entry3.created_at = 2_000_000;
        db.insert_request_log(&entry3).unwrap();

        let logs = db.query_recent_logs(10).unwrap();
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].created_at, 3_000_000, "最新记录应排第一");
        assert_eq!(logs[1].created_at, 2_000_000, "次新记录应排第二");
        assert_eq!(logs[2].created_at, 1_000_000, "最旧记录应排第三");
    }

    /// Test: LogEntry 所有 18 个字段均可正确写入和读取（含 Option 字段为 None 的情况）
    #[test]
    fn test_all_option_fields_none() {
        let db = make_test_db();
        let entry = LogEntry {
            created_at: 1_700_000_000_001,
            provider_name: "minimal-provider".to_string(),
            cli_id: "codex".to_string(),
            method: "GET".to_string(),
            path: "/health".to_string(),
            status_code: None,
            is_streaming: 1,
            request_model: None,
            upstream_model: None,
            protocol_type: "open_ai_chat_completions".to_string(),
            input_tokens: None,
            output_tokens: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            ttfb_ms: None,
            duration_ms: None,
            stop_reason: None,
            error_message: None,
        };

        let id = db.insert_request_log(&entry).unwrap();
        assert!(id > 0);

        let logs = db.query_recent_logs(10).unwrap();
        assert_eq!(logs.len(), 1);
        let log = &logs[0];

        // 验证 NOT NULL 字段
        assert_eq!(log.created_at, entry.created_at);
        assert_eq!(log.provider_name, entry.provider_name);
        assert_eq!(log.cli_id, entry.cli_id);
        assert_eq!(log.method, entry.method);
        assert_eq!(log.path, entry.path);
        assert_eq!(log.is_streaming, 1);
        assert_eq!(log.protocol_type, entry.protocol_type);

        // 验证所有 Option 字段均为 None
        assert!(log.status_code.is_none(), "status_code 应为 None");
        assert!(log.request_model.is_none(), "request_model 应为 None");
        assert!(log.upstream_model.is_none(), "upstream_model 应为 None");
        assert!(log.input_tokens.is_none(), "input_tokens 应为 None");
        assert!(log.output_tokens.is_none(), "output_tokens 应为 None");
        assert!(
            log.cache_creation_tokens.is_none(),
            "cache_creation_tokens 应为 None"
        );
        assert!(
            log.cache_read_tokens.is_none(),
            "cache_read_tokens 应为 None"
        );
        assert!(log.ttfb_ms.is_none(), "ttfb_ms 应为 None");
        assert!(log.duration_ms.is_none(), "duration_ms 应为 None");
        assert!(log.stop_reason.is_none(), "stop_reason 应为 None");
        assert!(log.error_message.is_none(), "error_message 应为 None");
    }

    /// Test: insert 流式记录（token=None），update_streaming_log 填充后 query 验证字段正确
    #[test]
    fn test_update_streaming_log_fills_tokens() {
        let db = make_test_db();

        // 插入一条流式记录（token 字段全为 None）
        let entry = LogEntry {
            created_at: 1_700_000_000_000,
            provider_name: "test-provider".to_string(),
            cli_id: "claude".to_string(),
            method: "POST".to_string(),
            path: "/v1/messages".to_string(),
            status_code: Some(200),
            is_streaming: 1,
            request_model: Some("claude-3-5-sonnet".to_string()),
            upstream_model: None,
            protocol_type: "anthropic".to_string(),
            input_tokens: None,
            output_tokens: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            ttfb_ms: None,
            duration_ms: None,
            stop_reason: None,
            error_message: None,
        };
        let id = db.insert_request_log(&entry).unwrap();

        // UPDATE 填充 token/ttfb/duration/stop_reason
        let data = StreamTokenData {
            input_tokens: Some(100),
            output_tokens: Some(50),
            cache_creation_tokens: Some(10),
            cache_read_tokens: Some(5),
            stop_reason: Some("end_turn".to_string()),
        };
        db.update_streaming_log(id, &data, Some(120), Some(500))
            .unwrap();

        // query 验证字段正确
        let logs = db.query_recent_logs(10).unwrap();
        assert_eq!(logs.len(), 1);
        let log = &logs[0];
        assert_eq!(log.input_tokens, Some(100));
        assert_eq!(log.output_tokens, Some(50));
        assert_eq!(log.cache_creation_tokens, Some(10));
        assert_eq!(log.cache_read_tokens, Some(5));
        assert_eq!(log.stop_reason, Some("end_turn".to_string()));
        assert_eq!(log.ttfb_ms, Some(120));
        assert_eq!(log.duration_ms, Some(500));
    }

    /// Test: UPDATE 不存在的 id 不 panic，返回 Ok（affected rows = 0）
    #[test]
    fn test_update_streaming_log_nonexistent_id() {
        let db = make_test_db();

        // 对不存在的 id=999 执行 UPDATE，应返回 Ok（0 行受影响）
        let data = StreamTokenData {
            input_tokens: Some(10),
            output_tokens: Some(5),
            cache_creation_tokens: None,
            cache_read_tokens: None,
            stop_reason: Some("end_turn".to_string()),
        };
        let result = db.update_streaming_log(999, &data, None, None);
        assert!(result.is_ok(), "UPDATE 不存在的 id 应返回 Ok，实际: {:?}", result);
    }

    /// Test: UPDATE 部分字段为 None，验证字段按预期写入（None 字段写为 SQL NULL）
    #[test]
    fn test_update_streaming_log_partial_none() {
        let db = make_test_db();

        let entry = LogEntry {
            created_at: 1_700_000_000_000,
            provider_name: "test-provider".to_string(),
            cli_id: "claude".to_string(),
            method: "POST".to_string(),
            path: "/v1/messages".to_string(),
            status_code: Some(200),
            is_streaming: 1,
            request_model: None,
            upstream_model: None,
            protocol_type: "anthropic".to_string(),
            input_tokens: None,
            output_tokens: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            ttfb_ms: None,
            duration_ms: None,
            stop_reason: None,
            error_message: None,
        };
        let id = db.insert_request_log(&entry).unwrap();

        // cache_creation_tokens 和 cache_read_tokens 为 None
        let data = StreamTokenData {
            input_tokens: Some(50),
            output_tokens: Some(20),
            cache_creation_tokens: None,
            cache_read_tokens: None,
            stop_reason: Some("max_tokens".to_string()),
        };
        db.update_streaming_log(id, &data, Some(80), None).unwrap();

        let logs = db.query_recent_logs(10).unwrap();
        assert_eq!(logs.len(), 1);
        let log = &logs[0];
        assert_eq!(log.input_tokens, Some(50));
        assert_eq!(log.output_tokens, Some(20));
        assert!(log.cache_creation_tokens.is_none(), "cache_creation_tokens 应为 None");
        assert!(log.cache_read_tokens.is_none(), "cache_read_tokens 应为 None");
        assert_eq!(log.stop_reason, Some("max_tokens".to_string()));
        assert_eq!(log.ttfb_ms, Some(80));
        assert!(log.duration_ms.is_none(), "duration_ms 应为 None");
    }

    /// Test: TrafficLogPayload 能从 LogEntry + id + type 构建，derive Serialize 正确
    #[test]
    fn test_traffic_log_payload_from_entry() {
        let entry = make_full_entry();
        let payload = TrafficLogPayload::from_entry(42, &entry, "new");

        assert_eq!(payload.id, 42);
        assert_eq!(payload.event_type, "new");
        assert_eq!(payload.created_at, entry.created_at);
        assert_eq!(payload.provider_name, entry.provider_name);
        assert_eq!(payload.cli_id, entry.cli_id);
        assert_eq!(payload.method, entry.method);
        assert_eq!(payload.path, entry.path);
        assert_eq!(payload.status_code, entry.status_code);
        assert_eq!(payload.is_streaming, entry.is_streaming);
        assert_eq!(payload.request_model, entry.request_model);
        assert_eq!(payload.upstream_model, entry.upstream_model);
        assert_eq!(payload.protocol_type, entry.protocol_type);
        assert_eq!(payload.input_tokens, entry.input_tokens);
        assert_eq!(payload.output_tokens, entry.output_tokens);
        assert_eq!(payload.cache_creation_tokens, entry.cache_creation_tokens);
        assert_eq!(payload.cache_read_tokens, entry.cache_read_tokens);
        assert_eq!(payload.ttfb_ms, entry.ttfb_ms);
        assert_eq!(payload.duration_ms, entry.duration_ms);
        assert_eq!(payload.stop_reason, entry.stop_reason);
        assert_eq!(payload.error_message, entry.error_message);

        // 验证 Serialize：event_type 应序列化为 "type" 键
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["type"], "new", "event_type 应序列化为 type 键");
        assert_eq!(json["id"], 42);
        assert!(json.get("event_type").is_none(), "不应有 event_type 键");
    }
}
