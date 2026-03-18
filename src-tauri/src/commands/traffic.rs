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

/// 查询按 Provider 聚合的统计数据
#[tauri::command]
pub async fn get_provider_stats(
    traffic_db: tauri::State<'_, crate::traffic::TrafficDb>,
    range: String,  // "24h" 或 "7d"
) -> Result<Vec<crate::traffic::rollup::ProviderStat>, String> {
    traffic_db.query_provider_stats(&range).map_err(|e| e.to_string())
}

/// 查询按时间聚合的趋势数据
#[tauri::command]
pub async fn get_time_trend(
    traffic_db: tauri::State<'_, crate::traffic::TrafficDb>,
    range: String,  // "24h" 或 "7d"
) -> Result<Vec<crate::traffic::rollup::TimeStat>, String> {
    traffic_db.query_time_trend(&range).map_err(|e| e.to_string())
}
