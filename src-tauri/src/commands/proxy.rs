use tauri::State;

use crate::provider::ProtocolType;
use crate::proxy::{ProxyService, ProxyStatusInfo, UpstreamTarget};

/// 解析 protocol_type 字符串为枚举
fn parse_protocol_type(s: &str) -> Result<ProtocolType, String> {
    match s {
        "anthropic" => Ok(ProtocolType::Anthropic),
        "open_ai_compatible" => Ok(ProtocolType::OpenAiCompatible),
        other => Err(format!("无效的 protocol_type: '{}', 期望 'anthropic' 或 'open_ai_compatible'", other)),
    }
}

/// 启动指定 CLI 的代理服务器
#[tauri::command]
pub async fn proxy_start(
    cli_id: String,
    port: u16,
    api_key: String,
    base_url: String,
    protocol_type: String,
    proxy_service: State<'_, ProxyService>,
) -> Result<(), String> {
    let pt = parse_protocol_type(&protocol_type)?;
    let target = UpstreamTarget {
        api_key,
        base_url,
        protocol_type: pt,
    };
    proxy_service
        .start(&cli_id, port, target)
        .await
        .map_err(|e| e.to_string())
}

/// 停止指定 CLI 的代理服务器
#[tauri::command]
pub async fn proxy_stop(
    cli_id: String,
    proxy_service: State<'_, ProxyService>,
) -> Result<(), String> {
    proxy_service
        .stop(&cli_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取所有代理服务器的状态
#[tauri::command]
pub async fn proxy_status(
    proxy_service: State<'_, ProxyService>,
) -> Result<ProxyStatusInfo, String> {
    Ok(proxy_service.status().await)
}

/// 动态更新指定 CLI 的上游目标
#[tauri::command]
pub async fn proxy_update_upstream(
    cli_id: String,
    api_key: String,
    base_url: String,
    protocol_type: String,
    proxy_service: State<'_, ProxyService>,
) -> Result<(), String> {
    let pt = parse_protocol_type(&protocol_type)?;
    let target = UpstreamTarget {
        api_key,
        base_url,
        protocol_type: pt,
    };
    proxy_service
        .update_upstream(&cli_id, target)
        .await
        .map_err(|e| e.to_string())
}
