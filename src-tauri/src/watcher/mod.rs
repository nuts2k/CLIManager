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
    let repatched = match crate::commands::provider::sync_changed_active_providers(&changed_files) {
        Ok(repatched) => repatched,
        Err(e) => {
            log::error!("Failed to re-patch CLI configs after sync: {:?}", e);
            let _ = app_handle.emit("sync-repatch-failed", e.to_string());
            false
        }
    };

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
