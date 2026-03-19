pub mod db;
pub mod log;
pub mod rollup;
pub mod schema;

use std::sync::Mutex;

/// Traffic DB 托管状态
///
/// 通过 Tauri 的 .manage() 注入为全局状态，后续 Phase 通过
/// app_handle.try_state::<TrafficDb>() 安全访问（不 panic）。
///
/// 使用 std::sync::Mutex（非 tokio::sync::Mutex）：
/// rusqlite Connection 是同步 API，std Mutex 更高效，符合 Tauri 2 最佳实践。
pub struct TrafficDb {
    pub conn: Mutex<rusqlite::Connection>,
}

pub const HIDDEN_REQUESTS_SQL_FILTER: &str =
    "path != '/v1/token_count' AND path NOT LIKE '%/count_tokens'";

/// 是否应从流量视图中隐藏该请求路径
pub fn should_hide_request_from_traffic_views(path: &str) -> bool {
    path == "/v1/token_count" || path.ends_with("/count_tokens")
}

/// 初始化 traffic DB，返回 Option<TrafficDb>
///
/// - Some(TrafficDb)：DB 初始化成功，可通过 .manage() 注入 Tauri 状态
/// - None：初始化失败（降级运行，代理正常工作但不记录流量）
///
/// 调用方（lib.rs setup 闭包）应使用 if let Some 模式，
/// 不得使用 ? 传播错误（RESEARCH.md Pitfall 3）。
pub fn init_traffic_db() -> Option<TrafficDb> {
    db::open_traffic_db().map(|conn| TrafficDb {
        conn: Mutex::new(conn),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_should_hide_request_from_traffic_views() {
        assert!(super::should_hide_request_from_traffic_views("/v1/token_count"));
        assert!(super::should_hide_request_from_traffic_views("/v1/messages/count_tokens"));
        assert!(!super::should_hide_request_from_traffic_views("/v1/messages"));
        assert!(!super::should_hide_request_from_traffic_views("/v1/responses"));
    }
}
