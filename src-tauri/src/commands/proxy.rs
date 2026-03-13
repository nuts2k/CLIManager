use std::path::Path;

use serde::Serialize;
use tauri::{Emitter, State};

use crate::adapter::CliAdapter;
use crate::error::AppError;
use crate::provider::{normalize_origin_base_url, ProtocolType, Provider};
use crate::proxy::{proxy_port_for_cli, ProxyService, ProxyStatusInfo, UpstreamTarget};
use crate::storage::local::{ProxySettings, ProxyTakeover};

/// 解析 protocol_type 字符串为枚举
fn parse_protocol_type(s: &str) -> Result<ProtocolType, String> {
    match s {
        "anthropic" => Ok(ProtocolType::Anthropic),
        "open_ai_compatible" => Ok(ProtocolType::OpenAiCompatible),
        other => Err(format!(
            "无效的 protocol_type: '{}', 期望 'anthropic' 或 'open_ai_compatible'",
            other
        )),
    }
}

fn build_upstream_target(
    api_key: String,
    base_url: String,
    protocol_type: String,
) -> Result<UpstreamTarget, String> {
    let pt = parse_protocol_type(&protocol_type)?;
    let normalized_base_url = normalize_origin_base_url(&base_url)?;

    Ok(UpstreamTarget {
        api_key,
        base_url: normalized_base_url,
        protocol_type: pt,
    })
}

// --- 模式切换返回类型 ---

/// 代理模式全局状态
#[derive(Debug, Clone, Serialize)]
pub struct ProxyModeStatus {
    pub global_enabled: bool,
    pub cli_statuses: Vec<CliProxyStatus>,
}

/// 单个 CLI 的代理状态
#[derive(Debug, Clone, Serialize)]
pub struct CliProxyStatus {
    pub cli_id: String,
    /// 用户开关状态
    pub enabled: bool,
    /// 代理实际运行中
    pub active: bool,
    /// 是否有活跃 Provider
    pub has_provider: bool,
    /// 代理端口
    pub port: Option<u16>,
}

// --- 模式切换内部函数（可测试） ---

/// 获取 CLI 对应的 adapter
fn get_adapter_for_cli(
    cli_id: &str,
    settings: &crate::storage::local::LocalSettings,
) -> Result<Box<dyn CliAdapter>, AppError> {
    crate::commands::provider::get_adapter_for_cli_pub(cli_id, settings)
}

/// 构造代理专用 Provider（临时，不保存到 iCloud）
fn make_proxy_provider(cli_id: &str, port: u16, real_provider: &Provider) -> Provider {
    let (protocol_type, base_url) = match cli_id {
        "codex" => (
            ProtocolType::OpenAiCompatible,
            format!("http://127.0.0.1:{}", port),
        ),
        _ => (
            ProtocolType::Anthropic,
            format!("http://127.0.0.1:{}", port),
        ),
    };

    Provider {
        id: real_provider.id.clone(),
        cli_id: cli_id.to_string(),
        name: real_provider.name.clone(),
        protocol_type,
        api_key: "PROXY_MANAGED".to_string(),
        base_url,
        model: real_provider.model.clone(),
        model_config: real_provider.model_config.clone(),
        notes: None,
        created_at: real_provider.created_at,
        updated_at: real_provider.updated_at,
        schema_version: real_provider.schema_version,
    }
}

/// 内部：开启指定 CLI 的代理模式
pub(crate) async fn _proxy_enable_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    cli_id: &str,
    proxy_service: &ProxyService,
    adapter: Option<Box<dyn CliAdapter + Send>>,
) -> Result<(), AppError> {
    // 所有同步操作（含 adapter）在 block 内完成，确保 adapter 在 .await 前 drop
    let (port, upstream, real_provider, settings) = {
        let settings = crate::storage::local::read_local_settings_from(local_settings_path)?;

        // 1. 检查该 CLI 是否有活跃 Provider
        let provider_id = settings
            .active_providers
            .get(cli_id)
            .and_then(|pid| pid.as_ref())
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "该 CLI ({}) 无活跃 Provider，无法开启代理",
                    cli_id
                ))
            })?;

        // 2. 从 iCloud 读取真实 Provider
        let real_provider =
            crate::storage::icloud::get_provider_in(providers_dir, provider_id)?;

        // 3. 获取端口
        let port = proxy_port_for_cli(cli_id).ok_or_else(|| {
            AppError::Validation(format!("不支持的 CLI: {}", cli_id))
        })?;

        // 4. 构造代理专用 Provider 并 patch CLI 配置
        let proxy_provider = make_proxy_provider(cli_id, port, &real_provider);
        let real_adapter = if let Some(a) = adapter {
            a
        } else {
            get_adapter_for_cli(cli_id, &settings)?
        };
        real_adapter.patch(&proxy_provider)?;

        // 5. 构造 UpstreamTarget（真实凭据）
        let upstream = UpstreamTarget {
            api_key: real_provider.api_key.clone(),
            base_url: real_provider.base_url.clone(),
            protocol_type: real_provider.protocol_type.clone(),
        };

        (port, upstream, real_provider, settings)
        // real_adapter 在此处 drop
    };

    // 6. 启动代理（async 操作，此时 adapter 已 drop）
    if let Err(start_err) = proxy_service.start(cli_id, port, upstream).await {
        // 启动失败：回滚 CLI 配置为真实凭据
        log::error!(
            "代理启动失败，回滚 CLI 配置: cli_id={}, err={}",
            cli_id,
            start_err
        );
        if let Err(rollback_err) = get_adapter_for_cli(cli_id, &settings)
            .and_then(|a| a.patch(&real_provider).map(|_| ()))
        {
            log::error!("回滚 CLI 配置也失败: {}", rollback_err);
        }
        return Err(AppError::Validation(format!(
            "代理启动失败: {}",
            start_err
        )));
    }

    // 7. 更新 local.json：proxy_takeover + proxy.cli_enabled
    let mut settings = crate::storage::local::read_local_settings_from(local_settings_path)?;

    let takeover = settings
        .proxy_takeover
        .get_or_insert_with(ProxyTakeover::default);
    if !takeover.cli_ids.contains(&cli_id.to_string()) {
        takeover.cli_ids.push(cli_id.to_string());
    }

    let proxy = settings.proxy.get_or_insert_with(ProxySettings::default);
    proxy.cli_enabled.insert(cli_id.to_string(), true);

    crate::storage::local::write_local_settings_to(local_settings_path, &settings)?;

    log::info!("代理模式已开启: cli_id={}", cli_id);
    Ok(())
}

/// 内部：关闭指定 CLI 的代理模式
pub(crate) async fn _proxy_disable_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    cli_id: &str,
    proxy_service: &ProxyService,
    adapter: Option<Box<dyn CliAdapter + Send>>,
) -> Result<(), AppError> {
    // 所有同步 adapter 操作在 block 内完成
    {
        let settings = crate::storage::local::read_local_settings_from(local_settings_path)?;

        // 1. 读取当前活跃 Provider 并还原 CLI 配置
        let provider_id = settings
            .active_providers
            .get(cli_id)
            .and_then(|pid| pid.as_ref());

        if let Some(pid) = provider_id {
            match crate::storage::icloud::get_provider_in(providers_dir, pid) {
                Ok(real_provider) => {
                    let real_adapter = if let Some(a) = adapter {
                        a
                    } else {
                        get_adapter_for_cli(cli_id, &settings)?
                    };
                    if let Err(patch_err) = real_adapter.patch(&real_provider) {
                        log::error!(
                            "还原 CLI 配置失败，尝试 clear: cli_id={}, err={}",
                            cli_id,
                            patch_err
                        );
                        // 尝试 clear 作为回退
                        if let Ok(fallback_adapter) = get_adapter_for_cli(cli_id, &settings) {
                            if let Err(clear_err) = fallback_adapter.clear() {
                                log::error!("clear 也失败: {}", clear_err);
                            }
                        }
                    }
                }
                Err(err) => {
                    log::warn!(
                        "无法读取 Provider 还原 CLI 配置: cli_id={}, provider_id={}, err={}",
                        cli_id,
                        pid,
                        err
                    );
                }
            }
        }
        // adapter 在此处 drop
    }

    // 2. 停止代理（best-effort，async）
    if let Err(stop_err) = proxy_service.stop(cli_id).await {
        log::warn!(
            "停止代理失败（best-effort）: cli_id={}, err={}",
            cli_id,
            stop_err
        );
    }

    // 3. 更新 local.json
    let mut settings = crate::storage::local::read_local_settings_from(local_settings_path)?;

    if let Some(ref mut takeover) = settings.proxy_takeover {
        takeover.cli_ids.retain(|id| id != cli_id);
    }

    if let Some(ref mut proxy) = settings.proxy {
        proxy.cli_enabled.insert(cli_id.to_string(), false);
    }

    crate::storage::local::write_local_settings_to(local_settings_path, &settings)?;

    log::info!("代理模式已关闭: cli_id={}", cli_id);
    Ok(())
}

// --- 底层代理命令（Phase 8 保留） ---

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
    let target = build_upstream_target(api_key, base_url, protocol_type)?;
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
    proxy_service.stop(&cli_id).await.map_err(|e| e.to_string())
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
    let target = build_upstream_target(api_key, base_url, protocol_type)?;
    proxy_service
        .update_upstream(&cli_id, target)
        .await
        .map_err(|e| e.to_string())
}

// --- 模式切换 Tauri 命令 ---

/// 开启指定 CLI 的代理模式
#[tauri::command]
pub async fn proxy_enable(
    cli_id: String,
    proxy_service: State<'_, ProxyService>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let providers_dir = crate::storage::icloud::get_icloud_providers_dir()
        .map_err(|e| e.to_string())?;
    let settings_path = crate::storage::local::get_local_settings_path();

    _proxy_enable_in(&providers_dir, &settings_path, &cli_id, &proxy_service, None)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit("proxy-mode-changed", ());
    Ok(())
}

/// 关闭指定 CLI 的代理模式
#[tauri::command]
pub async fn proxy_disable(
    cli_id: String,
    proxy_service: State<'_, ProxyService>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let providers_dir = crate::storage::icloud::get_icloud_providers_dir()
        .map_err(|e| e.to_string())?;
    let settings_path = crate::storage::local::get_local_settings_path();

    _proxy_disable_in(&providers_dir, &settings_path, &cli_id, &proxy_service, None)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit("proxy-mode-changed", ());
    Ok(())
}

/// 设置全局代理开关
#[tauri::command]
pub async fn proxy_set_global(
    enabled: bool,
    proxy_service: State<'_, ProxyService>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let providers_dir = crate::storage::icloud::get_icloud_providers_dir()
        .map_err(|e| e.to_string())?;
    let settings_path = crate::storage::local::get_local_settings_path();

    // 读取当前设置
    let mut settings = crate::storage::local::read_local_settings_from(&settings_path)
        .map_err(|e| e.to_string())?;

    let proxy = settings.proxy.get_or_insert_with(ProxySettings::default);
    proxy.global_enabled = enabled;
    crate::storage::local::write_local_settings_to(&settings_path, &settings)
        .map_err(|e| e.to_string())?;

    if enabled {
        // 开启：对所有 cli_enabled=true 的 CLI 启动代理
        let cli_ids_to_enable: Vec<String> = settings
            .proxy
            .as_ref()
            .map(|p| {
                p.cli_enabled
                    .iter()
                    .filter_map(|(cli_id, &en)| if en { Some(cli_id.clone()) } else { None })
                    .collect()
            })
            .unwrap_or_default();

        for cli_id in cli_ids_to_enable {
            // 跳过已在运行的
            let status = proxy_service.status().await;
            let already_running = status
                .servers
                .iter()
                .any(|s| s.cli_id == cli_id && s.running);
            if already_running {
                continue;
            }

            if let Err(err) = _proxy_enable_in(
                &providers_dir,
                &settings_path,
                &cli_id,
                &proxy_service,
                None,
            )
            .await
            {
                log::error!("全局开启代理时 {} 失败: {}", cli_id, err);
            }
        }
    } else {
        // 关闭：对所有 proxy_takeover.cli_ids 中的 CLI 关闭代理
        let cli_ids_to_disable: Vec<String> = settings
            .proxy_takeover
            .as_ref()
            .map(|t| t.cli_ids.clone())
            .unwrap_or_default();

        for cli_id in cli_ids_to_disable {
            if let Err(err) = _proxy_disable_in(
                &providers_dir,
                &settings_path,
                &cli_id,
                &proxy_service,
                None,
            )
            .await
            {
                log::error!("全局关闭代理时 {} 失败: {}", cli_id, err);
            }
        }
    }

    let _ = app_handle.emit("proxy-mode-changed", ());
    Ok(())
}

/// 获取代理模式全局状态
#[tauri::command]
pub async fn proxy_get_mode_status(
    proxy_service: State<'_, ProxyService>,
) -> Result<ProxyModeStatus, String> {
    let settings = crate::storage::local::read_local_settings()
        .map_err(|e| e.to_string())?;

    let proxy = settings.proxy.as_ref();
    let global_enabled = proxy.map_or(false, |p| p.global_enabled);

    // 获取代理实际运行状态
    let running_status = proxy_service.status().await;

    // 所有已知 CLI
    let known_clis = vec!["claude", "codex"];
    let cli_statuses: Vec<CliProxyStatus> = known_clis
        .into_iter()
        .map(|cli_id| {
            let enabled = proxy
                .and_then(|p| p.cli_enabled.get(cli_id))
                .copied()
                .unwrap_or(false);
            let active = running_status
                .servers
                .iter()
                .any(|s| s.cli_id == cli_id && s.running);
            let has_provider = settings
                .active_providers
                .get(cli_id)
                .map_or(false, |pid| pid.is_some());
            let port = proxy_port_for_cli(cli_id);

            CliProxyStatus {
                cli_id: cli_id.to_string(),
                enabled,
                active,
                has_provider,
                port,
            }
        })
        .collect();

    Ok(ProxyModeStatus {
        global_enabled,
        cli_statuses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::{PROXY_PORT_CLAUDE, PROXY_PORT_CODEX};

    #[test]
    fn test_build_upstream_target_normalizes_base_url() {
        let target = build_upstream_target(
            "sk-test".to_string(),
            "https://api.openai.com/".to_string(),
            "open_ai_compatible".to_string(),
        )
        .unwrap();

        assert_eq!(target.base_url, "https://api.openai.com");
        assert!(matches!(
            target.protocol_type,
            ProtocolType::OpenAiCompatible
        ));
    }

    #[test]
    fn test_build_upstream_target_rejects_base_url_with_path() {
        let err = build_upstream_target(
            "sk-test".to_string(),
            "https://api.openai.com/v1".to_string(),
            "open_ai_compatible".to_string(),
        )
        .unwrap_err();

        assert_eq!(err, "Provider base URL must not contain a path");
    }

    #[test]
    fn test_proxy_port_constants() {
        assert_eq!(PROXY_PORT_CLAUDE, 15800);
        assert_eq!(PROXY_PORT_CODEX, 15801);
    }

    #[tokio::test]
    async fn test_proxy_mode_status_default() {
        let service = ProxyService::new();
        let status = service.status().await;
        assert!(status.servers.is_empty());
    }

    #[test]
    fn test_make_proxy_provider_claude() {
        let real = Provider {
            id: "p1".to_string(),
            cli_id: "claude".to_string(),
            name: "Test".to_string(),
            protocol_type: ProtocolType::Anthropic,
            api_key: "sk-real-key".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            model_config: None,
            notes: None,
            created_at: 0,
            updated_at: 0,
            schema_version: 1,
        };

        let proxy = make_proxy_provider("claude", 15800, &real);
        assert_eq!(proxy.api_key, "PROXY_MANAGED");
        assert_eq!(proxy.base_url, "http://127.0.0.1:15800");
        assert!(matches!(proxy.protocol_type, ProtocolType::Anthropic));
        assert_eq!(proxy.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_make_proxy_provider_codex() {
        let real = Provider {
            id: "p2".to_string(),
            cli_id: "codex".to_string(),
            name: "Test Codex".to_string(),
            protocol_type: ProtocolType::OpenAiCompatible,
            api_key: "sk-real-key".to_string(),
            base_url: "https://api.openai.com".to_string(),
            model: "o4-mini".to_string(),
            model_config: None,
            notes: None,
            created_at: 0,
            updated_at: 0,
            schema_version: 1,
        };

        let proxy = make_proxy_provider("codex", 15801, &real);
        assert_eq!(proxy.api_key, "PROXY_MANAGED");
        assert_eq!(proxy.base_url, "http://127.0.0.1:15801");
        assert!(matches!(proxy.protocol_type, ProtocolType::OpenAiCompatible));
    }
}
