# Phase 4: iCloud Sync and File Watching - Context

**Gathered:** 2026-03-11
**Status:** Ready for planning

<domain>
## Phase Boundary

FSEvents-based file watcher monitors the iCloud providers directory for changes from other devices. When changes arrive, the UI refreshes automatically and active providers trigger CLI config re-patching. Handles event storms gracefully with debouncing and self-write filtering. No new capabilities (no new CRUD, no new UI pages) -- this phase adds reactive sync to existing functionality.

</domain>

<decisions>
## Implementation Decisions

### Self-write detection
- Track writes per file path with a 1-second ignore window after each write
- When we write a provider file, record that file path + timestamp
- FSEvents for that file within 1 second are silently skipped
- Other files' events during the same window are processed normally

### UI refresh behavior
- On sync change: background refresh + toast notification showing specific Provider name (e.g., "已同步：Provider 'My API' 已更新")
- Toast auto-dismisses after 2-3 seconds
- When active Provider is modified by sync AND CLI config is re-patched, show an additional toast indicating CLI config was updated (e.g., "Claude 配置已自动更新")

### iCloud file eviction
- v1 does NOT handle file eviction (no objc2/Swift bridge needed)
- If a file read fails (evicted or other IO error), skip that file gracefully and log the error
- Continue processing other files normally -- never crash on a single file failure

### Event debounce strategy
- 500ms debounce window: after receiving first event, collect all events for 500ms, then process as a batch
- Multiple file changes within the window are merged into a single refresh operation
- Filter out non-.json files at the event handling layer (ignore .icloud placeholders, .tmp files, etc.)

### Backend-to-frontend communication
- Use Tauri event system: Rust emits a single 'providers-changed' event via app.emit()
- Event payload includes list of changed file names (for toast content)
- Frontend listens with listen() and triggers full provider list reload
- No polling -- purely event-driven

### Startup initial sync
- On app startup, simply read all providers from iCloud directory (existing behavior -- no local cache means every startup gets fresh data)
- Call sync_active_providers on startup to re-patch all active providers to CLI configs, ensuring consistency after offline iCloud changes
- No snapshot comparison needed -- the architecture naturally handles this

### File watcher lifecycle
- Watcher starts when app window opens, stops when window closes
- Window re-open restarts the watcher
- Watcher targets the iCloud providers directory (same path as get_icloud_providers_dir())

### Re-patch failure handling
- When sync-triggered auto re-patch fails, show an error toast to the user (e.g., "同步后自动更新 Claude 配置失败")
- No automatic retry -- user can manually re-switch to trigger another patch
- Log the error for debugging

### Claude's Discretion
- Specific FSEvents/notify crate configuration details
- Debounce implementation approach (timer-based, channel-based, etc.)
- Toast component library choice or implementation
- Exact event payload structure
- Thread/async architecture for the watcher

</decisions>

<specifics>
## Specific Ideas

- sync_active_providers command already exists and handles re-patching all active providers -- reuse this for startup sync and sync-triggered re-patch
- useProviders and useSettings hooks already have refresh() callbacks -- wire Tauri event listener to call these
- The 'providers-changed' event should carry enough info to build meaningful toast messages (Provider names, change types)

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `sync_active_providers` command (commands/provider.rs): Already iterates active_providers and patches each CLI -- perfect for startup sync and sync-triggered re-patch
- `useProviders.refresh()` (src/hooks/useProviders.ts): Re-fetches provider list from backend -- wire to event listener
- `useSettings.refresh()` (src/hooks/useSettings.ts): Re-fetches settings -- may be needed if active provider changes
- `get_icloud_providers_dir()` (storage/icloud.rs): Returns the watch target directory with iCloud/fallback logic
- `atomic_write()` (storage/mod.rs): Uses temp+rename pattern -- self-write detection must account for this

### Established Patterns
- Tauri commands are thin wrappers delegating to storage modules (commands/provider.rs)
- Internal `_in/_to` function variants for testability with injectable paths
- Injectable adapter pattern via `Option<Box<dyn CliAdapter>>` for test isolation

### Integration Points
- `lib.rs` run(): Tauri Builder setup -- watcher initialization hooks into app setup
- `tauri::Builder::default()`: Need to add setup() hook for watcher initialization and manage state
- Frontend `App` component or root-level effect: Add Tauri event listener for 'providers-changed'

</code_context>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 04-icloud-sync-and-file-watching*
*Context gathered: 2026-03-11*
