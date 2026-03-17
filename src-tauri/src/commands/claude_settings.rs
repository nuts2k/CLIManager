use crate::adapter::json_merge::strip_protected_fields;
use crate::error::AppError;
use crate::storage::icloud::{
    read_claude_settings_overlay, write_claude_settings_overlay, OverlayStorageInfo,
};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::Manager;

// ============================================================
// overlay apply 通知模型（实时事件与 startup 缓存共用）
// ============================================================

/// apply 触发来源
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApplySource {
    /// 用户保存时触发
    Save,
    /// 应用启动时 best-effort 触发
    Startup,
    /// iCloud watcher 检测到文件变更时触发
    Watcher,
}

/// 通知类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyNotificationKind {
    /// apply 成功
    Success,
    /// apply 失败
    Failed,
    /// overlay 中包含保护字段，已被忽略
    ProtectedFieldsIgnored,
}

/// overlay apply 通知（统一 payload，供实时事件与 startup 缓存共用）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClaudeOverlayApplyNotification {
    /// 通知类型
    pub kind: ApplyNotificationKind,
    /// 触发来源
    pub source: ApplySource,
    /// settings.json 路径（成功时填写）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings_path: Option<String>,
    /// 错误信息（failed 时填写）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 被忽略的保护字段路径列表（protected_fields_ignored 时填写）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
}

// ============================================================
// startup 通知缓存队列（仅缓存 source="startup" 期间的通知）
// ============================================================

/// 启动阶段 overlay apply 通知缓存队列。
/// setup 阶段比 WebView 事件监听更早完成，直接 emit 会丢失，
/// 因此改为写入此队列，由前端 useSyncListener 挂载后主动 take。
pub struct ClaudeOverlayStartupNotificationQueue(pub Mutex<Vec<ClaudeOverlayApplyNotification>>);

impl ClaudeOverlayStartupNotificationQueue {
    pub fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    /// 追加一条通知到缓存队列
    fn push(&self, notification: ClaudeOverlayApplyNotification) {
        let mut queue = self.0.lock().unwrap();
        queue.push(notification);
    }

    /// 一次性取出并清空队列（take 语义，避免重复 toast）
    fn take_all(&self) -> Vec<ClaudeOverlayApplyNotification> {
        let mut queue = self.0.lock().unwrap();
        std::mem::take(&mut *queue)
    }
}

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

fn resolve_claude_apply_provider(
    local_settings: &crate::storage::local::LocalSettings,
    real_provider: crate::provider::Provider,
) -> Result<crate::provider::Provider, AppError> {
    let in_proxy_takeover = local_settings
        .proxy_takeover
        .as_ref()
        .is_some_and(|takeover| takeover.cli_ids.iter().any(|cli_id| cli_id == "claude"));

    if !in_proxy_takeover {
        return Ok(real_provider);
    }

    let port = crate::proxy::proxy_port_for_cli("claude").ok_or_else(|| {
        AppError::Validation("Claude 代理端口未配置，无法保留代理接管配置".to_string())
    })?;

    Ok(crate::commands::proxy::make_proxy_provider(
        "claude",
        port,
        &real_provider,
    ))
}

fn resolve_claude_config_dir_with_home(
    local_settings: &crate::storage::local::LocalSettings,
    home: &Path,
) -> PathBuf {
    local_settings
        .cli_paths
        .claude_config_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".claude"))
}

fn resolve_claude_settings_path_with_home(
    local_settings: &crate::storage::local::LocalSettings,
    home: &Path,
) -> PathBuf {
    resolve_claude_config_dir_with_home(local_settings, home).join("settings.json")
}

fn resolve_claude_settings_path(
    local_settings: &crate::storage::local::LocalSettings,
) -> Result<PathBuf, AppError> {
    let home = dirs::home_dir().ok_or(AppError::ICloudUnavailable)?;
    Ok(resolve_claude_settings_path_with_home(
        local_settings,
        &home,
    ))
}

// ============================================================
// apply 核心实现
// ============================================================

/// 执行 overlay apply：将 overlay 深度合并到 Claude `settings.json`。
///
/// - source="save"/"watcher"：实时 emit 到前端
/// - source="startup"：写入缓存队列（startup 期间前端 listener 尚未挂载）
///
/// 返回 Ok(()) 表示 apply 成功或 overlay 为空（noop）。
/// 返回 Err 表示 overlay JSON 非法或 settings.json 无法写入。
pub fn apply_claude_settings_overlay(
    app_handle: &tauri::AppHandle,
    source: ApplySource,
) -> Result<(), AppError> {
    // 1. 读取当前 overlay
    let (overlay_text_opt, _info) = read_claude_settings_overlay()?;

    // overlay 文件不存在 => noop（best-effort）
    let overlay_text = match overlay_text_opt {
        None => return Ok(()),
        Some(t) => t,
    };

    // 2. 校验 overlay：必须是合法 JSON 且 root 为 object
    let overlay_val: serde_json::Value = serde_json::from_str(&overlay_text)
        .map_err(|e| AppError::Validation(format!("overlay JSON 不合法: {}", e)))?;
    if !overlay_val.is_object() {
        return Err(AppError::Validation(
            "overlay JSON root 必须是 object".to_string(),
        ));
    }

    // 3. 检查保护字段，生成 protected_fields_ignored 通知
    let strip_result = strip_protected_fields(&overlay_val)?;
    if !strip_result.stripped_paths.is_empty() {
        let notification = ClaudeOverlayApplyNotification {
            kind: ApplyNotificationKind::ProtectedFieldsIgnored,
            source: source.clone(),
            settings_path: None,
            error: None,
            paths: Some(strip_result.stripped_paths.clone()),
        };
        deliver_notification(app_handle, &source, notification);
    }

    // 4. 读取 local settings，查找 claude 的活跃 provider
    let local_settings = crate::storage::local::read_local_settings().unwrap_or_default();
    let active_provider_id = local_settings
        .active_providers
        .get("claude")
        .and_then(|v| v.as_deref());

    // 5. 执行 apply
    let apply_result = if let Some(provider_id) = active_provider_id {
        // 有活跃 provider：通过 ClaudeAdapter.patch() 执行（保证保护字段由 Provider 决定）
        let providers_dir = crate::storage::icloud::get_icloud_providers_dir()?;
        match crate::storage::icloud::get_provider_in(&providers_dir, provider_id) {
            Ok(provider) => {
                let provider = resolve_claude_apply_provider(&local_settings, provider)?;
                let adapter =
                    crate::commands::provider::get_adapter_for_cli_pub("claude", &local_settings)?;
                adapter.patch(&provider)
            }
            Err(e) => Err(e),
        }
    } else {
        // 无活跃 provider：只做 overlay 合并，但必须 strip 保护字段
        apply_overlay_without_provider(&overlay_text, &local_settings)
    };

    // 6. 根据结果生成并分发通知
    match apply_result {
        Ok(patch_result) => {
            let settings_path = patch_result.files_written.first().cloned();
            let notification = ClaudeOverlayApplyNotification {
                kind: ApplyNotificationKind::Success,
                source: source.clone(),
                settings_path,
                error: None,
                paths: None,
            };
            deliver_notification(app_handle, &source, notification);
            Ok(())
        }
        Err(e) => {
            let error_str = e.to_string();
            let notification = ClaudeOverlayApplyNotification {
                kind: ApplyNotificationKind::Failed,
                source: source.clone(),
                settings_path: None,
                error: Some(error_str.clone()),
                paths: None,
            };
            deliver_notification(app_handle, &source, notification);
            Err(AppError::Validation(error_str.clone()))
        }
    }
}

/// 无活跃 Provider 时：直接将 overlay 合并到 settings.json，但 strip 保护字段。
fn apply_overlay_without_provider(
    overlay_text: &str,
    local_settings: &crate::storage::local::LocalSettings,
) -> Result<crate::adapter::PatchResult, AppError> {
    use crate::adapter::json_merge::{merge_with_null_delete, strip_protected_fields};
    use crate::storage::atomic_write;
    use std::fs;

    let settings_path = resolve_claude_settings_path(local_settings)?;
    let claude_dir = settings_path
        .parent()
        .ok_or_else(|| AppError::Validation("Claude settings 路径缺少父目录".to_string()))?
        .to_path_buf();

    // 读取现有 settings
    let existing = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path).map_err(|e| AppError::Io {
            path: settings_path.display().to_string(),
            source: e,
        })?;
        serde_json::from_str::<serde_json::Value>(&content).map_err(|_| {
            AppError::Validation(format!("现有 {} 不是合法 JSON", settings_path.display()))
        })?;
        content
    } else {
        "{}".to_string()
    };

    // 解析 overlay 并 strip 保护字段
    let overlay_val: serde_json::Value = serde_json::from_str(overlay_text)
        .map_err(|e| AppError::Validation(format!("overlay JSON 不合法: {}", e)))?;
    let strip_result = strip_protected_fields(&overlay_val)?;

    // 深度合并
    let mut base: serde_json::Value = serde_json::from_str(&existing)
        .map_err(|_| AppError::Validation("解析 settings JSON 失败".to_string()))?;
    merge_with_null_delete(&mut base, &strip_result.overlay)?;

    // 写入
    let patched = serde_json::to_string_pretty(&base)?;
    fs::create_dir_all(&claude_dir).map_err(|e| AppError::Io {
        path: claude_dir.display().to_string(),
        source: e,
    })?;
    atomic_write(&settings_path, patched.as_bytes())?;

    Ok(crate::adapter::PatchResult {
        files_written: vec![settings_path.display().to_string()],
        backups_created: vec![],
    })
}

/// 按 source 分流通知：save/watcher 实时 emit，startup 写入缓存队列
fn deliver_notification(
    app_handle: &tauri::AppHandle,
    source: &ApplySource,
    notification: ClaudeOverlayApplyNotification,
) {
    let event_name = match notification.kind {
        ApplyNotificationKind::Success => "claude-overlay-apply-success",
        ApplyNotificationKind::Failed => "claude-overlay-apply-failed",
        ApplyNotificationKind::ProtectedFieldsIgnored => "claude-overlay-protected-fields-ignored",
    };

    match source {
        ApplySource::Save | ApplySource::Watcher => {
            // 实时 emit 到前端
            use tauri::Emitter;
            if let Err(e) = app_handle.emit(event_name, &notification) {
                log::error!("emit {} 失败: {:?}", event_name, e);
            }
        }
        ApplySource::Startup => {
            // startup 不依赖实时 emit，写入缓存队列并记录日志
            log::info!(
                "startup overlay apply 通知入队: kind={:?}, error={:?}, paths={:?}",
                notification.kind,
                notification.error,
                notification.paths
            );
            if let Some(queue) = app_handle.try_state::<ClaudeOverlayStartupNotificationQueue>() {
                queue.push(notification);
            } else {
                log::warn!("startup 通知队列未注册，丢弃通知");
            }
        }
    }
}

// ============================================================
// Tauri 命令
// ============================================================

/// 读取 Claude settings overlay 文件内容及存储信息。
/// 文件不存在时 content 为 null，不报错。
#[tauri::command]
pub fn get_claude_settings_overlay() -> Result<GetClaudeSettingsOverlayResponse, AppError> {
    let (content, storage) = read_claude_settings_overlay()?;
    Ok(GetClaudeSettingsOverlayResponse { content, storage })
}

/// 保存 Claude settings overlay 文件，并立即 apply 到 Claude settings.json（强一致）。
/// 仅做 JSON 校验（合法 JSON 且 root 为 object），通过后规范化格式写入。
/// 写入前通过 SelfWriteTracker 标记路径，避免 watcher 处理自写事件。
#[tauri::command]
pub fn set_claude_settings_overlay(
    app_handle: tauri::AppHandle,
    overlay_json: String,
) -> Result<SetClaudeSettingsOverlayResponse, AppError> {
    // 1. JSON 校验：必须是合法 JSON 且 root 为 object（空字符串允许清空）
    if !overlay_json.trim().is_empty() {
        let value: serde_json::Value = serde_json::from_str(&overlay_json)
            .map_err(|e| AppError::Validation(format!("overlay_json 不是合法 JSON: {}", e)))?;
        if !value.is_object() {
            return Err(AppError::Validation(
                "overlay_json 的 root 必须是 JSON object".to_string(),
            ));
        }
    }

    // 2. 规范化格式（避免 diff 抖动）；空字符串直接写入空串
    let normalized = if overlay_json.trim().is_empty() {
        overlay_json.clone()
    } else {
        let value: serde_json::Value = serde_json::from_str(&overlay_json)?;
        serde_json::to_string_pretty(&value)?
    };

    // 3. 解析 overlay 文件路径（用于 SelfWriteTracker 记录）
    let (overlay_path, _) = crate::storage::icloud::get_claude_overlay_path()?;

    // 4. 通过 SelfWriteTracker 标记，避免后续 watcher 处理自写事件
    let tracker = app_handle.state::<crate::watcher::SelfWriteTracker>();
    tracker.record_write(overlay_path.clone());

    // 5. 原子写入
    let storage = write_claude_settings_overlay(&normalized)?;

    // 6. 立即 apply（强一致，COVL-09）
    if let Err(e) = apply_claude_settings_overlay(&app_handle, ApplySource::Save) {
        // apply 失败：set 整体返回 Err（强一致）
        // emit 已在 apply_claude_settings_overlay 内部完成
        return Err(e);
    }

    Ok(SetClaudeSettingsOverlayResponse { storage })
}

/// Tauri 命令包装：对外暴露 apply_claude_settings_overlay（source 固定为 save）。
/// 前端一般不直接调用此命令（保存时由 set_claude_settings_overlay 内部触发）。
/// 此命令主要供调试或特殊场景使用。
#[tauri::command]
pub fn apply_claude_settings_overlay_cmd(app_handle: tauri::AppHandle) -> Result<(), AppError> {
    apply_claude_settings_overlay(&app_handle, ApplySource::Save)
}

/// 一次性取出并清空 startup 期间积累的 overlay apply 通知（take 语义）。
/// 前端 useSyncListener 挂载完成后调用此命令，确保 startup 结果不会因时序丢失。
#[tauri::command]
pub fn take_claude_overlay_startup_notifications(
    app_handle: tauri::AppHandle,
) -> Result<Vec<ClaudeOverlayApplyNotification>, AppError> {
    let queue = app_handle.state::<ClaudeOverlayStartupNotificationQueue>();
    Ok(queue.take_all())
}

#[cfg(test)]
mod tests {
    use super::{
        apply_overlay_without_provider, resolve_claude_apply_provider,
        resolve_claude_settings_path_with_home,
    };
    use crate::provider::{ProtocolType, Provider};
    use crate::storage::local::{CliPaths, LocalSettings, ProxyTakeover};
    use serde_json::Value;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn test_provider() -> Provider {
        Provider {
            id: "p1".to_string(),
            cli_id: "claude".to_string(),
            name: "Test Provider".to_string(),
            protocol_type: ProtocolType::Anthropic,
            api_key: "sk-real-key".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            model_config: None,
            notes: None,
            test_model: None,
            upstream_model: None,
            upstream_model_map: None,
            created_at: 0,
            updated_at: 0,
            schema_version: 1,
        }
    }

    #[test]
    fn test_resolve_claude_apply_provider_uses_real_provider_in_direct_mode() {
        let settings = LocalSettings::default();

        let resolved = resolve_claude_apply_provider(&settings, test_provider()).unwrap();

        assert_eq!(resolved.api_key, "sk-real-key");
        assert_eq!(resolved.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn test_resolve_claude_apply_provider_preserves_proxy_takeover_credentials() {
        let mut settings = LocalSettings::default();
        settings.proxy_takeover = Some(ProxyTakeover {
            cli_ids: vec!["claude".to_string()],
        });

        let resolved = resolve_claude_apply_provider(&settings, test_provider()).unwrap();

        assert_eq!(resolved.api_key, "PROXY_MANAGED");
        assert_eq!(resolved.base_url, "http://127.0.0.1:15800");
        assert_eq!(resolved.model, "claude-sonnet-4-20250514");
        assert!(matches!(resolved.protocol_type, ProtocolType::Anthropic));
    }

    #[test]
    fn test_resolve_claude_settings_path_with_home_uses_custom_config_dir() {
        let home = Path::new("/fake-home");
        let settings = LocalSettings {
            cli_paths: CliPaths {
                claude_config_dir: Some("/custom/claude".to_string()),
                codex_config_dir: None,
            },
            ..LocalSettings::default()
        };

        let path = resolve_claude_settings_path_with_home(&settings, home);

        assert_eq!(path, PathBuf::from("/custom/claude/settings.json"));
    }

    #[test]
    fn test_apply_overlay_without_provider_uses_custom_claude_config_dir() {
        let tmp = TempDir::new().unwrap();
        let custom_dir = tmp.path().join("claude-custom");
        std::fs::create_dir_all(&custom_dir).unwrap();

        let settings = LocalSettings {
            cli_paths: CliPaths {
                claude_config_dir: Some(custom_dir.to_string_lossy().to_string()),
                codex_config_dir: None,
            },
            ..LocalSettings::default()
        };

        let result = apply_overlay_without_provider(
            r#"{"permissions": {"allow": ["Bash"]}, "env": {"EXTRA": "custom-dir"}}"#,
            &settings,
        )
        .unwrap();

        let written_path = custom_dir.join("settings.json");
        assert_eq!(
            result.files_written,
            vec![written_path.display().to_string()]
        );

        let patched: Value =
            serde_json::from_str(&std::fs::read_to_string(&written_path).unwrap()).unwrap();
        assert_eq!(patched["permissions"]["allow"], serde_json::json!(["Bash"]));
        assert_eq!(patched["env"]["EXTRA"], "custom-dir");
    }
}
