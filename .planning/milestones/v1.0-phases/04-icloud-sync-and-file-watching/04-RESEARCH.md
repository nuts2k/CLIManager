# Phase 4: iCloud Sync and File Watching - Research

**Researched:** 2026-03-11
**Domain:** FSEvents file watching, Tauri event system, debounce patterns
**Confidence:** HIGH

## Summary

Phase 4 adds reactive file watching to the existing provider management system. The `notify` crate (v8.x) is the de facto standard for cross-platform file watching in Rust, with built-in FSEvents support on macOS. Combined with `notify-debouncer-mini`, it handles the 500ms debounce requirement directly. The Tauri v2 event system (`app.emit()` / `listen()`) provides the backend-to-frontend communication channel. The existing `sync_active_providers` command already handles re-patching, and `useProviders.refresh()` / `useSettings.refresh()` already support programmatic reload.

The architecture is straightforward: a file watcher thread started in the Tauri `setup()` hook monitors the iCloud providers directory, debounces events, filters for `.json` files, skips self-writes, and emits a `providers-changed` event to the frontend. The frontend listens for this event, refreshes data, and shows toast notifications.

**Primary recommendation:** Use `notify` v8.x with `notify-debouncer-mini` v0.7.x for file watching. Use Tauri's `setup()` hook to initialize the watcher with a cloned `AppHandle`. Implement self-write tracking with a `Mutex<HashMap<PathBuf, Instant>>` managed as Tauri state.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Self-write detection: Track writes per file path with 1-second ignore window after each write. Record file path + timestamp on write, skip FSEvents within 1 second for that file.
- UI refresh behavior: Background refresh + toast notification showing Provider name (e.g., "已同步：Provider 'My API' 已更新"). Toast auto-dismisses after 2-3 seconds. Additional toast when active Provider is modified and CLI config re-patched.
- iCloud file eviction: v1 does NOT handle file eviction. Skip gracefully on IO error, log error, continue processing other files.
- Event debounce: 500ms window. Collect all events, process as batch. Merge multiple file changes into single refresh. Filter out non-.json files.
- Backend-to-frontend: Tauri event system with `app.emit()` for 'providers-changed' event. Payload includes changed file names. Frontend listens with `listen()` and triggers full provider list reload. No polling.
- Startup sync: Read all providers on startup (existing behavior). Call `sync_active_providers` on startup to re-patch all active providers.
- File watcher lifecycle: Start on window open, stop on window close, restart on re-open.
- Re-patch failure: Show error toast, no automatic retry, log error.

### Claude's Discretion
- Specific FSEvents/notify crate configuration details
- Debounce implementation approach (timer-based, channel-based, etc.)
- Toast component library choice or implementation
- Exact event payload structure
- Thread/async architecture for the watcher

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SYNC-03 | File watcher (FSEvents) monitors iCloud sync directory for Provider file changes | `notify` v8.x with `macos_fsevent` feature provides FSEvents backend; `notify-debouncer-mini` handles 500ms debounce; watcher initialized in Tauri `setup()` hook |
| SYNC-04 | UI automatically refreshes when Provider files are added, modified, or deleted via sync | Tauri `app.emit("providers-changed", payload)` from Rust + `listen("providers-changed")` in frontend triggers `refresh()` on existing hooks |
| SYNC-05 | When active Provider is modified by sync, CLI configs are automatically re-patched with updated values | Existing `sync_active_providers` command handles re-patching; watcher calls it when detecting changes to active provider files |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| notify | 8.x | Cross-platform file watching (FSEvents on macOS) | Used by rust-analyzer, deno, cargo-watch, watchexec; de facto standard |
| notify-debouncer-mini | 0.7.x | Debounced file watching (one event per file per timeframe) | Official companion crate; handles 500ms debounce natively |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| @tauri-apps/api (event module) | ^2 | Frontend event listening | Already installed; import `listen` from `@tauri-apps/api/event` |
| sonner | ^2.0.7 | Toast notifications | Already installed and configured with `<Toaster>` in App.tsx |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| notify-debouncer-mini | notify + manual debounce | More code, more bugs; debouncer-mini is < 200 lines and well-tested |
| notify-debouncer-mini | notify-debouncer-full | Full tracks file IDs and renames; overkill for this use case |
| FSEvents via notify | Raw fsevent-sys | No cross-platform fallback, more unsafe code |

**Installation (Rust):**
```toml
# src-tauri/Cargo.toml [dependencies]
notify = { version = "8", features = ["macos_fsevent"] }
notify-debouncer-mini = { version = "0.7", features = ["macos_fsevent"] }
```

**No new frontend packages needed** -- `@tauri-apps/api` and `sonner` already installed.

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
├── watcher/
│   ├── mod.rs          # File watcher module: init, start, stop
│   └── self_write.rs   # Self-write tracking (Mutex<HashMap<PathBuf, Instant>>)
├── lib.rs              # Updated: setup() hook, manage state
├── storage/
│   └── icloud.rs       # Existing: get_icloud_providers_dir()
└── commands/
    └── provider.rs     # Existing: sync_active_providers (reused by watcher)

src/
├── hooks/
│   ├── useProviders.ts # Existing: refresh() wired to event listener
│   └── useSettings.ts  # Existing: refresh() wired to event listener
├── components/
│   └── layout/
│       └── AppShell.tsx # Updated: add Tauri event listener
└── lib/
    └── tauri.ts        # Existing: syncActiveProviders (reused)
```

### Pattern 1: Watcher Initialization in setup() Hook
**What:** Start the file watcher in Tauri's `setup()` hook, which runs once when the app initializes.
**When to use:** Always -- this is the standard Tauri pattern for background services.
**Example:**
```rust
// Source: Tauri v2 docs + notify docs
use tauri::{Manager, Emitter};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use std::sync::Mutex;
use std::time::Duration;

pub fn run() {
    tauri::Builder::default()
        .manage(SelfWriteTracker::new()) // Tauri state
        .setup(|app| {
            let handle = app.handle().clone();
            start_file_watcher(handle)?;
            Ok(())
        })
        .invoke_handler(/* ... */)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Pattern 2: Self-Write Tracking via Tauri Managed State
**What:** A `Mutex<HashMap<PathBuf, Instant>>` stored as Tauri managed state, recording recent writes.
**When to use:** Every write to the providers directory must register the file path; every incoming FSEvent must check against it.
**Example:**
```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct SelfWriteTracker {
    writes: Mutex<HashMap<PathBuf, Instant>>,
}

impl SelfWriteTracker {
    pub fn new() -> Self {
        Self { writes: Mutex::new(HashMap::new()) }
    }

    pub fn record_write(&self, path: PathBuf) {
        self.writes.lock().unwrap().insert(path, Instant::now());
    }

    pub fn is_self_write(&self, path: &PathBuf) -> bool {
        let mut writes = self.writes.lock().unwrap();
        if let Some(ts) = writes.get(path) {
            if ts.elapsed() < Duration::from_secs(1) {
                return true;
            }
            writes.remove(path); // Clean up expired entries
        }
        false
    }
}
```

### Pattern 3: Event Emission from Watcher Thread
**What:** The watcher callback filters events, checks self-write, then emits a Tauri event.
**When to use:** On every debounced FSEvent batch.
**Example:**
```rust
use tauri::Emitter;
use serde::Serialize;

#[derive(Clone, Serialize)]
struct ProvidersChangedPayload {
    changed_files: Vec<String>,
}

// Inside the debouncer callback:
fn handle_events(
    events: Vec<DebouncedEvent>,
    app_handle: &AppHandle,
    tracker: &SelfWriteTracker,
) {
    let changed: Vec<String> = events
        .into_iter()
        .filter(|e| {
            e.path.extension().map_or(false, |ext| ext == "json")
                && !tracker.is_self_write(&e.path)
        })
        .filter_map(|e| {
            e.path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
        })
        .collect();

    if !changed.is_empty() {
        let _ = app_handle.emit("providers-changed", ProvidersChangedPayload {
            changed_files: changed,
        });
    }
}
```

### Pattern 4: Frontend Event Listener in AppShell
**What:** Root-level effect subscribes to `providers-changed` event.
**When to use:** In AppShell or a dedicated hook, ensuring cleanup on unmount.
**Example:**
```typescript
// Source: Tauri v2 event API docs
import { listen } from "@tauri-apps/api/event";

useEffect(() => {
    const unlisten = listen<{ changed_files: string[] }>(
        "providers-changed",
        async (event) => {
            // Refresh provider data
            await refresh();
            // Show toast with changed file info
            toast.info(`已同步：${event.payload.changed_files.length} 个 Provider 已更新`);
        }
    );
    return () => { unlisten.then(fn => fn()); };
}, []);
```

### Anti-Patterns to Avoid
- **Polling instead of events:** Never use `setInterval` to poll for provider changes. The FSEvents + Tauri event architecture is purely reactive.
- **Watcher in a Tauri command:** Don't start the watcher from a frontend-invoked command. Use the `setup()` hook for app-lifecycle services.
- **Sharing watcher across threads without proper sync:** The `notify` watcher itself is not Send+Sync friendly for stopping. Store the watcher handle in the setup closure or a managed state with proper locking.
- **Processing .icloud placeholder files:** Always filter to `.json` extension only. iCloud creates `.filename.icloud` placeholder files that must be ignored.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| File system event debouncing | Custom timer/channel debounce | `notify-debouncer-mini` | Handles edge cases: rapid events, OS-specific batching, thread safety |
| Cross-platform file watching | Manual FSEvents/inotify bindings | `notify` crate | FSEvents API is C-based and tricky; notify abstracts platform differences |
| Backend-to-frontend events | Custom WebSocket or polling | Tauri `app.emit()` + `listen()` | Built into Tauri, zero config, type-safe payloads |
| Toast notifications | Custom notification component | `sonner` (already installed) | Already configured with Toaster in App.tsx |
| CLI config re-patching | Custom re-patch logic | Existing `sync_active_providers` command | Already handles iterating active providers and patching each CLI adapter |

**Key insight:** This phase is primarily integration work. All the building blocks exist (provider CRUD, sync command, hooks with refresh, toast system). The new code is the watcher module and the event wiring.

## Common Pitfalls

### Pitfall 1: Infinite Loop from Self-Writes
**What goes wrong:** App writes a provider file -> FSEvents fires -> watcher triggers re-patch -> re-patch writes config -> ... (loop if config is in watched directory, which it's not, but provider writes can loop)
**Why it happens:** atomic_write() creates a temp file then renames, generating multiple FSEvents for a single logical write.
**How to avoid:** Self-write tracker with 1-second ignore window. Record the final path (not the temp path) since `fs::rename` generates an event on the destination path.
**Warning signs:** Console shows repeated "providers-changed" events after a single manual edit.

### Pitfall 2: iCloud Event Storms
**What goes wrong:** iCloud sync can deliver dozens of events in rapid succession when syncing multiple files from another device.
**Why it happens:** iCloud syncs files individually, each generating FSEvents as it downloads and writes.
**How to avoid:** 500ms debounce window merges all events into a single batch. The debouncer-mini handles this natively.
**Warning signs:** Multiple toasts appearing in rapid succession.

### Pitfall 3: Reading Partially-Written Files
**What goes wrong:** FSEvent fires mid-write, app reads incomplete JSON, deserialization fails.
**Why it happens:** iCloud writes may not be atomic from the watcher's perspective.
**How to avoid:** Wrap file reads in try/catch. On deserialization failure, skip the file and log the error (matches the "skip gracefully on IO error" decision). The next debounced event will likely see the complete file.
**Warning signs:** Intermittent JSON parse errors in logs during sync.

### Pitfall 4: Watcher Not Watching After iCloud Directory Creation
**What goes wrong:** If the iCloud providers directory doesn't exist at startup, `get_icloud_providers_dir()` creates it, but the watcher may have been set up before the directory exists.
**Why it happens:** Race condition between directory resolution and watcher initialization.
**How to avoid:** Call `get_icloud_providers_dir()` first to ensure the directory exists, then set up the watcher on the returned path.
**Warning signs:** No events detected despite file changes in the iCloud directory.

### Pitfall 5: Forgetting to Unlisten on Frontend
**What goes wrong:** Memory leak or duplicate event handlers if component remounts without cleanup.
**Why it happens:** Tauri's `listen()` returns a promise of an unlisten function; easy to forget cleanup.
**How to avoid:** Always return cleanup from `useEffect`: `return () => { unlisten.then(fn => fn()); };`
**Warning signs:** Duplicate toast notifications, multiplying on each navigation.

### Pitfall 6: atomic_write Temp Files Triggering Events
**What goes wrong:** The `.{filename}.tmp` file created by `atomic_write()` triggers an FSEvent before the rename.
**Why it happens:** FSEvents reports all filesystem operations including temp file creation.
**How to avoid:** Filter events to only process `.json` extension files. Temp files use `.tmp` extension and dot-prefix, so they're naturally excluded.
**Warning signs:** Events for files with `.tmp` extension appearing in logs.

## Code Examples

### Complete Watcher Module Setup
```rust
// src-tauri/src/watcher/mod.rs
// Source: notify 8.x docs + Tauri v2 setup pattern

use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEvent};
use notify::RecursiveMode;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

mod self_write;
pub use self_write::SelfWriteTracker;

#[derive(Clone, serde::Serialize)]
pub struct ProvidersChangedPayload {
    pub changed_files: Vec<String>,
}

pub fn start_file_watcher(app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let providers_dir = crate::storage::icloud::get_icloud_providers_dir()?;
    let handle = app_handle.clone();

    let mut debouncer = new_debouncer(
        Duration::from_millis(500),
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => process_events(events, &handle),
                Err(errors) => {
                    for e in errors {
                        log::error!("File watcher error: {:?}", e);
                    }
                }
            }
        },
    )?;

    debouncer.watcher().watch(&providers_dir, RecursiveMode::NonRecursive)?;

    // Keep the debouncer alive by leaking it (or store in managed state)
    // The watcher needs to live for the app lifetime
    std::mem::forget(debouncer);

    Ok(())
}

fn process_events(events: Vec<DebouncedEvent>, app_handle: &AppHandle) {
    let tracker = app_handle.state::<SelfWriteTracker>();

    let changed_files: Vec<String> = events
        .into_iter()
        .filter(|e| {
            e.path.extension().map_or(false, |ext| ext == "json")
                && !tracker.is_self_write(&e.path)
        })
        .filter_map(|e| {
            e.path.file_stem().map(|s| s.to_string_lossy().to_string())
        })
        .collect::<std::collections::HashSet<_>>() // deduplicate
        .into_iter()
        .collect();

    if changed_files.is_empty() {
        return;
    }

    // Trigger re-patch for active providers
    if let Err(e) = crate::commands::provider::sync_active_providers() {
        log::error!("Auto re-patch failed after sync: {:?}", e);
        let _ = app_handle.emit("sync-repatch-failed", e.to_string());
    }

    let _ = app_handle.emit("providers-changed", ProvidersChangedPayload { changed_files });
}
```

### Integrating Self-Write Tracking with Existing Writes
```rust
// In storage/icloud.rs or a wrapper, call tracker.record_write()
// after every successful provider file write.
// The tracker is accessed via AppHandle state in commands,
// or passed directly to storage functions.

// Option: Add a record_write call in the command layer (commands/provider.rs)
// after save_provider / save_existing_provider / delete_provider calls.
```

### Frontend Event Listener Hook
```typescript
// src/hooks/useSyncListener.ts
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { syncActiveProviders } from "@/lib/tauri";

interface ProvidersChangedPayload {
  changed_files: string[];
}

export function useSyncListener(
  refreshProviders: () => Promise<void>,
  refreshSettings: () => Promise<void>,
) {
  const { t } = useTranslation();

  useEffect(() => {
    const unlistenProviders = listen<ProvidersChangedPayload>(
      "providers-changed",
      async (event) => {
        await refreshProviders();
        await refreshSettings();
        const count = event.payload.changed_files.length;
        toast.info(
          t("sync.providersUpdated", { count }),
          { duration: 3000 }
        );
      }
    );

    const unlistenRepatchFail = listen<string>(
      "sync-repatch-failed",
      (event) => {
        toast.error(
          t("sync.repatchFailed"),
          { duration: 5000 }
        );
        console.error("Sync re-patch failed:", event.payload);
      }
    );

    return () => {
      unlistenProviders.then((fn) => fn());
      unlistenRepatchFail.then((fn) => fn());
    };
  }, [refreshProviders, refreshSettings, t]);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| notify v4-v6 with built-in debounce | notify v8 + separate debouncer crates | notify 5.0 (2022) | Debouncing is now modular via `notify-debouncer-mini` or `notify-debouncer-full` |
| Tauri v1 `app.emit_all()` | Tauri v2 `app.emit()` via `Emitter` trait | Tauri 2.0 (2024) | Must `use tauri::Emitter;` -- method moved to trait |
| Manual FSEvents bindings | notify with `macos_fsevent` feature | Long-standing | notify handles the C FFI internally |

**Deprecated/outdated:**
- `notify` v4/v5 built-in `watcher()` with debounce config: Removed; use companion debouncer crates instead
- `Tauri::emit_all()`: Replaced by `app.emit()` in Tauri v2

## Open Questions

1. **Watcher lifecycle on window close/reopen**
   - What we know: User decided watcher should stop on window close and restart on window open.
   - What's unclear: Whether to use Tauri's window lifecycle events (`WINDOW_DESTROYED`/`WINDOW_CREATED`) or the `RunEvent::ExitRequested` handler. Also, for a single-window desktop app, the simplest approach may be to just let the watcher run for the entire app lifetime (start in setup, never stop), since closing the window typically closes the app.
   - Recommendation: Keep the watcher running for the app lifetime (start once in `setup()`). If the app uses window hide/show instead of destroy/create, stopping/restarting the watcher adds unnecessary complexity. The watcher consuming events while the window is hidden is harmless.

2. **Self-write tracker integration point**
   - What we know: Need to call `tracker.record_write()` after every provider file write.
   - What's unclear: Whether to modify `storage/icloud.rs` functions to accept the tracker, or intercept at the command layer.
   - Recommendation: Intercept at the command layer (commands/provider.rs) since it has access to Tauri state. Add `app_handle: tauri::AppHandle` parameter to commands that write files, call `tracker.record_write()` after successful writes. Alternatively, the watcher module can expose a standalone function that the commands call.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`#[test]`) + cargo test |
| Config file | src-tauri/Cargo.toml (dev-dependencies: tempfile) |
| Quick run command | `cd src-tauri && cargo test` |
| Full suite command | `cd src-tauri && cargo test` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SYNC-03 | File watcher detects changes in providers directory | unit | `cd src-tauri && cargo test watcher -x` | No - Wave 0 |
| SYNC-03 | Self-write tracker ignores recent writes within 1s | unit | `cd src-tauri && cargo test self_write -x` | No - Wave 0 |
| SYNC-03 | Non-json files are filtered out | unit | `cd src-tauri && cargo test watcher::filter -x` | No - Wave 0 |
| SYNC-04 | providers-changed event emitted with correct payload | integration | Manual - requires Tauri runtime | N/A manual-only |
| SYNC-05 | sync_active_providers called on watcher event | unit | `cd src-tauri && cargo test watcher::repatch -x` | No - Wave 0 |

### Sampling Rate
- **Per task commit:** `cd src-tauri && cargo test`
- **Per wave merge:** `cd src-tauri && cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src-tauri/src/watcher/self_write.rs` -- unit tests for SelfWriteTracker (record, expiry, cleanup)
- [ ] `src-tauri/src/watcher/mod.rs` -- unit tests for event filtering logic (json-only, dedup)
- Note: Full integration testing (watcher -> event -> frontend) requires running Tauri app; covered by manual verification

## Sources

### Primary (HIGH confidence)
- [notify crate docs](https://docs.rs/notify/latest/notify/) - API, RecommendedWatcher, FSEvents backend
- [notify-debouncer-mini docs](https://docs.rs/notify-debouncer-mini/latest/notify_debouncer_mini/) - new_debouncer API, v0.7.0
- [Tauri v2 Calling Frontend](https://v2.tauri.app/develop/calling-frontend/) - app.emit(), Emitter trait, setup hook
- [Tauri v2 Event API](https://v2.tauri.app/reference/javascript/api/namespaceevent/) - listen(), unlisten, Event type
- [Tauri v2 State Management](https://v2.tauri.app/develop/state-management/) - manage(), Mutex patterns, AppHandle in threads

### Secondary (MEDIUM confidence)
- [notify GitHub](https://github.com/notify-rs/notify) - v8.2.0 latest stable, MSRV 1.85, macos_fsevent feature
- [Tauri tutorials](https://tauritutorials.com/blog/tauri-events-basics) - practical event patterns

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - notify is the undisputed standard for Rust file watching; Tauri events are the official communication mechanism
- Architecture: HIGH - patterns follow official Tauri docs and established notify usage
- Pitfalls: HIGH - iCloud event storms and self-write loops are well-documented challenges in file-watching systems

**Research date:** 2026-03-11
**Valid until:** 2026-04-11 (stable ecosystem, 30-day validity)
