mod self_write;

pub use self_write::SelfWriteTracker;

use std::collections::HashSet;
use std::path::PathBuf;

use notify_debouncer_mini::DebouncedEvent;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, serde::Serialize)]
pub struct ProvidersChangedPayload {
    pub changed_files: Vec<String>,
    pub repatched: bool,
}

/// Start the file watcher on the iCloud providers directory.
/// This watches for .json file changes and emits Tauri events.
pub fn start_file_watcher(app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let providers_dir = crate::storage::icloud::get_icloud_providers_dir()?;

    let handle = app_handle.clone();
    let watch_path = providers_dir.clone();

    let mut debouncer = notify_debouncer_mini::new_debouncer(
        std::time::Duration::from_millis(500),
        move |result: Result<Vec<DebouncedEvent>, notify::Error>| match result {
            Ok(events) => process_events(events, &handle),
            Err(e) => log::error!("File watcher error: {:?}", e),
        },
    )?;

    // Watch directory (non-recursive, providers are flat files)
    debouncer
        .watcher()
        .watch(&watch_path, notify::RecursiveMode::NonRecursive)?;

    // Keep the debouncer alive for the app lifetime
    std::mem::forget(debouncer);

    log::info!("File watcher started on {:?}", providers_dir);
    Ok(())
}

/// Process debounced file events: filter, deduplicate, and emit Tauri event.
fn process_events(events: Vec<DebouncedEvent>, app_handle: &AppHandle) {
    let tracker = app_handle.state::<SelfWriteTracker>();
    let changed_files = filter_and_dedup_events(&events, |path| tracker.is_self_write(path));

    if changed_files.is_empty() {
        return;
    }

    // Auto re-patch CLI configs
    let (repatched, proxy_cli_ids_to_disable) =
        match crate::commands::provider::sync_changed_active_providers(&changed_files) {
            Ok(result) => (result.repatched, result.proxy_cli_ids_to_disable),
            Err(e) => {
                log::error!("Failed to re-patch CLI configs after sync: {:?}", e);
                let _ = app_handle.emit("sync-repatch-failed", e.to_string());
                (false, vec![])
            }
        };

    // 代理模式下活跃 Provider 被同步删除：异步关闭代理并重新 reconcile
    if !proxy_cli_ids_to_disable.is_empty() {
        let handle_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            crate::commands::provider::disable_proxy_for_deleted_providers(
                &handle_clone,
                proxy_cli_ids_to_disable,
            )
            .await;
        });
    }

    // 代理模式联动：iCloud 同步变更活跃 Provider 时，自动更新代理上游
    update_proxy_upstream_if_needed(app_handle, &changed_files);

    let payload = ProvidersChangedPayload {
        changed_files,
        repatched,
    };

    if let Err(e) = app_handle.emit("providers-changed", &payload) {
        log::error!("Failed to emit providers-changed event: {:?}", e);
    }

    // Rebuild tray menu to reflect provider changes from iCloud sync
    #[cfg(desktop)]
    crate::tray::update_tray_menu(app_handle);
}

/// 代理模式联动：检查变更的文件是否对应代理模式下的活跃 Provider，
/// 如果是则通过 spawn async 更新代理上游。
///
/// process_events 在 notify 回调中执行，是同步上下文，不能直接 `.await`，
/// 因此使用 `tauri::async_runtime::spawn` 提交异步操作。
fn update_proxy_upstream_if_needed(app_handle: &AppHandle, changed_files: &[String]) {
    let settings_path = crate::storage::local::get_local_settings_path();
    let settings = match crate::storage::local::read_local_settings_from(&settings_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("代理联动：读取 local settings 失败: {}", e);
            return;
        }
    };

    // 检查代理全局开关和 takeover 状态
    let global_enabled = settings
        .proxy
        .as_ref()
        .map_or(false, |p| p.global_enabled);
    if !global_enabled {
        return;
    }

    let cli_ids = match settings.proxy_takeover {
        Some(ref t) if !t.cli_ids.is_empty() => &t.cli_ids,
        _ => return,
    };

    let providers_dir = match crate::storage::icloud::get_icloud_providers_dir() {
        Ok(d) => d,
        Err(e) => {
            log::error!("代理联动：获取 providers 目录失败: {}", e);
            return;
        }
    };

    for cli_id in cli_ids {
        // 获取该 CLI 的活跃 Provider ID
        let active_provider_id = match settings.active_providers.get(cli_id) {
            Some(Some(pid)) => pid,
            _ => continue,
        };

        // 检查该 Provider ID 是否在 changed_files 中（changed_files 元素是文件 stem，即 provider id）
        if !changed_files.iter().any(|f| f == active_provider_id) {
            continue;
        }

        // 从 iCloud 重新读取该 Provider，构造 UpstreamTarget
        let provider = match crate::storage::icloud::get_provider_in(&providers_dir, active_provider_id) {
            Ok(p) => p,
            Err(e) => {
                log::error!(
                    "代理联动：读取 Provider 失败: cli_id={}, provider_id={}, err={}",
                    cli_id,
                    active_provider_id,
                    e
                );
                continue;
            }
        };

        let base_url = match crate::provider::extract_origin_base_url(&provider.base_url) {
            Ok(url) => url,
            Err(e) => {
                log::error!(
                    "代理联动：解析 base_url 失败: cli_id={}, err={}",
                    cli_id,
                    e
                );
                continue;
            }
        };

        let upstream = crate::proxy::UpstreamTarget {
            api_key: provider.api_key.clone(),
            base_url,
            protocol_type: provider.protocol_type.clone(),
        };

        let cli_id_owned = cli_id.clone();
        let handle_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let proxy_service = handle_clone.state::<crate::proxy::ProxyService>();
            if let Err(e) = proxy_service.update_upstream(&cli_id_owned, upstream).await {
                log::error!(
                    "代理联动：更新上游失败: cli_id={}, err={}",
                    cli_id_owned,
                    e
                );
            } else {
                log::info!("代理联动：iCloud 同步后已更新上游: cli_id={}", cli_id_owned);
            }
        });
    }
}

/// Pure function: filter events to .json files, exclude self-writes, deduplicate by file stem.
/// The `is_self_write` parameter is a closure for testability.
fn filter_and_dedup_events<F>(events: &[DebouncedEvent], is_self_write: F) -> Vec<String>
where
    F: Fn(&PathBuf) -> bool,
{
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for event in events {
        let path = &event.path;

        // Only .json files
        let is_json = path.extension().is_some_and(|ext| ext == "json");

        if !is_json {
            continue;
        }

        // Skip self-writes
        if is_self_write(path) {
            continue;
        }

        // Deduplicate by file stem
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if seen.insert(stem.to_string()) {
                result.push(stem.to_string());
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify_debouncer_mini::DebouncedEvent;
    use std::path::PathBuf;

    fn make_event(path: &str) -> DebouncedEvent {
        DebouncedEvent {
            path: PathBuf::from(path),
            kind: notify_debouncer_mini::DebouncedEventKind::Any,
        }
    }

    #[test]
    fn test_filter_only_json_files() {
        let events = vec![
            make_event("/providers/abc.json"),
            make_event("/providers/.abc.icloud"),
            make_event("/providers/abc.tmp"),
            make_event("/providers/def.json"),
        ];

        let result = filter_and_dedup_events(&events, |_| false);
        assert_eq!(result, vec!["abc", "def"]);
    }

    #[test]
    fn test_filter_excludes_self_writes() {
        let self_written = PathBuf::from("/providers/abc.json");
        let events = vec![
            make_event("/providers/abc.json"),
            make_event("/providers/def.json"),
        ];

        let result = filter_and_dedup_events(&events, |path| *path == self_written);
        assert_eq!(result, vec!["def"]);
    }

    #[test]
    fn test_dedup_same_file_stem() {
        let events = vec![
            make_event("/providers/abc.json"),
            make_event("/providers/abc.json"),
            make_event("/providers/abc.json"),
        ];

        let result = filter_and_dedup_events(&events, |_| false);
        assert_eq!(result, vec!["abc"]);
    }

    #[test]
    fn test_empty_events() {
        let events: Vec<DebouncedEvent> = vec![];
        let result = filter_and_dedup_events(&events, |_| false);
        assert!(result.is_empty());
    }

    #[test]
    fn test_all_filtered_out() {
        let events = vec![
            make_event("/providers/abc.icloud"),
            make_event("/providers/.DS_Store"),
        ];

        let result = filter_and_dedup_events(&events, |_| false);
        assert!(result.is_empty());
    }

    #[test]
    fn test_mixed_filtering_and_dedup() {
        let self_written = PathBuf::from("/providers/self.json");
        let events = vec![
            make_event("/providers/abc.json"),
            make_event("/providers/self.json"),  // self-write
            make_event("/providers/abc.json"),   // duplicate
            make_event("/providers/def.icloud"), // non-json
            make_event("/providers/ghi.json"),
        ];

        let result = filter_and_dedup_events(&events, |path| *path == self_written);
        assert_eq!(result, vec!["abc", "ghi"]);
    }
}
