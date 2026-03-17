use crate::error::AppError;
use crate::storage::icloud::{
    read_claude_settings_overlay, write_claude_settings_overlay, OverlayStorageInfo,
};
use tauri::Manager;

// ============================================================
// 响应结构体
// ============================================================

/// get_claude_settings_overlay 命令的返回值
#[derive(Debug, serde::Serialize)]
pub struct GetClaudeSettingsOverlayResponse {
    /// overlay 内容（JSON 字符串）；文件不存在时为 null
    pub content: Option<String>,
    /// 存储元信息（位置、路径、sync_enabled）
    pub storage: OverlayStorageInfo,
}

/// set_claude_settings_overlay 命令的返回值
#[derive(Debug, serde::Serialize)]
pub struct SetClaudeSettingsOverlayResponse {
    /// 写入后的存储元信息
    pub storage: OverlayStorageInfo,
}

// ============================================================
// Tauri 命令
// ============================================================

/// 读取 Claude settings overlay 文件内容及存储信息。
/// 文件不存在时 content 为 null，不报错。
#[tauri::command]
pub fn get_claude_settings_overlay(
) -> Result<GetClaudeSettingsOverlayResponse, AppError> {
    let (content, storage) = read_claude_settings_overlay()?;
    Ok(GetClaudeSettingsOverlayResponse { content, storage })
}

/// 保存 Claude settings overlay 文件。
/// 仅做 JSON 校验（合法 JSON 且 root 为 object），通过后规范化格式写入。
/// 写入前通过 SelfWriteTracker 标记路径，避免 watcher 处理自写事件。
#[tauri::command]
pub fn set_claude_settings_overlay(
    app_handle: tauri::AppHandle,
    overlay_json: String,
) -> Result<SetClaudeSettingsOverlayResponse, AppError> {
    // 1. JSON 校验：必须是合法 JSON 且 root 为 object
    let value: serde_json::Value = serde_json::from_str(&overlay_json).map_err(|e| {
        AppError::Validation(format!("overlay_json 不是合法 JSON: {}", e))
    })?;
    if !value.is_object() {
        return Err(AppError::Validation(
            "overlay_json 的 root 必须是 JSON object".to_string(),
        ));
    }

    // 2. 规范化格式（避免 diff 抖动）
    let normalized = serde_json::to_string_pretty(&value)?;

    // 3. 解析 overlay 文件路径（用于 SelfWriteTracker 记录）
    let (overlay_path, _) = crate::storage::icloud::get_claude_overlay_path()?;

    // 4. 通过 SelfWriteTracker 标记，避免后续 watcher 处理自写事件
    let tracker = app_handle.state::<crate::watcher::SelfWriteTracker>();
    tracker.record_write(overlay_path.clone());

    // 5. 原子写入
    let storage = write_claude_settings_overlay(&normalized)?;

    Ok(SetClaudeSettingsOverlayResponse { storage })
}
