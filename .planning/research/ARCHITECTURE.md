# Architecture Patterns

**Domain:** Desktop CLI configuration manager (Tauri 2)
**Researched:** 2026-03-10
**Confidence:** HIGH (based on detailed analysis of cc-switch reference code + Tauri 2 architecture knowledge + iCloud sync root-cause analysis)

## Recommended Architecture

CLIManager uses a **layered architecture** with clear separation between IPC surface, business logic, storage, and external I/O. The key departure from cc-switch is replacing SQLite SSOT with a **two-layer file-based storage** model designed for iCloud safety, and replacing whole-file rewrites with **surgical Read-Modify-Write patching**.

```
+------------------------------------------------------------------+
|                        React Frontend                             |
|  +------------------+  +------------------+  +----------------+  |
|  | Provider List UI |  | Provider Editor  |  | Settings UI    |  |
|  +--------+---------+  +--------+---------+  +-------+--------+  |
|           |                     |                     |           |
|  +--------+---------------------+---------------------+--------+ |
|  |              TanStack Query (cache + mutations)             | |
|  +--------+---------------------+---------------------+--------+ |
|           |                     |                     |           |
|  +--------+---------------------+---------------------+--------+ |
|  |           IPC API Layer (invoke wrappers)                   | |
|  +-------------------------------------------------------------+ |
+------------------------------------------------------------------+
                              | Tauri IPC (JSON)
+------------------------------------------------------------------+
|                        Rust Backend                               |
|                                                                   |
|  +-------------------------------------------------------------+ |
|  |              Commands Layer (thin IPC handlers)              | |
|  |  provider.rs | settings.rs | import.rs | watcher.rs         | |
|  +--------+---------------------+---------------------+--------+ |
|           |                     |                     |           |
|  +--------+---------------------+---------------------+--------+ |
|  |              Services Layer (business logic)                | |
|  |  ProviderService | SyncService | WatcherService             | |
|  |  ImportService   | SettingsService                          | |
|  +--------+---------------------+---------------------+--------+ |
|           |                     |                     |           |
|  +--------v--------+  +--------v--------+  +--------v--------+  |
|  | Storage Layer   |  | CLI Adapters    |  | File Watcher    |  |
|  | (iCloud + Local)|  | (per-CLI R/M/W) |  | (FSEvents)      |  |
|  +-----------------+  +-----------------+  +-----------------+  |
+------------------------------------------------------------------+
                |                |                     |
    +-----------v---+   +-------v--------+   +--------v--------+
    | iCloud Drive  |   | ~/.claude/     |   | iCloud sync dir |
    | sync dir      |   | ~/.codex/      |   | (watched)       |
    | (per-provider |   | (CLI live      |   |                 |
    |  JSON files)  |   |  configs)      |   |                 |
    +---------------+   +----------------+   +-----------------+
    | ~/.cli-manager/|
    | local.json     |
    +----------------+
```

### Component Boundaries

| Component | Responsibility | Communicates With | Location |
|-----------|---------------|-------------------|----------|
| **Commands** | Thin Tauri IPC handlers; parse args, delegate to services, map errors | Services (calls), Frontend (receives IPC from) | `src-tauri/src/commands/` |
| **ProviderService** | Provider CRUD, switching, validation | Storage Layer (read/write providers), CLI Adapters (patch on switch) | `src-tauri/src/services/provider.rs` |
| **SyncService** | Orchestrates switch flow: load provider -> patch all applicable CLIs -> update local state | ProviderService, CLI Adapters, SettingsService | `src-tauri/src/services/sync.rs` |
| **WatcherService** | FSEvents watcher on iCloud sync directory; debounce + emit events | Storage Layer (detects changes), Frontend (emits Tauri events) | `src-tauri/src/services/watcher.rs` |
| **ImportService** | First-launch scan of existing CLI configs to create initial providers | CLI Adapters (read current), ProviderService (create) | `src-tauri/src/services/import.rs` |
| **SettingsService** | Device-local settings management (active provider, locale, paths) | Local Storage (read/write local.json) | `src-tauri/src/services/settings.rs` |
| **Storage Layer** | Two-layer file I/O: iCloud sync dir (per-provider JSON) + local.json | Filesystem directly | `src-tauri/src/storage/` |
| **CLI Adapters** | Per-CLI Read-Modify-Write logic for surgical patching | CLI config files on disk | `src-tauri/src/adapters/` |
| **File Watcher** | Low-level FSEvents binding, raw event stream | WatcherService (provides events to) | `src-tauri/src/watcher/` |
| **Frontend IPC Layer** | TypeScript invoke wrappers with typed args/returns | Tauri IPC bridge | `src/lib/api/` |
| **Frontend Query Layer** | TanStack Query queries/mutations, cache invalidation | IPC Layer, React components | `src/lib/query/` |
| **Frontend UI** | React components for provider management and settings | Query Layer | `src/components/` |

### Data Flow

#### Flow 1: User Switches Provider (Primary Happy Path)

```
User clicks "Switch" in UI
  -> React component calls mutation
    -> invoke("switch_provider", { id: "xxx" })
      -> Commands::switch_provider()
        -> SyncService::switch(provider_id)
          1. ProviderService::get(provider_id)  -- reads from iCloud sync dir
          2. For each supported CLI (Claude Code, Codex):
             CLIAdapter::patch(provider)        -- Read-Modify-Write on CLI config
          3. SettingsService::set_active(provider_id)  -- writes local.json
          4. Return SwitchResult { warnings }
      -> Command returns Result to frontend
    -> TanStack Query invalidates relevant queries
  -> UI updates to show new active provider
```

#### Flow 2: iCloud Sync Triggers Provider Refresh

```
Another device saves/updates a provider JSON file
  -> iCloud Drive syncs file to local disk
    -> FSEvents fires event on sync directory
      -> WatcherService receives event
        -> Debounce (100-300ms window)
        -> Determine change type (add/modify/delete)
        -> Emit Tauri event: "providers-changed" { kind, provider_id }
          -> Frontend listener receives event
            -> TanStack Query invalidates provider list query
            -> UI re-renders with fresh data
        -> If changed provider is active provider:
           -> SyncService::re_patch_active()
             -> Read updated provider from sync dir
             -> Re-patch all CLI configs with new credentials
```

#### Flow 3: Provider CRUD (Create as example)

```
User fills form, clicks "Save"
  -> invoke("create_provider", { provider })
    -> Commands::create_provider()
      -> ProviderService::create(provider)
        1. Validate fields (protocol type, required credentials)
        2. Generate UUID
        3. Write {uuid}.json to iCloud sync dir
        4. Return created provider
    -> Frontend invalidates provider list
  -> UI shows new provider in list
```

#### Flow 4: First Launch Auto-Import

```
App starts
  -> init() in lib.rs setup
    -> ImportService::scan_and_import()
      1. For each supported CLI:
         adapter.read_current_config()  -- read live config
         If credentials found:
           ProviderService::create(extracted_provider)
      2. If any providers created:
         SettingsService::set_active(first_created.id)
      3. Emit "import-complete" event to frontend
```

## Component Design Details

### Storage Layer: Two-Layer File Architecture

This is the most critical architectural decision. It replaces cc-switch's SQLite SSOT with a design that is inherently iCloud-safe.

**Sync Layer (iCloud Drive directory)**
```
~/Library/Mobile Documents/com~apple~CloudDocs/CLIManager/
  providers/
    {uuid-1}.json    # One file per provider
    {uuid-2}.json
    {uuid-3}.json
```

Each provider file:
```json
{
  "id": "uuid-1",
  "name": "Anthropic Direct",
  "protocol": "anthropic",
  "credentials": {
    "api_key": "sk-ant-...",
    "base_url": "https://api.anthropic.com"
  },
  "model": "claude-sonnet-4-20250514",
  "notes": "Personal account",
  "icon": "anthropic",
  "icon_color": "#D97757",
  "sort_index": 0,
  "created_at": 1710000000,
  "updated_at": 1710000100
}
```

**Why one file per provider:**
- iCloud syncs at file granularity. Editing Provider A never creates a conflict with Provider B
- No cross-file transaction needed -- each file is its own atomic unit
- Deleting a provider = deleting a file -- iCloud handles this naturally
- File-level conflict resolution is manageable (last-write-wins is acceptable for single-provider edits)

**Local Layer (device-specific, NOT synced)**
```
~/.cli-manager/
  local.json         # Device-local state
```

local.json:
```json
{
  "active_provider_id": "uuid-1",
  "locale": "zh-CN",
  "claude_config_dir": null,
  "codex_config_dir": null,
  "sync_dir": "~/Library/Mobile Documents/com~apple~CloudDocs/CLIManager"
}
```

**Why separate:**
- Active provider is per-device (my Mac uses Provider A, my MacBook Pro uses Provider B)
- Config directory overrides are per-device (different install paths)
- Prevents the "state ping-pong" bug in cc-switch where syncing settings.json caused active provider to flip between devices

### CLI Adapters: Surgical Patch via Read-Modify-Write

Each CLI adapter implements a common trait:

```rust
pub trait CliAdapter {
    /// Which protocol types this CLI supports
    fn supported_protocols(&self) -> &[ProtocolType];

    /// Read current credential fields from live config
    fn read_credentials(&self) -> Result<Option<Credentials>, AdapterError>;

    /// Surgical patch: read config, modify only credential+model fields, write back
    fn patch(&self, provider: &Provider) -> Result<PatchResult, AdapterError>;

    /// Remove credentials written by this app (for "deactivate" scenarios)
    fn unpatch(&self) -> Result<(), AdapterError>;
}
```

**Claude Code Adapter** (`adapters/claude.rs`):
- Target: `~/.claude/settings.json` (JSON)
- Fields to patch: `env.ANTHROPIC_AUTH_TOKEN`, `env.ANTHROPIC_BASE_URL`, `env.ANTHROPIC_MODEL` (via deep merge)
- Read-Modify-Write: `serde_json::from_str` -> modify env keys -> `serde_json::to_string_pretty` -> write
- Critical: preserve all other keys in settings.json (permissions, allowedTools, etc.)

**Codex Adapter** (`adapters/codex.rs`):
- Target: `~/.codex/auth.json` (JSON) + `~/.codex/config.toml` (TOML)
- Fields: auth.json has API key, config.toml has model + base_url
- Two-phase write with rollback (carry over from cc-switch's proven pattern)
- Use `toml_edit` (not `toml`) to preserve comments and formatting in config.toml

### Protocol Type Model

Providers are modeled by API protocol, not by CLI:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    Anthropic,        // Anthropic native API (api.anthropic.com)
    OpenAiCompatible, // OpenAI-compatible (OpenRouter, Azure, etc.)
}
```

Each protocol defines which credential fields are required:

```rust
impl ProtocolType {
    pub fn required_fields(&self) -> &[&str] {
        match self {
            ProtocolType::Anthropic => &["api_key"],
            ProtocolType::OpenAiCompatible => &["api_key", "base_url"],
        }
    }
}
```

Each CLI adapter knows how to map protocol credentials to its own config format. For example:
- Claude Code + Anthropic protocol -> writes `ANTHROPIC_AUTH_TOKEN` to env
- Claude Code + OpenAI-compatible -> writes `ANTHROPIC_AUTH_TOKEN` + `ANTHROPIC_BASE_URL` to env (Claude Code uses these env vars for compatible endpoints too)
- Codex + Anthropic protocol -> writes API key to auth.json, no base_url in config.toml
- Codex + OpenAI-compatible -> writes API key to auth.json, model + base_url to config.toml

### File Watcher: FSEvents-Based with Debounce

```rust
pub struct WatcherService {
    watcher: RecommendedWatcher,  // from `notify` crate
    debouncer: Debouncer,
}

impl WatcherService {
    pub fn new(sync_dir: PathBuf, app_handle: AppHandle) -> Self {
        // Watch sync_dir/providers/ recursively
        // On events: debounce 200ms, then:
        //   - Parse which provider file changed
        //   - Emit "providers-changed" Tauri event
        //   - If active provider changed, trigger re-patch
    }
}
```

**Debounce rationale:** iCloud may trigger multiple FSEvents for a single logical file update (write + metadata update + xattr update). A 200ms debounce window coalesces these into one logical event.

**Event types to handle:**
- `Create` -> new provider synced from another device
- `Modify` -> provider updated on another device
- `Remove` -> provider deleted on another device
- Ignore `.icloud` placeholder files (iCloud "evicted" state)

### Application State

Simpler than cc-switch (no SQLite, no ProxyService):

```rust
pub struct AppState {
    pub sync_dir: PathBuf,           // iCloud sync directory
    pub local_settings: RwLock<LocalSettings>,  // Device-local state (from local.json)
    pub watcher: Mutex<Option<WatcherService>>, // File watcher (initialized after setup)
}
```

Using `RwLock` for local_settings because reads are far more frequent than writes. Using `Mutex<Option<...>>` for watcher because it's initialized lazily during app setup.

### Frontend Architecture

```
src/
  main.tsx              # App entry, QueryClientProvider, Tauri event listeners
  App.tsx               # Router / main layout
  components/
    providers/
      ProviderList.tsx  # List with switch buttons
      ProviderForm.tsx  # Create/edit form
      ProviderCard.tsx  # Individual provider display
    settings/
      SettingsPage.tsx  # Locale, paths, sync dir
    layout/
      Sidebar.tsx       # Navigation
      Header.tsx        # App title, active provider indicator
  lib/
    api/
      providers.ts      # invoke("get_providers"), invoke("switch_provider"), etc.
      settings.ts       # invoke("get_settings"), invoke("update_settings")
      import.ts         # invoke("scan_and_import")
    query/
      providers.ts      # useQuery/useMutation hooks for providers
      settings.ts       # useQuery/useMutation hooks for settings
    i18n/
      index.ts          # i18n setup
      locales/
        zh-CN.json
        en-US.json
    types/
      provider.ts       # Provider, ProtocolType, Credentials TypeScript types
      settings.ts       # LocalSettings type
```

**Tauri event listener pattern** (in main.tsx):

```typescript
import { listen } from "@tauri-apps/api/event";

// On "providers-changed" from backend watcher:
listen("providers-changed", (event) => {
  queryClient.invalidateQueries({ queryKey: ["providers"] });
});
```

This keeps the frontend reactive to iCloud-synced changes without polling.

## Patterns to Follow

### Pattern 1: Thin Commands, Fat Services

**What:** Command handlers do only argument parsing and error mapping. All business logic lives in services.

**When:** Every Tauri command.

**Why:** Keeps the IPC surface declarative and testable. Services can be unit-tested without Tauri runtime.

**Example (Rust):**
```rust
// commands/provider.rs -- thin
#[tauri::command]
pub fn switch_provider(
    state: State<'_, AppState>,
    id: String,
) -> Result<SwitchResult, String> {
    SyncService::switch(&state, &id).map_err(|e| e.to_string())
}

// services/sync.rs -- fat
impl SyncService {
    pub fn switch(state: &AppState, provider_id: &str) -> Result<SwitchResult, AppError> {
        let provider = ProviderService::get(state, provider_id)?;
        let mut warnings = Vec::new();

        for adapter in get_adapters_for_protocol(&provider.protocol) {
            match adapter.patch(&provider) {
                Ok(result) => warnings.extend(result.warnings),
                Err(e) => warnings.push(format!("Failed to patch {}: {}", adapter.name(), e)),
            }
        }

        SettingsService::set_active(state, provider_id)?;
        Ok(SwitchResult { warnings })
    }
}
```

### Pattern 2: Read-Modify-Write with Preserve

**What:** When patching a CLI config file, read the entire file, modify only the target fields, write the entire file back. Never construct a new file from scratch.

**When:** Every CLI adapter patch operation.

**Why:** This is the core value proposition of CLIManager over cc-switch. cc-switch's `atomic_write` reconstructed config from SSOT, which destroyed user settings (permissions, tools, etc.).

**Example (Rust):**
```rust
// adapters/claude.rs
fn patch(&self, provider: &Provider) -> Result<PatchResult, AdapterError> {
    let path = self.settings_path();

    // READ: Load existing config (or empty object if file doesn't exist)
    let mut config: Value = if path.exists() {
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content)?
    } else {
        json!({})
    };

    // MODIFY: Only touch credential fields
    let env = config.as_object_mut()
        .unwrap()
        .entry("env")
        .or_insert(json!({}));

    env["ANTHROPIC_AUTH_TOKEN"] = json!(provider.credentials.api_key);
    if let Some(base_url) = &provider.credentials.base_url {
        env["ANTHROPIC_BASE_URL"] = json!(base_url);
    }
    // Do NOT touch any other keys in the config

    // WRITE: Write back the full config
    let content = serde_json::to_string_pretty(&config)?;
    fs::write(&path, content)?;

    Ok(PatchResult::default())
}
```

### Pattern 3: Event-Driven Frontend Refresh

**What:** Backend emits Tauri events when data changes (from file watcher or from own writes). Frontend listens and invalidates TanStack Query caches.

**When:** Any backend state change that the frontend should reflect.

**Why:** Decouples backend state changes from frontend polling. Works for both local changes and iCloud-synced remote changes.

**Example (TypeScript):**
```typescript
// In main.tsx or a dedicated hook
useEffect(() => {
  const unlisten = listen("providers-changed", () => {
    queryClient.invalidateQueries({ queryKey: ["providers"] });
  });
  return () => { unlisten.then(fn => fn()); };
}, []);
```

### Pattern 4: Adapter Registry

**What:** CLI adapters are registered in a central registry. When switching providers, iterate applicable adapters by protocol type.

**When:** Provider switch, first-launch import, config validation.

**Why:** Adding a new CLI support (e.g., OpenCode in v2) requires only adding a new adapter module and registering it. Zero changes to switch logic, commands, or frontend.

**Example (Rust):**
```rust
pub fn get_adapters() -> Vec<Box<dyn CliAdapter>> {
    vec![
        Box::new(ClaudeAdapter::new()),
        Box::new(CodexAdapter::new()),
        // Future: Box::new(OpenCodeAdapter::new()),
    ]
}

pub fn get_adapters_for_protocol(protocol: &ProtocolType) -> Vec<Box<dyn CliAdapter>> {
    get_adapters()
        .into_iter()
        .filter(|a| a.supported_protocols().contains(protocol))
        .collect()
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Whole-File Rewrite on Switch

**What:** Constructing a complete config file from stored data and replacing the live config entirely.

**Why bad:** This is cc-switch's core bug. When you rebuild `settings.json` from your SSOT, you lose:
- User-configured `allowedTools` lists
- Permission settings
- Custom environment variables unrelated to the provider
- Any other settings the CLI added that your app doesn't model

**Instead:** Read-Modify-Write. Only touch the fields you own (credentials + model).

### Anti-Pattern 2: SQLite in iCloud Sync Directory

**What:** Placing a SQLite database in a directory synced by iCloud.

**Why bad:** Extensively documented in `icloud-sync-root-cause-zh.md`. SQLite depends on local filesystem lock semantics. iCloud's eventual consistency + lack of cross-device locks = corruption, conflict copies, phantom state resets.

**Instead:** Per-provider JSON files in iCloud. JSON files are append/replace-friendly. Single-file granularity avoids cross-file transaction issues.

### Anti-Pattern 3: Syncing Device-Local State

**What:** Putting active-provider-id, locale, or path overrides in the iCloud sync directory.

**Why bad:** Device A sets active = Provider1, Device B sets active = Provider2. iCloud syncs -- both devices now ping-pong between Provider1 and Provider2.

**Instead:** `local.json` stays in `~/.cli-manager/` which is NOT in iCloud.

### Anti-Pattern 4: Fat Commands

**What:** Putting business logic directly in `#[tauri::command]` handlers.

**Why bad:** Cannot unit test without Tauri runtime. Mixes IPC concerns with domain logic. Hard to reuse logic (e.g., watcher triggering re-patch uses same logic as user-initiated switch).

**Instead:** Commands delegate to services. Services are pure Rust with no Tauri dependency (except through passed state).

### Anti-Pattern 5: Unbounded File Watcher Events

**What:** Reacting to every FSEvent immediately without debouncing.

**Why bad:** iCloud file sync generates multiple events per logical change (content write, extended attribute update, metadata update). Processing each one triggers redundant re-reads and UI refreshes.

**Instead:** Debounce with a 200ms window. Coalesce events for the same file path.

## Suggested Build Order (Dependencies)

The components have clear dependency ordering. Building bottom-up ensures each layer has its foundation before the layer above starts.

```
Phase 1: Foundation
  Storage Layer (iCloud sync dir + local.json read/write)
  Provider data model (ProtocolType, Provider struct, validation)
  -> These have zero external dependencies. Everything else builds on them.

Phase 2: Core Logic
  CLI Adapters (Claude Code + Codex R/M/W)
  ProviderService (CRUD using Storage Layer)
  SettingsService (local.json management)
  -> Depends on Phase 1. This is where the core value lives.

Phase 3: Integration
  SyncService (orchestrates switch: provider -> adapters -> settings)
  Commands Layer (IPC surface for all services)
  -> Depends on Phase 2. Wire everything to Tauri IPC.

Phase 4: Frontend Shell
  Frontend types + IPC wrappers
  TanStack Query hooks
  Basic Provider list + switch UI
  -> Depends on Phase 3 commands existing. First usable UI.

Phase 5: Reactive Features
  File Watcher (FSEvents on sync dir)
  WatcherService (debounce + event emission)
  Frontend event listeners (cache invalidation on sync)
  -> Depends on Phase 4 having working UI to refresh.

Phase 6: Onboarding
  ImportService (first-launch scan)
  i18n setup
  Settings UI (locale, path overrides)
  -> Can be built in parallel with Phase 5. Not blocking core flow.
```

**Critical path:** Phase 1 -> 2 -> 3 -> 4 (minimum viable product)

**Parallelizable:** Phase 5 and 6 can be developed concurrently once Phase 4 is complete.

## Scalability Considerations

| Concern | At 5 providers | At 50 providers | At 500 providers |
|---------|---------------|-----------------|-------------------|
| Storage (sync dir) | 5 small JSON files, trivial | 50 files, still trivial for iCloud | Unlikely scenario; directory listing may slow. Consider index file |
| File watcher events | Infrequent, no concern | Moderate during bulk sync | Debounce window may need widening; batch processing |
| Provider list rendering | Simple flat list | Needs search/filter | Virtual scrolling, categorization |
| Switch operation | 2 CLI adapters, <100ms | Same -- switch is per-provider, not per-list | Same |
| iCloud sync bandwidth | Negligible (few KB) | Still negligible | Consider lazy download of .icloud placeholders |

**Realistic scale:** Most users will have 3-10 providers. The architecture handles 50+ without modification. 500+ is an edge case that can be addressed with an index file if ever needed.

## Key Differences from cc-switch

| Aspect | cc-switch | CLIManager |
|--------|-----------|------------|
| SSOT | SQLite database | Per-provider JSON files |
| Config write | Whole-file rewrite (`atomic_write`) | Surgical Read-Modify-Write |
| Sync mechanism | SQLite + settings.json in iCloud (broken) | Per-provider files in iCloud (safe by design) |
| Device state | settings.json (sometimes synced) | local.json (never synced) |
| CLI support | 5 CLIs (Claude/Codex/Gemini/OpenCode/OpenClaw) | 2 CLIs (Claude Code + Codex), extensible via adapter trait |
| Feature scope | Providers + MCP + Prompts + Skills + Proxy + Sessions | Providers only (v1) |
| State management | `AppState { db, proxy_service }` | `AppState { sync_dir, local_settings, watcher }` |
| Backend layers | Commands -> Services -> Database (DAO) | Commands -> Services -> Storage + Adapters |

## Sources

- cc-switch reference code (read-only): `/Users/kelin/Workspace/CLIManager/cc-switch/`
- cc-switch architecture notes: `/Users/kelin/Workspace/CLIManager/cc-switch-ref-notes-zh.md`
- iCloud sync root cause analysis: `/Users/kelin/Workspace/CLIManager/icloud-sync-root-cause-zh.md`
- Project requirements: `/Users/kelin/Workspace/CLIManager/.planning/PROJECT.md`
- Tauri 2 architecture patterns: Based on training data (Tauri 2.x stable release architecture). Confidence: HIGH -- Tauri 2's command/state/event model is well-documented and stable.
- `notify` crate for file watching: Standard Rust file watcher crate, wraps FSEvents on macOS. Confidence: HIGH.
- `toml_edit` for comment-preserving TOML editing: Confidence: HIGH -- this is the standard approach for preserving TOML formatting.
