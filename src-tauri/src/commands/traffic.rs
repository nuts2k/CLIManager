/// 查询最近 N 条流量日志
#[tauri::command]
pub async fn get_recent_logs(
    traffic_db: tauri::State<'_, crate::traffic::TrafficDb>,
    limit: Option<i64>,
) -> Result<Vec<crate::traffic::log::TrafficLogPayload>, String> {
    let limit = limit.unwrap_or(100).min(1000);
    traffic_db
        .query_recent_logs(limit)
        .map_err(|e| e.to_string())
}
