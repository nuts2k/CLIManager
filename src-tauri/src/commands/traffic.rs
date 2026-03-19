/// 查询最近 N 条流量日志
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

/// 查询按 Provider 聚合的统计数据
#[tauri::command]
pub async fn get_provider_stats(
    app_handle: tauri::AppHandle,
    range: String,  // "24h" 或 "7d"
) -> Result<Vec<crate::traffic::rollup::ProviderStat>, String> {
    use tauri::Manager;
    let db = app_handle
        .try_state::<crate::traffic::TrafficDb>()
        .ok_or_else(|| "数据库不可用（DB 初始化失败，请检查磁盘空间和权限）".to_string())?;
    db.query_provider_stats(&range).map_err(|e| e.to_string())
}

/// 查询按时间聚合的趋势数据
#[tauri::command]
pub async fn get_time_trend(
    app_handle: tauri::AppHandle,
    range: String,  // "24h" 或 "7d"
) -> Result<Vec<crate::traffic::rollup::TimeStat>, String> {
    use tauri::Manager;
    let db = app_handle
        .try_state::<crate::traffic::TrafficDb>()
        .ok_or_else(|| "数据库不可用（DB 初始化失败，请检查磁盘空间和权限）".to_string())?;
    db.query_time_trend(&range).map_err(|e| e.to_string())
}
