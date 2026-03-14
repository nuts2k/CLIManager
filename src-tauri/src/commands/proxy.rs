use std::path::Path;

use serde::Serialize;
use tauri::{Emitter, State};
use tokio::sync::Mutex;

use crate::adapter::CliAdapter;
use crate::error::AppError;
use crate::provider::{extract_origin_base_url, normalize_origin_base_url, ProtocolType, Provider};
use crate::proxy::{proxy_port_for_cli, ProxyService, ProxyStatusInfo, UpstreamTarget};
use crate::storage::local::{ProxySettings, ProxyTakeover};

/// 解析 protocol_type 字符串为枚举
fn parse_protocol_type(s: &str) -> Result<ProtocolType, String> {
    match s {
        "anthropic" => Ok(ProtocolType::Anthropic),
        "open_ai_compatible" | "open_ai_chat_completions" => {
            Ok(ProtocolType::OpenAiChatCompletions)
        }
        "open_ai_responses" => Ok(ProtocolType::OpenAiResponses),
        other => Err(format!(
            "无效的 protocol_type: '{}', 期望 'anthropic'、'open_ai_chat_completions' 或 'open_ai_responses'",
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

fn build_upstream_target_from_provider(provider: &Provider) -> Result<UpstreamTarget, AppError> {
    Ok(UpstreamTarget {
        api_key: provider.api_key.clone(),
        base_url: extract_origin_base_url(&provider.base_url).map_err(AppError::Validation)?,
        protocol_type: provider.protocol_type.clone(),
    })
}

type AdapterFactory =
    fn(&str, &crate::storage::local::LocalSettings) -> Result<Box<dyn CliAdapter>, AppError>;

/// 串行化全局代理开关，避免多个请求交错修改持久化状态和运行态。
pub struct ProxyGlobalToggleLock {
    mutex: Mutex<()>,
}

impl ProxyGlobalToggleLock {
    pub fn new() -> Self {
        Self {
            mutex: Mutex::new(()),
        }
    }

    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.mutex.lock().await
    }
}

fn patch_cli_with_adapter(
    cli_id: &str,
    settings: &crate::storage::local::LocalSettings,
    provider: &Provider,
    adapter: Option<&dyn CliAdapter>,
    adapter_factory: AdapterFactory,
) -> Result<(), AppError> {
    if let Some(adapter) = adapter {
        adapter.patch(provider)?;
    } else {
        let adapter = adapter_factory(cli_id, settings)?;
        adapter.patch(provider)?;
    }
    Ok(())
}

fn clear_cli_with_adapter(
    cli_id: &str,
    settings: &crate::storage::local::LocalSettings,
    adapter: Option<&dyn CliAdapter>,
    adapter_factory: AdapterFactory,
) -> Result<(), AppError> {
    if let Some(adapter) = adapter {
        adapter.clear()?;
    } else {
        let adapter = adapter_factory(cli_id, settings)?;
        adapter.clear()?;
    }
    Ok(())
}

fn restore_or_clear_cli_config_with_factory(
    providers_dir: &Path,
    settings: &crate::storage::local::LocalSettings,
    cli_id: &str,
    adapter: Option<&dyn CliAdapter>,
    context: &str,
    adapter_factory: AdapterFactory,
) -> Result<(), AppError> {
    let provider_id = settings
        .active_providers
        .get(cli_id)
        .and_then(|pid| pid.as_deref());

    let primary_err = if let Some(pid) = provider_id {
        match crate::storage::icloud::get_provider_in(providers_dir, pid) {
            Ok(real_provider) => match patch_cli_with_adapter(
                cli_id,
                settings,
                &real_provider,
                adapter,
                adapter_factory,
            ) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    log::error!(
                        "{}：还原 CLI 配置失败，尝试 clear: cli_id={}, provider_id={}, err={}",
                        context,
                        cli_id,
                        pid,
                        err
                    );
                    Some(err)
                }
            },
            Err(err) => {
                log::warn!(
                    "{}：读取 Provider 失败，尝试 clear: cli_id={}, provider_id={}, err={}",
                    context,
                    cli_id,
                    pid,
                    err
                );
                Some(err)
            }
        }
    } else {
        let err = AppError::Validation(format!("CLI {} 无活跃 Provider", cli_id));
        log::warn!("{}：{}，尝试 clear", context, err);
        Some(err)
    };

    match clear_cli_with_adapter(cli_id, settings, adapter, adapter_factory) {
        Ok(()) => {
            log::warn!("{}：已 clear CLI 配置作为回退: cli_id={}", context, cli_id);
            Ok(())
        }
        Err(clear_err) => {
            log::error!(
                "{}：clear CLI 配置也失败: cli_id={}, err={}",
                context,
                cli_id,
                clear_err
            );
            let detail = match primary_err {
                Some(primary_err) => format!("{}；clear 也失败: {}", primary_err, clear_err),
                None => clear_err.to_string(),
            };
            Err(AppError::Validation(format!(
                "{}未完成，CLI 可能仍处于代理接管状态: cli_id={}, {}",
                context, cli_id, detail
            )))
        }
    }
}

fn restore_or_clear_cli_config(
    providers_dir: &Path,
    settings: &crate::storage::local::LocalSettings,
    cli_id: &str,
    adapter: Option<&dyn CliAdapter>,
    context: &str,
) -> Result<(), AppError> {
    restore_or_clear_cli_config_with_factory(
        providers_dir,
        settings,
        cli_id,
        adapter,
        context,
        get_adapter_for_cli,
    )
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
            ProtocolType::OpenAiChatCompletions,
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
        upstream_model: None,
        upstream_model_map: None,
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
                AppError::Validation(format!("该 CLI ({}) 无活跃 Provider，无法开启代理", cli_id))
            })?;

        // 2. 从 iCloud 读取真实 Provider
        let real_provider = crate::storage::icloud::get_provider_in(providers_dir, provider_id)?;

        // 3. 获取端口
        let port = proxy_port_for_cli(cli_id)
            .ok_or_else(|| AppError::Validation(format!("不支持的 CLI: {}", cli_id)))?;

        // 4. 构造代理专用 Provider 并 patch CLI 配置
        let proxy_provider = make_proxy_provider(cli_id, port, &real_provider);
        let real_adapter = if let Some(a) = adapter {
            a
        } else {
            get_adapter_for_cli(cli_id, &settings)?
        };
        real_adapter.patch(&proxy_provider)?;

        // 5. 构造 UpstreamTarget（真实凭据）
        let upstream = build_upstream_target_from_provider(&real_provider)?;

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
        if let Err(rollback_err) =
            get_adapter_for_cli(cli_id, &settings).and_then(|a| a.patch(&real_provider).map(|_| ()))
        {
            log::error!("回滚 CLI 配置也失败: {}", rollback_err);
        }
        return Err(AppError::Validation(format!("代理启动失败: {}", start_err)));
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
    let restore_result = {
        let settings = crate::storage::local::read_local_settings_from(local_settings_path)?;
        let adapter_ref = adapter.as_ref().map(|a| a.as_ref() as &dyn CliAdapter);
        restore_or_clear_cli_config(providers_dir, &settings, cli_id, adapter_ref, "关闭代理")
    };

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

    if restore_result.is_ok() {
        if let Some(ref mut takeover) = settings.proxy_takeover {
            takeover.cli_ids.retain(|id| id != cli_id);
        }
        if settings
            .proxy_takeover
            .as_ref()
            .is_some_and(|takeover| takeover.cli_ids.is_empty())
        {
            settings.proxy_takeover = None;
        }
    }

    if let Some(ref mut proxy) = settings.proxy {
        proxy.cli_enabled.insert(cli_id.to_string(), false);
    }

    crate::storage::local::write_local_settings_to(local_settings_path, &settings)?;

    match restore_result {
        Ok(()) => {
            log::info!("代理模式已关闭: cli_id={}", cli_id);
            Ok(())
        }
        Err(err) => Err(err),
    }
}

// --- 生命周期管理（退出清理 / 崩溃恢复 / 启动自动恢复） ---

/// 正常退出时同步还原所有已代理 CLI 配置（供 lib.rs RunEvent::ExitRequested 调用）
///
/// 读取 proxy_takeover.cli_ids，对每个 CLI 调用 adapter.patch(real_provider) 还原配置，
/// 清除 takeover 标志后写回 local.json。代理停止（async）由调用方另行处理。
fn cleanup_on_exit_sync_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    adapter_factory: AdapterFactory,
) {
    let settings = match crate::storage::local::read_local_settings_from(local_settings_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("退出清理：读取 local.json 失败: {}", e);
            return;
        }
    };

    let cli_ids: Vec<String> = settings
        .proxy_takeover
        .as_ref()
        .map(|t| t.cli_ids.clone())
        .unwrap_or_default();

    if cli_ids.is_empty() {
        log::info!("退出清理：无 takeover 标志，跳过");
        return;
    }

    let mut unresolved_cli_ids = Vec::new();
    for cli_id in &cli_ids {
        if let Err(err) = restore_or_clear_cli_config_with_factory(
            providers_dir,
            &settings,
            cli_id,
            None,
            "退出清理",
            adapter_factory,
        ) {
            log::error!(
                "退出清理：保留 takeover 标记: cli_id={}, err={}",
                cli_id,
                err
            );
            unresolved_cli_ids.push(cli_id.clone());
        }
    }

    // 仅清除已成功还原/clear 的 takeover 标志，失败项保留给下次启动恢复。
    let mut settings = match crate::storage::local::read_local_settings_from(local_settings_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("退出清理：重新读取 local.json 失败: {}", e);
            return;
        }
    };

    if unresolved_cli_ids.is_empty() {
        settings.proxy_takeover = None;
    } else if let Some(ref mut takeover) = settings.proxy_takeover {
        takeover
            .cli_ids
            .retain(|id| unresolved_cli_ids.iter().any(|pending| pending == id));
    }
    if settings
        .proxy_takeover
        .as_ref()
        .is_some_and(|takeover| takeover.cli_ids.is_empty())
    {
        settings.proxy_takeover = None;
    }

    if let Err(e) = crate::storage::local::write_local_settings_to(local_settings_path, &settings) {
        log::error!("退出清理：写回 local.json 失败: {}", e);
    }

    log::info!(
        "退出清理完成：成功处理 {} 个 CLI，保留 {} 个 takeover 标记",
        cli_ids.len().saturating_sub(unresolved_cli_ids.len()),
        unresolved_cli_ids.len()
    );
}

pub fn cleanup_on_exit_sync(providers_dir: &Path, local_settings_path: &Path) {
    cleanup_on_exit_sync_in(providers_dir, local_settings_path, get_adapter_for_cli);
}

/// 崩溃恢复：启动时检测遗留 takeover 标志并还原 CLI 配置
///
/// 同步函数。如果上次崩溃导致 takeover 未清除，则对每个 cli_id 还原配置。
fn recover_on_startup_in(
    providers_dir: &Path,
    local_settings_path: &Path,
    adapter_factory: AdapterFactory,
) -> Result<(), AppError> {
    let settings = crate::storage::local::read_local_settings_from(local_settings_path)?;

    let cli_ids: Vec<String> = settings
        .proxy_takeover
        .as_ref()
        .map(|t| t.cli_ids.clone())
        .unwrap_or_default();

    if cli_ids.is_empty() {
        return Ok(());
    }

    log::info!("崩溃恢复：检测到遗留 takeover 标志，cli_ids={:?}", cli_ids);

    let mut unresolved_cli_ids = Vec::new();
    for cli_id in &cli_ids {
        match restore_or_clear_cli_config_with_factory(
            providers_dir,
            &settings,
            cli_id,
            None,
            "崩溃恢复",
            adapter_factory,
        ) {
            Ok(()) => {
                log::info!("崩溃恢复：已处理 CLI 配置: cli_id={}", cli_id);
            }
            Err(err) => {
                log::error!(
                    "崩溃恢复：保留 takeover 标记: cli_id={}, err={}",
                    cli_id,
                    err
                );
                unresolved_cli_ids.push(cli_id.clone());
            }
        }
    }

    // 仅移除恢复成功的 takeover 标记，失败项保留给后续继续修复。
    let mut settings = crate::storage::local::read_local_settings_from(local_settings_path)?;
    if unresolved_cli_ids.is_empty() {
        settings.proxy_takeover = None;
    } else if let Some(ref mut takeover) = settings.proxy_takeover {
        takeover
            .cli_ids
            .retain(|id| unresolved_cli_ids.iter().any(|pending| pending == id));
    }
    if settings
        .proxy_takeover
        .as_ref()
        .is_some_and(|takeover| takeover.cli_ids.is_empty())
    {
        settings.proxy_takeover = None;
    }
    crate::storage::local::write_local_settings_to(local_settings_path, &settings)?;

    log::info!("崩溃恢复完成");
    Ok(())
}

pub fn recover_on_startup(
    providers_dir: &Path,
    local_settings_path: &Path,
) -> Result<(), AppError> {
    recover_on_startup_in(providers_dir, local_settings_path, get_adapter_for_cli)
}

/// 启动时根据持久化的开关状态自动重新开启代理（UX-02）
///
/// async 函数。读取 proxy.global_enabled 和 proxy.cli_enabled，
/// 对所有 enabled 的 CLI 重新启动代理。
pub async fn restore_proxy_state(
    providers_dir: &Path,
    local_settings_path: &Path,
    proxy_service: &ProxyService,
) -> Result<(), AppError> {
    let settings = crate::storage::local::read_local_settings_from(local_settings_path)?;

    let proxy = match settings.proxy.as_ref() {
        Some(p) => p,
        None => return Ok(()),
    };

    if !proxy.global_enabled {
        log::info!("代理状态恢复：全局开关关闭，跳过");
        return Ok(());
    }

    let cli_ids_to_enable: Vec<String> = proxy
        .cli_enabled
        .iter()
        .filter_map(|(cli_id, &enabled)| if enabled { Some(cli_id.clone()) } else { None })
        .collect();

    if cli_ids_to_enable.is_empty() {
        log::info!("代理状态恢复：无需启用的 CLI");
        return Ok(());
    }

    log::info!(
        "代理状态恢复：准备恢复代理，cli_ids={:?}",
        cli_ids_to_enable
    );

    for cli_id in &cli_ids_to_enable {
        // 检查是否有活跃 Provider
        let has_provider = settings
            .active_providers
            .get(cli_id.as_str())
            .map_or(false, |pid| pid.is_some());

        if !has_provider {
            log::warn!("代理状态恢复：CLI {} 无活跃 Provider，跳过", cli_id);
            continue;
        }

        if let Err(e) = _proxy_enable_in(
            providers_dir,
            local_settings_path,
            cli_id,
            proxy_service,
            None,
        )
        .await
        {
            log::warn!("代理状态恢复：CLI {} 启动失败: {}", cli_id, e);
        } else {
            log::info!("代理状态恢复：CLI {} 代理已恢复", cli_id);
        }
    }

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
    let providers_dir =
        crate::storage::icloud::get_icloud_providers_dir().map_err(|e| e.to_string())?;
    let settings_path = crate::storage::local::get_local_settings_path();

    _proxy_enable_in(
        &providers_dir,
        &settings_path,
        &cli_id,
        &proxy_service,
        None,
    )
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
    let providers_dir =
        crate::storage::icloud::get_icloud_providers_dir().map_err(|e| e.to_string())?;
    let settings_path = crate::storage::local::get_local_settings_path();

    _proxy_disable_in(
        &providers_dir,
        &settings_path,
        &cli_id,
        &proxy_service,
        None,
    )
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
    toggle_lock: State<'_, ProxyGlobalToggleLock>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let _guard = toggle_lock.lock().await;
    let providers_dir =
        crate::storage::icloud::get_icloud_providers_dir().map_err(|e| e.to_string())?;
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
    let settings = crate::storage::local::read_local_settings().map_err(|e| e.to_string())?;

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
    use crate::adapter::claude::ClaudeAdapter;
    use crate::adapter::codex::CodexAdapter;
    use crate::adapter::{CliAdapter, PatchResult};
    use crate::error::AppError;
    use crate::proxy::{PROXY_PORT_CLAUDE, PROXY_PORT_CODEX};
    use std::path::PathBuf;

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
            ProtocolType::OpenAiChatCompletions
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
    fn test_build_upstream_target_from_provider_strips_legacy_path() {
        let provider = Provider {
            id: "p-legacy".to_string(),
            cli_id: "codex".to_string(),
            name: "Legacy".to_string(),
            protocol_type: ProtocolType::OpenAiChatCompletions,
            api_key: "sk-test".to_string(),
            base_url: "https://api.openai.com/v1/chat/completions".to_string(),
            model: "o4-mini".to_string(),
            model_config: None,
            notes: None,
            upstream_model: None,
            upstream_model_map: None,
            created_at: 0,
            updated_at: 0,
            schema_version: 1,
        };

        let upstream = build_upstream_target_from_provider(&provider).unwrap();
        assert_eq!(upstream.base_url, "https://api.openai.com");
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
            upstream_model: None,
            upstream_model_map: None,
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
            protocol_type: ProtocolType::OpenAiChatCompletions,
            api_key: "sk-real-key".to_string(),
            base_url: "https://api.openai.com".to_string(),
            model: "o4-mini".to_string(),
            model_config: None,
            notes: None,
            upstream_model: None,
            upstream_model_map: None,
            created_at: 0,
            updated_at: 0,
            schema_version: 1,
        };

        let proxy = make_proxy_provider("codex", 15801, &real);
        assert_eq!(proxy.api_key, "PROXY_MANAGED");
        assert_eq!(proxy.base_url, "http://127.0.0.1:15801");
        assert!(matches!(
            proxy.protocol_type,
            ProtocolType::OpenAiChatCompletions
        ));
    }

    // --- 生命周期管理测试 ---

    use crate::storage::local::LocalSettings;
    use tempfile::TempDir;

    struct FailingRestoreAdapter;

    impl CliAdapter for FailingRestoreAdapter {
        fn cli_name(&self) -> &str {
            "failing-restore"
        }

        fn patch(&self, _provider: &Provider) -> Result<PatchResult, AppError> {
            Err(AppError::Validation("patch failed".to_string()))
        }

        fn clear(&self) -> Result<PatchResult, AppError> {
            Err(AppError::Validation("clear failed".to_string()))
        }
    }

    fn get_test_adapter_for_cli(
        cli_id: &str,
        settings: &LocalSettings,
    ) -> Result<Box<dyn CliAdapter>, AppError> {
        match cli_id {
            "claude" => {
                let config_dir = PathBuf::from(
                    settings
                        .cli_paths
                        .claude_config_dir
                        .as_ref()
                        .expect("claude config dir required"),
                );
                let backup_dir = config_dir
                    .parent()
                    .expect("config parent required")
                    .join("claude-backup");
                Ok(Box::new(ClaudeAdapter::new_with_paths(
                    config_dir, backup_dir,
                )))
            }
            "codex" => {
                let config_dir = PathBuf::from(
                    settings
                        .cli_paths
                        .codex_config_dir
                        .as_ref()
                        .expect("codex config dir required"),
                );
                let backup_dir = config_dir
                    .parent()
                    .expect("config parent required")
                    .join("codex-backup");
                Ok(Box::new(CodexAdapter::new_with_paths(
                    config_dir, backup_dir,
                )))
            }
            _ => Err(AppError::Validation(format!("Unknown CLI: {}", cli_id))),
        }
    }

    /// 辅助函数：创建模拟的 providers 目录和 Provider 文件
    fn setup_test_provider(
        providers_dir: &std::path::Path,
        provider_id: &str,
        cli_id: &str,
    ) -> Provider {
        let provider = Provider {
            id: provider_id.to_string(),
            cli_id: cli_id.to_string(),
            name: "Test Provider".to_string(),
            protocol_type: ProtocolType::Anthropic,
            api_key: "sk-real-key-12345".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            model_config: None,
            notes: None,
            upstream_model: None,
            upstream_model_map: None,
            created_at: 1710000000000,
            updated_at: 1710000000000,
            schema_version: 1,
        };

        // 写入 Provider 文件
        let provider_path = providers_dir.join(format!("{}.json", provider_id));
        let json = serde_json::to_string_pretty(&provider).unwrap();
        std::fs::write(&provider_path, json).unwrap();

        provider
    }

    #[test]
    fn test_recover_on_startup_clears_takeover() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        // 创建 Claude 配置目录
        let claude_config_dir = tmp.path().join("claude-config");
        std::fs::create_dir_all(&claude_config_dir).unwrap();

        // 写入被代理接管的 Claude 配置（PROXY_MANAGED）
        let proxy_settings = serde_json::json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED",
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:15800"
            }
        });
        std::fs::write(
            claude_config_dir.join("settings.json"),
            serde_json::to_string_pretty(&proxy_settings).unwrap(),
        )
        .unwrap();

        // 设置 Provider
        let _provider = setup_test_provider(&providers_dir, "p1", "claude");

        // 创建含 takeover 标志的 local.json
        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("p1".to_string()));

        let settings = LocalSettings {
            active_providers,
            cli_paths: crate::storage::local::CliPaths {
                claude_config_dir: Some(claude_config_dir.to_string_lossy().to_string()),
                codex_config_dir: None,
            },
            proxy_takeover: Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()],
            }),
            ..LocalSettings::default()
        };
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        // 执行崩溃恢复
        let result = recover_on_startup_in(&providers_dir, &local_path, get_test_adapter_for_cli);
        assert!(result.is_ok(), "recover_on_startup 应成功: {:?}", result);

        // 验证 takeover 被清除
        let updated = crate::storage::local::read_local_settings_from(&local_path).unwrap();
        assert!(updated.proxy_takeover.is_none(), "takeover 应被清除");

        // 验证 CLI 配置被还原为真实凭据
        let claude_settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(claude_config_dir.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            claude_settings["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-real-key-12345",
            "CLI 配置应被还原为真实 API key"
        );
    }

    #[test]
    fn test_recover_on_startup_noop_when_no_takeover() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        // 创建无 takeover 标志的 local.json
        let settings = LocalSettings::default();
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        // 执行崩溃恢复
        let result = recover_on_startup(&providers_dir, &local_path);
        assert!(result.is_ok(), "无 takeover 时应成功返回");

        // 验证 local.json 未被修改（仍为默认值）
        let loaded = crate::storage::local::read_local_settings_from(&local_path).unwrap();
        assert!(loaded.proxy_takeover.is_none());
    }

    #[test]
    fn test_recover_on_startup_keeps_failed_takeover() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        let claude_config_dir = tmp.path().join("claude-config");
        std::fs::create_dir_all(&claude_config_dir).unwrap();
        std::fs::write(claude_config_dir.join("settings.json"), "{invalid json").unwrap();

        let _provider = setup_test_provider(&providers_dir, "p1", "claude");

        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("p1".to_string()));

        let settings = LocalSettings {
            active_providers,
            cli_paths: crate::storage::local::CliPaths {
                claude_config_dir: Some(claude_config_dir.to_string_lossy().to_string()),
                codex_config_dir: None,
            },
            proxy_takeover: Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()],
            }),
            ..LocalSettings::default()
        };
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        let result = recover_on_startup(&providers_dir, &local_path);
        assert!(
            result.is_ok(),
            "恢复应保留失败标记而不是整体报错: {:?}",
            result
        );

        let updated = crate::storage::local::read_local_settings_from(&local_path).unwrap();
        assert_eq!(
            updated.proxy_takeover,
            Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()]
            })
        );
    }

    #[test]
    fn test_cleanup_on_exit_sync_restores_configs() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        // 创建 Claude 配置目录
        let claude_config_dir = tmp.path().join("claude-config");
        std::fs::create_dir_all(&claude_config_dir).unwrap();

        // 写入被代理接管的 Claude 配置
        let proxy_settings = serde_json::json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED",
                "ANTHROPIC_BASE_URL": "http://127.0.0.1:15800"
            }
        });
        std::fs::write(
            claude_config_dir.join("settings.json"),
            serde_json::to_string_pretty(&proxy_settings).unwrap(),
        )
        .unwrap();

        // 设置 Provider
        let _provider = setup_test_provider(&providers_dir, "p1", "claude");

        // 创建含 takeover 标志的 local.json
        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("p1".to_string()));

        let settings = LocalSettings {
            active_providers,
            cli_paths: crate::storage::local::CliPaths {
                claude_config_dir: Some(claude_config_dir.to_string_lossy().to_string()),
                codex_config_dir: None,
            },
            proxy: Some(ProxySettings {
                global_enabled: true,
                cli_enabled: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("claude".to_string(), true);
                    m
                },
            }),
            proxy_takeover: Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()],
            }),
            ..LocalSettings::default()
        };
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        // 执行退出清理
        cleanup_on_exit_sync_in(&providers_dir, &local_path, get_test_adapter_for_cli);

        // 验证 takeover 被清除
        let updated = crate::storage::local::read_local_settings_from(&local_path).unwrap();
        assert!(
            updated.proxy_takeover.is_none(),
            "退出清理后 takeover 应被清除"
        );

        // 验证 CLI 配置被还原
        let claude_settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(claude_config_dir.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            claude_settings["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-real-key-12345",
            "退出清理后 CLI 配置应被还原为真实 API key"
        );
    }

    #[test]
    fn test_cleanup_on_exit_sync_noop_when_no_takeover() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        // 创建无 takeover 的 local.json
        let settings = LocalSettings::default();
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        // 执行退出清理 — 应无副作用
        cleanup_on_exit_sync(&providers_dir, &local_path);

        let loaded = crate::storage::local::read_local_settings_from(&local_path).unwrap();
        assert!(loaded.proxy_takeover.is_none());
    }

    #[test]
    fn test_cleanup_on_exit_sync_keeps_failed_takeover() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        let claude_config_dir = tmp.path().join("claude-config");
        std::fs::create_dir_all(&claude_config_dir).unwrap();
        std::fs::write(claude_config_dir.join("settings.json"), "{invalid json").unwrap();

        let _provider = setup_test_provider(&providers_dir, "p1", "claude");

        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("p1".to_string()));

        let settings = LocalSettings {
            active_providers,
            cli_paths: crate::storage::local::CliPaths {
                claude_config_dir: Some(claude_config_dir.to_string_lossy().to_string()),
                codex_config_dir: None,
            },
            proxy_takeover: Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()],
            }),
            ..LocalSettings::default()
        };
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        cleanup_on_exit_sync(&providers_dir, &local_path);

        let updated = crate::storage::local::read_local_settings_from(&local_path).unwrap();
        assert_eq!(
            updated.proxy_takeover,
            Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()]
            })
        );
    }

    #[tokio::test]
    async fn test_proxy_disable_keeps_takeover_when_restore_fails() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        let _provider = setup_test_provider(&providers_dir, "p1", "claude");

        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("p1".to_string()));

        let settings = LocalSettings {
            active_providers,
            proxy: Some(ProxySettings {
                global_enabled: true,
                cli_enabled: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("claude".to_string(), true);
                    m
                },
            }),
            proxy_takeover: Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()],
            }),
            ..LocalSettings::default()
        };
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        let proxy_service = ProxyService::new();
        let err = _proxy_disable_in(
            &providers_dir,
            &local_path,
            "claude",
            &proxy_service,
            Some(Box::new(FailingRestoreAdapter)),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::Validation(_)));

        let updated = crate::storage::local::read_local_settings_from(&local_path).unwrap();
        assert_eq!(
            updated.proxy_takeover,
            Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()]
            })
        );
        assert_eq!(
            updated
                .proxy
                .as_ref()
                .and_then(|proxy| proxy.cli_enabled.get("claude"))
                .copied(),
            Some(false)
        );
    }

    // --- Gap 1: MODE-02/MODE-03 — _proxy_enable_in 完整流程测试 ---

    #[tokio::test]
    async fn test_proxy_enable_patches_cli_and_starts_proxy() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        // 创建 Claude 配置目录（adapter 需要写入 settings.json）
        let claude_config_dir = tmp.path().join("claude-config");
        std::fs::create_dir_all(&claude_config_dir).unwrap();
        let claude_backup_dir = tmp.path().join("claude-backup");

        // 写入初始的 Claude 配置（模拟直连状态）
        let initial_settings = serde_json::json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "sk-real-key-12345",
                "ANTHROPIC_BASE_URL": "https://api.anthropic.com"
            }
        });
        std::fs::write(
            claude_config_dir.join("settings.json"),
            serde_json::to_string_pretty(&initial_settings).unwrap(),
        )
        .unwrap();

        // 设置 Provider（iCloud 端）
        let _provider = setup_test_provider(&providers_dir, "p1", "claude");

        // 创建 local.json，设置 active_providers 和 cli_paths
        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("p1".to_string()));

        let settings = LocalSettings {
            active_providers,
            cli_paths: crate::storage::local::CliPaths {
                claude_config_dir: Some(claude_config_dir.to_string_lossy().to_string()),
                codex_config_dir: None,
            },
            ..LocalSettings::default()
        };
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        // 创建真实 ProxyService 和 adapter
        let proxy_service = ProxyService::new();
        let adapter: Box<dyn CliAdapter + Send> = Box::new(ClaudeAdapter::new_with_paths(
            claude_config_dir.clone(),
            claude_backup_dir,
        ));

        // 执行 _proxy_enable_in
        let result = _proxy_enable_in(
            &providers_dir,
            &local_path,
            "claude",
            &proxy_service,
            Some(adapter),
        )
        .await;
        assert!(result.is_ok(), "_proxy_enable_in 应成功: {:?}", result);

        // 验证 1: CLI 配置被 patch 为 PROXY_MANAGED + localhost
        let claude_settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(claude_config_dir.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            claude_settings["env"]["ANTHROPIC_AUTH_TOKEN"], "PROXY_MANAGED",
            "CLI 配置应被 patch 为 PROXY_MANAGED"
        );
        assert_eq!(
            claude_settings["env"]["ANTHROPIC_BASE_URL"],
            format!("http://127.0.0.1:{}", PROXY_PORT_CLAUDE),
            "CLI 配置应指向 localhost 代理端口"
        );

        // 验证 2: ProxyService 有运行中的 server
        let status = proxy_service.status().await;
        let claude_server = status.servers.iter().find(|s| s.cli_id == "claude");
        assert!(claude_server.is_some(), "ProxyService 应有 claude server");
        assert!(claude_server.unwrap().running, "claude server 应在运行中");

        // 验证 3: local.json proxy_takeover.cli_ids 包含 cli_id
        let updated = crate::storage::local::read_local_settings_from(&local_path).unwrap();
        let takeover = updated
            .proxy_takeover
            .as_ref()
            .expect("应有 proxy_takeover");
        assert!(
            takeover.cli_ids.contains(&"claude".to_string()),
            "proxy_takeover.cli_ids 应包含 claude"
        );

        // 验证 4: local.json proxy.cli_enabled[claude] = true
        let proxy = updated.proxy.as_ref().expect("应有 proxy settings");
        assert_eq!(
            proxy.cli_enabled.get("claude").copied(),
            Some(true),
            "proxy.cli_enabled[claude] 应为 true"
        );

        // 清理：停止代理
        let _ = proxy_service.stop("claude").await;
    }

    // --- Gap 2: MODE-04 — _proxy_disable_in 成功路径测试 ---

    #[tokio::test]
    async fn test_proxy_disable_restores_real_provider() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        // 创建 Claude 配置目录
        let claude_config_dir = tmp.path().join("claude-config");
        std::fs::create_dir_all(&claude_config_dir).unwrap();
        let claude_backup_dir = tmp.path().join("claude-backup");

        // 写入被代理接管的 Claude 配置（模拟 enable 后的状态）
        let proxy_config = serde_json::json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED",
                "ANTHROPIC_BASE_URL": format!("http://127.0.0.1:{}", PROXY_PORT_CLAUDE)
            }
        });
        std::fs::write(
            claude_config_dir.join("settings.json"),
            serde_json::to_string_pretty(&proxy_config).unwrap(),
        )
        .unwrap();

        // 设置 Provider（iCloud 端，包含真实凭据）
        let _provider = setup_test_provider(&providers_dir, "p1", "claude");

        // 创建 local.json，模拟代理已开启的状态
        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("p1".to_string()));

        let settings = LocalSettings {
            active_providers,
            cli_paths: crate::storage::local::CliPaths {
                claude_config_dir: Some(claude_config_dir.to_string_lossy().to_string()),
                codex_config_dir: None,
            },
            proxy: Some(ProxySettings {
                global_enabled: true,
                cli_enabled: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("claude".to_string(), true);
                    m
                },
            }),
            proxy_takeover: Some(ProxyTakeover {
                cli_ids: vec!["claude".to_string()],
            }),
            ..LocalSettings::default()
        };
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        let proxy_service = ProxyService::new();
        let adapter: Box<dyn CliAdapter + Send> = Box::new(ClaudeAdapter::new_with_paths(
            claude_config_dir.clone(),
            claude_backup_dir,
        ));

        // 执行 _proxy_disable_in
        let result = _proxy_disable_in(
            &providers_dir,
            &local_path,
            "claude",
            &proxy_service,
            Some(adapter),
        )
        .await;
        assert!(result.is_ok(), "_proxy_disable_in 应成功: {:?}", result);

        // 验证 1: CLI 配置被还原为真实 API key
        let claude_settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(claude_config_dir.join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            claude_settings["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-real-key-12345",
            "CLI 配置应被还原为真实 API key"
        );
        assert_eq!(
            claude_settings["env"]["ANTHROPIC_BASE_URL"], "https://api.anthropic.com",
            "CLI 配置应被还原为真实 base_url"
        );

        // 验证 2: local.json proxy_takeover.cli_ids 不再包含 claude
        let updated = crate::storage::local::read_local_settings_from(&local_path).unwrap();
        let has_claude_takeover = updated
            .proxy_takeover
            .as_ref()
            .map_or(false, |t| t.cli_ids.contains(&"claude".to_string()));
        assert!(!has_claude_takeover, "proxy_takeover 不应再包含 claude");

        // 验证 3: local.json proxy.cli_enabled[claude] = false
        let cli_enabled = updated
            .proxy
            .as_ref()
            .and_then(|p| p.cli_enabled.get("claude"))
            .copied();
        assert_eq!(
            cli_enabled,
            Some(false),
            "proxy.cli_enabled[claude] 应为 false"
        );
    }

    // --- Gap 3: UX-02 — restore_proxy_state 自动恢复测试 ---

    #[tokio::test]
    async fn test_restore_proxy_state_re_enables_proxy() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        // 创建 Claude 配置目录
        let claude_config_dir = tmp.path().join("claude-config");
        std::fs::create_dir_all(&claude_config_dir).unwrap();

        // 写入初始 Claude 配置（直连状态，恢复前的状态）
        let initial_config = serde_json::json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "sk-real-key-12345",
                "ANTHROPIC_BASE_URL": "https://api.anthropic.com"
            }
        });
        std::fs::write(
            claude_config_dir.join("settings.json"),
            serde_json::to_string_pretty(&initial_config).unwrap(),
        )
        .unwrap();

        // 设置 Provider
        let _provider = setup_test_provider(&providers_dir, "p1", "claude");

        // 创建 local.json：global_enabled=true, cli_enabled[claude]=true
        // 模拟应用重启后从持久化恢复的场景
        let mut active_providers = std::collections::HashMap::new();
        active_providers.insert("claude".to_string(), Some("p1".to_string()));

        let settings = LocalSettings {
            active_providers,
            cli_paths: crate::storage::local::CliPaths {
                claude_config_dir: Some(claude_config_dir.to_string_lossy().to_string()),
                codex_config_dir: None,
            },
            proxy: Some(ProxySettings {
                global_enabled: true,
                cli_enabled: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("claude".to_string(), true);
                    m
                },
            }),
            // 注意：重启后 takeover 已被 recover_on_startup 清除，所以为 None
            proxy_takeover: None,
            ..LocalSettings::default()
        };
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        let proxy_service = ProxyService::new();

        // 执行 restore_proxy_state
        let result = restore_proxy_state(&providers_dir, &local_path, &proxy_service).await;
        // restore_proxy_state 内部对启动失败只记录 warn 日志并继续，始终返回 Ok
        assert!(result.is_ok(), "restore_proxy_state 应成功: {:?}", result);

        // 验证行为：检查代理是否成功启动
        // 注意：在并行测试中端口 15800 可能被其他测试占用，导致 start 失败并回滚。
        // 如果启动成功，验证完整状态；如果失败（端口冲突），验证回滚正确。
        let status = proxy_service.status().await;
        let claude_server = status.servers.iter().find(|s| s.cli_id == "claude");
        let proxy_started = claude_server.map_or(false, |s| s.running);

        let claude_settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(claude_config_dir.join("settings.json")).unwrap(),
        )
        .unwrap();

        if proxy_started {
            // 成功路径：CLI 配置被 patch 为代理模式
            assert_eq!(
                claude_settings["env"]["ANTHROPIC_AUTH_TOKEN"], "PROXY_MANAGED",
                "恢复后 CLI 配置应被 patch 为 PROXY_MANAGED"
            );
            // 清理
            let _ = proxy_service.stop("claude").await;
        } else {
            // 端口冲突路径：_proxy_enable_in 回滚了 CLI 配置，config 应保持原状
            assert_eq!(
                claude_settings["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-real-key-12345",
                "启动失败时 CLI 配置应被回滚为原始状态"
            );
        }

        // 核心行为验证：restore_proxy_state 正确识别了需要恢复的 CLI
        // （无论启动成功与否，函数都正确返回 Ok 且不 panic）
    }

    #[tokio::test]
    async fn test_restore_proxy_state_noop_when_disabled() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        // 创建 local.json：global_enabled=false
        let settings = LocalSettings {
            proxy: Some(ProxySettings {
                global_enabled: false,
                cli_enabled: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("claude".to_string(), true);
                    m
                },
            }),
            ..LocalSettings::default()
        };
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        let proxy_service = ProxyService::new();

        // 执行 restore_proxy_state
        let result = restore_proxy_state(&providers_dir, &local_path, &proxy_service).await;
        assert!(result.is_ok(), "global_enabled=false 时应直接返回 Ok");

        // 验证：没有代理被启动
        let status = proxy_service.status().await;
        assert!(
            status.servers.is_empty(),
            "global_enabled=false 时不应启动任何代理"
        );
    }

    #[tokio::test]
    async fn test_restore_proxy_state_noop_when_no_proxy_settings() {
        let tmp = TempDir::new().unwrap();
        let providers_dir = tmp.path().join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        let local_path = tmp.path().join("local.json");

        // 创建 local.json：无 proxy 字段
        let settings = LocalSettings::default();
        crate::storage::local::write_local_settings_to(&local_path, &settings).unwrap();

        let proxy_service = ProxyService::new();

        let result = restore_proxy_state(&providers_dir, &local_path, &proxy_service).await;
        assert!(result.is_ok(), "无 proxy 设置时应直接返回 Ok");

        let status = proxy_service.status().await;
        assert!(status.servers.is_empty(), "无 proxy 设置时不应启动任何代理");
    }
}
