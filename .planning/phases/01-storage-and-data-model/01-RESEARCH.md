# Phase 1: Storage and Data Model - Research

**Researched:** 2026-03-10
**Domain:** Tauri 2 project scaffolding, two-layer JSON storage architecture, protocol-based provider data model
**Confidence:** HIGH

## Summary

Phase 1 establishes the project foundation: a Tauri 2 desktop app with React/TypeScript frontend, a two-layer storage architecture (iCloud sync layer + device-local layer), and a protocol-based Provider data model. The project is greenfield -- no existing code to integrate with (cc-switch in `cc-switch/` is read-only reference only).

The iCloud sync layer stores one JSON file per Provider in `~/Library/Mobile Documents/com~apple~CloudDocs/CLIManager/providers/`. The device-local layer stores settings in `~/.cli-manager/local.json`. This separation ensures device-specific state (active provider, path overrides) never syncs across devices, avoiding the cross-device state conflicts that plagued cc-switch.

The Provider data model uses a `protocol_type` enum (Anthropic, OpenAI-compatible) rather than binding providers to specific CLIs. This enables future CLI reuse -- a single provider can serve multiple CLIs that share the same protocol.

**Primary recommendation:** Scaffold with `pnpm create tauri-app`, use UUID v4 for provider file naming, keep the Provider schema minimal (avoid cc-switch's bloated 50+ field model), and implement Rust-side CRUD with serde + thiserror for proper error handling.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None -- all implementation decisions are delegated to Claude's discretion.

### Claude's Discretion
- Provider JSON Schema: field design, protocol type enum, file naming, schema versioning
- iCloud directory location: specific path, subfolder naming, fallback when iCloud unavailable
- Local settings schema: `~/.cli-manager/local.json` structure, active provider tracking approach
- Project scaffolding: package manager, React config, state management, test framework

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SYNC-01 | Provider data stored as individual JSON files in iCloud Drive directory | iCloud path research, per-file JSON CRUD pattern, UUID-based naming |
| SYNC-02 | Device-local settings stored in `~/.cli-manager/local.json`, never synced | Local settings schema design, separation from iCloud layer |
| ADPT-03 | Provider data model uses protocol type for future CLI reuse | Protocol type enum design, extensible schema pattern from cc-switch reference |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri | 2.10.x | Desktop app framework (Rust backend) | Official latest stable, cc-switch uses 2.8.2 |
| @tauri-apps/cli | ^2.8.0 | Tauri CLI tooling | Required for `tauri dev` / `tauri build` |
| react | ^19.x | Frontend UI framework | Project constraint from PROJECT.md |
| typescript | ^5.x | Type safety for frontend | Standard for React projects |
| vite | ^6.x or ^7.x | Frontend build tool | Tauri 2 default bundler with create-tauri-app |
| serde + serde_json | 1.0 | Rust JSON serialization | Universal Rust standard, already in Tauri deps |
| uuid | 1.x (features: v4) | Provider file ID generation | v4 random UUIDs for file naming, simple and collision-safe |
| dirs | 5.0 | Home directory resolution | Cross-platform home dir, used by cc-switch |
| thiserror | 2.0 | Error type definitions | Standard Rust error handling, used by cc-switch |
| chrono | 0.4 | Timestamps | Created/updated timestamps in provider data |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| pnpm | latest | Package manager | Project-wide JS dependency management |
| vitest | ^2.x | Frontend unit testing | Testing React components and TypeScript logic |
| @tauri-apps/api | ^2.x | Frontend-to-Rust bridge | Invoking Tauri commands from React |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| UUID v4 file names | Slug-based names | Slugs are human-readable but risk collisions and special character issues; UUID v4 is safer for iCloud sync |
| pnpm | npm/yarn/bun | cc-switch uses pnpm; consistency is valuable, pnpm is fastest for monorepo-like setups |
| vitest | jest | vitest integrates natively with Vite; cc-switch already uses vitest |

**Installation (frontend):**
```bash
pnpm create tauri-app CLIManager -- --template react-ts
cd CLIManager
pnpm install
```

**Cargo.toml additions (Rust backend):**
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1", features = ["v4"] }
dirs = "5.0"
thiserror = "2.0"
chrono = { version = "0.4", features = ["serde"] }
log = "0.4"
tauri = { version = "2", features = [] }
tauri-plugin-log = "2"
```

## Architecture Patterns

### Recommended Project Structure
```
CLIManager/
├── src/                     # React frontend
│   ├── App.tsx
│   ├── main.tsx
│   └── lib/
│       └── bindings.ts      # Tauri command type bindings
├── src-tauri/               # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/        # Tauri 2 permission capabilities
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── error.rs          # AppError enum with thiserror
│       ├── provider.rs       # Provider data model + protocol types
│       ├── storage/
│       │   ├── mod.rs
│       │   ├── icloud.rs     # iCloud layer: provider JSON file CRUD
│       │   └── local.rs      # Local layer: ~/.cli-manager/local.json
│       └── commands/
│           ├── mod.rs
│           └── provider.rs   # Tauri commands for provider CRUD
├── package.json
├── vite.config.ts
└── tsconfig.json
```

### Pattern 1: Two-Layer Storage Architecture
**What:** Provider data lives in iCloud Drive (synced), device settings live locally (never synced).
**When to use:** Always -- this is the core architectural decision.

iCloud sync layer path:
```
~/Library/Mobile Documents/com~apple~CloudDocs/CLIManager/providers/
  {uuid}.json    # One file per provider
```

Local layer path:
```
~/.cli-manager/
  local.json     # Device-local settings
```

### Pattern 2: Provider JSON Schema (per-file)
**What:** Each provider is a standalone JSON file with all data needed to configure CLI adapters.

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "My Anthropic Direct",
  "protocol_type": "anthropic",
  "api_key": "sk-ant-...",
  "base_url": "https://api.anthropic.com",
  "model": "claude-sonnet-4-20250514",
  "model_config": {
    "haiku_model": "claude-haiku-4-20250514",
    "sonnet_model": "claude-sonnet-4-20250514",
    "opus_model": "claude-opus-4-20250514"
  },
  "notes": "",
  "created_at": 1710000000000,
  "updated_at": 1710000000000,
  "schema_version": 1
}
```

Rust struct:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    Anthropic,
    OpenAiCompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haiku_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sonnet_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opus_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub protocol_type: ProtocolType,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub model_config: Option<ModelConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub schema_version: u32,
}
```

### Pattern 3: Local Settings Schema
**What:** Device-local settings that must never be synced.

```json
{
  "active_provider_id": "550e8400-e29b-41d4-a716-446655440000",
  "icloud_dir_override": null,
  "cli_paths": {
    "claude_config_dir": null,
    "codex_config_dir": null
  },
  "schema_version": 1
}
```

Design decision: Track ONE active provider globally (not per-CLI). Rationale: a user switches their "provider" (API endpoint + key), and each CLI adapter reads the relevant fields from that provider based on protocol type. This is simpler than per-CLI tracking and matches the cc-switch UX.

### Pattern 4: Tauri 2 Command Pattern
**What:** Expose Rust CRUD operations as Tauri commands.

```rust
#[tauri::command]
fn list_providers() -> Result<Vec<Provider>, AppError> {
    crate::storage::icloud::list_providers()
}

#[tauri::command]
fn create_provider(provider: Provider) -> Result<Provider, AppError> {
    crate::storage::icloud::save_provider(&provider)?;
    Ok(provider)
}

#[tauri::command]
fn delete_provider(id: String) -> Result<(), AppError> {
    crate::storage::icloud::delete_provider(&id)
}
```

### Pattern 5: Error Handling
**What:** Custom error type using thiserror that serializes for Tauri frontend.

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO error at {path}: {source}")]
    Io { path: String, source: std::io::Error },
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Provider not found: {0}")]
    NotFound(String),
    #[error("iCloud directory not available")]
    ICloudUnavailable,
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}
```

### Anti-Patterns to Avoid
- **cc-switch's bloated Provider model:** 50+ fields including proxy, failover, usage scripts, partner badges. Keep it minimal -- only fields needed for CLI config patching.
- **SQLite in iCloud Drive:** cc-switch's root cause of sync corruption. Use individual JSON files instead.
- **Storing active_provider_id in the iCloud layer:** This is device-specific state. If synced, switching provider on Device A would unexpectedly switch Device B.
- **Atomic whole-file rewrite for CLI configs:** This is Phase 2's concern (surgical patch), but the data model must not assume it. Provider stores raw credentials/model, not pre-formatted CLI config blobs (unlike cc-switch's `settings_config: Value` approach).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| UUID generation | Custom ID schemes | `uuid` crate v4 | Collision-free, no coordination needed |
| Home dir resolution | `$HOME` env var parsing | `dirs::home_dir()` | Handles edge cases on all platforms |
| JSON serialization | Manual string building | `serde_json` | Type-safe, handles escaping, standard |
| Error types | String-based errors | `thiserror` derive | Structured, composable, idiomatic Rust |
| Atomic file writes | Direct `fs::write` | Write-to-temp + rename | Prevents half-written files on crash/power loss |

**Key insight:** The cc-switch reference code already demonstrates proper atomic_write (temp file + rename) and dirs-based path resolution. Reuse these patterns but NOT the code itself (cc-switch is read-only reference).

## Common Pitfalls

### Pitfall 1: iCloud Drive Path Assumptions
**What goes wrong:** Hard-coding `~/Library/Mobile Documents/com~apple~CloudDocs/` without checking it exists.
**Why it happens:** iCloud Drive may be disabled, user not logged into iCloud, or path doesn't exist yet.
**How to avoid:** Check directory existence at startup. If missing, create it. If iCloud Drive is entirely unavailable (the `Mobile Documents` parent doesn't exist), fall back to `~/.cli-manager/providers/` with a warning. Store the resolved path in app state.
**Warning signs:** `NotFound` errors on first provider create.

### Pitfall 2: File Name Conflicts with iCloud Sync
**What goes wrong:** Using human-readable file names (e.g., `my-provider.json`) that could collide when two devices create providers with similar names.
**Why it happens:** iCloud sync is eventually consistent; two devices can create files with the same name before sync propagates.
**How to avoid:** Use UUID v4 for file names (`{uuid}.json`). UUIDs are statistically unique without coordination.
**Warning signs:** Files being overwritten or iCloud creating `filename (2).json` duplicates.

### Pitfall 3: Rust Symlink Issues in iCloud Container
**What goes wrong:** Rust's `std::fs` has known issues with symlinks in iCloud containers (rust-lang/rust#109381).
**Why it happens:** iCloud uses symlinks internally for file eviction/download states.
**How to avoid:** For a non-sandboxed Tauri app, use the direct filesystem path (`~/Library/Mobile Documents/com~apple~CloudDocs/...`) rather than using `FileManager.url(forUbiquityContainerIdentifier:)`. Non-sandboxed apps bypass the symlink issues because they write to the actual filesystem location.
**Warning signs:** `Permission denied` or `No such file or directory` errors when the file visually exists in Finder.

### Pitfall 4: Tauri 2 Command Return Types
**What goes wrong:** Returning `Result<T, Box<dyn std::error::Error>>` from Tauri commands.
**Why it happens:** Standard Rust error types don't implement `serde::Serialize`.
**How to avoid:** Define a custom `AppError` enum with `thiserror` + manual `Serialize` impl. Return `Result<T, AppError>` from all commands.
**Warning signs:** Compile errors about `Serialize` not implemented for error types.

### Pitfall 5: Forgetting schema_version
**What goes wrong:** Schema evolution becomes impossible without a version field in persisted files.
**Why it happens:** Seems unnecessary at v1 when there's only one schema.
**How to avoid:** Include `schema_version: 1` in every persisted JSON file from day one. Future migrations read this field to decide upgrade path.
**Warning signs:** Breaking changes requiring manual data migration.

## Code Examples

### iCloud Layer: Provider CRUD

```rust
// Source: designed based on cc-switch config.rs patterns + Tauri 2 best practices

use std::fs;
use std::path::PathBuf;

fn get_icloud_providers_dir() -> Result<PathBuf, AppError> {
    let home = dirs::home_dir().ok_or(AppError::ICloudUnavailable)?;
    let icloud_dir = home
        .join("Library/Mobile Documents/com~apple~CloudDocs/CLIManager/providers");

    if !icloud_dir.exists() {
        fs::create_dir_all(&icloud_dir)
            .map_err(|e| AppError::Io {
                path: icloud_dir.display().to_string(),
                source: e,
            })?;
    }
    Ok(icloud_dir)
}

fn provider_file_path(id: &str) -> Result<PathBuf, AppError> {
    Ok(get_icloud_providers_dir()?.join(format!("{}.json", id)))
}

pub fn list_providers() -> Result<Vec<Provider>, AppError> {
    let dir = get_icloud_providers_dir()?;
    let mut providers = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content = fs::read_to_string(&path)
                    .map_err(|e| AppError::Io {
                        path: path.display().to_string(),
                        source: e,
                    })?;
                let provider: Provider = serde_json::from_str(&content)?;
                providers.push(provider);
            }
        }
    }

    providers.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(providers)
}

pub fn save_provider(provider: &Provider) -> Result<(), AppError> {
    let path = provider_file_path(&provider.id)?;
    let json = serde_json::to_string_pretty(provider)?;
    atomic_write(&path, json.as_bytes())
}

pub fn delete_provider(id: &str) -> Result<(), AppError> {
    let path = provider_file_path(id)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| AppError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
    }
    Ok(())
}
```

### Local Layer: Device Settings

```rust
use std::fs;
use std::path::PathBuf;

fn get_local_settings_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".cli-manager/local.json")
}

pub fn read_local_settings() -> Result<LocalSettings, AppError> {
    let path = get_local_settings_path();
    if !path.exists() {
        return Ok(LocalSettings::default());
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| AppError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
    Ok(serde_json::from_str(&content)?)
}

pub fn write_local_settings(settings: &LocalSettings) -> Result<(), AppError> {
    let path = get_local_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
    }
    let json = serde_json::to_string_pretty(settings)?;
    atomic_write(&path, json.as_bytes())
}
```

### Atomic Write (reusable utility)

```rust
pub fn atomic_write(path: &std::path::Path, data: &[u8]) -> Result<(), AppError> {
    use std::io::Write;

    let parent = path.parent().ok_or_else(|| AppError::Io {
        path: path.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent dir"),
    })?;
    fs::create_dir_all(parent).map_err(|e| AppError::Io {
        path: parent.display().to_string(),
        source: e,
    })?;

    let tmp_path = parent.join(format!(
        ".{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));

    let mut file = fs::File::create(&tmp_path)
        .map_err(|e| AppError::Io { path: tmp_path.display().to_string(), source: e })?;
    file.write_all(data)
        .map_err(|e| AppError::Io { path: tmp_path.display().to_string(), source: e })?;
    file.flush()
        .map_err(|e| AppError::Io { path: tmp_path.display().to_string(), source: e })?;

    fs::rename(&tmp_path, path).map_err(|e| AppError::Io {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tauri 1.x | Tauri 2.x (latest 2.10.x) | Oct 2024 | New permission system, mobile support, plugin architecture |
| `create-tauri-app` v3 | `create-tauri-app` v4.6.x | 2025 | Updated templates, better TypeScript support |
| cc-switch SQLite storage | Per-file JSON in iCloud Drive | This project | Eliminates iCloud + SQLite conflict issues |
| cc-switch `settings_config: Value` blob | Structured Provider fields | This project | Type safety, no opaque JSON blobs |

**Deprecated/outdated:**
- Tauri 1.x API: completely different from Tauri 2.x, do not reference v1 docs
- `tauri-plugin-store` for main data: Use direct filesystem for iCloud-synced data; plugin-store is for app preferences only

## Open Questions

1. **iCloud Drive fallback behavior**
   - What we know: `~/Library/Mobile Documents/com~apple~CloudDocs/` is the standard path; non-sandboxed apps can access it directly
   - What's unclear: Should we silently fall back to `~/.cli-manager/providers/` or show a user-facing warning?
   - Recommendation: Fall back silently with a log warning. Phase 3 UI can later add a settings indicator.

2. **Active provider tracking granularity**
   - What we know: cc-switch tracks per-CLI (separate Claude/Codex providers). CLIManager's design says one provider can serve multiple CLIs via protocol type.
   - What's unclear: Does one global `active_provider_id` suffice, or will users want different providers active for different CLIs?
   - Recommendation: Start with one global `active_provider_id`. If needed later, extend `local.json` to `active_providers: { claude: "...", codex: "..." }`. The schema_version field enables this migration.

3. **Schema versioning strategy**
   - What we know: schema_version field in every file enables future migration
   - What's unclear: How to handle reading files with unknown future schema versions (created on a newer device)
   - Recommendation: If `schema_version > CURRENT_VERSION`, read what you can (forward-compatible serde with `#[serde(flatten)]` for unknown fields) and log a warning. Do not modify files with unknown versions.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | vitest (frontend), cargo test (Rust backend) |
| Config file | vitest: `vite.config.ts` (inline config) or `vitest.config.ts`; cargo: built-in |
| Quick run command | `pnpm test:unit` / `cargo test` |
| Full suite command | `pnpm test:unit && cd src-tauri && cargo test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SYNC-01 | Provider CRUD in iCloud dir | unit (Rust) | `cargo test storage::icloud::tests -p cli-manager -- --test-threads=1` | Wave 0 |
| SYNC-02 | Local settings read/write, isolation from iCloud | unit (Rust) | `cargo test storage::local::tests -p cli-manager` | Wave 0 |
| ADPT-03 | Protocol type enum serialization, extensibility | unit (Rust) | `cargo test provider::tests -p cli-manager` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test`
- **Per wave merge:** `pnpm test:unit && cd src-tauri && cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src-tauri/src/storage/icloud.rs` -- tests for CRUD operations (SYNC-01)
- [ ] `src-tauri/src/storage/local.rs` -- tests for local settings isolation (SYNC-02)
- [ ] `src-tauri/src/provider.rs` -- tests for ProtocolType serde round-trip (ADPT-03)
- [ ] Test utility: temp dir helper for isolating iCloud/local paths during tests (use `tempfile` crate)

## Sources

### Primary (HIGH confidence)
- [Tauri 2 official docs: Create a Project](https://v2.tauri.app/start/create-project/) -- project scaffolding
- [Tauri 2 official docs: Calling Rust](https://v2.tauri.app/develop/calling-rust/) -- command pattern, error handling
- [uuid crate docs](https://docs.rs/uuid/latest/uuid/) -- v4 UUID generation
- [dirs crate](https://crates.io/crates/dirs) -- home directory resolution
- cc-switch source code (`cc-switch/src-tauri/src/provider.rs`, `config.rs`) -- reference data model patterns

### Secondary (MEDIUM confidence)
- [iCloud Drive path on macOS](https://www.d4d.lt/how-to-access-icloud-drive-from-the-command-line-in-macos/) -- `~/Library/Mobile Documents/com~apple~CloudDocs/` path
- [Rust iCloud symlink issue](https://github.com/rust-lang/rust/issues/109381) -- known limitation, mitigated by non-sandboxed approach
- [Tauri 2 File System plugin](https://v2.tauri.app/plugin/file-system/) -- scope configuration for frontend access

### Tertiary (LOW confidence)
- Tauri latest version reported as 2.10.3 (from docs.rs, dated 2026-03-04) -- verify at scaffold time with `pnpm create tauri-app`

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- well-established Rust/Tauri ecosystem, versions verified from official sources
- Architecture: HIGH -- two-layer storage design is explicitly defined in PROJECT.md and validated by cc-switch's failure mode analysis
- Pitfalls: HIGH -- iCloud path handling and Tauri error serialization are well-documented issues
- Data model: MEDIUM -- Provider schema is a recommendation based on cc-switch reference + requirements; exact fields may need adjustment during implementation

**Research date:** 2026-03-10
**Valid until:** 2026-04-10 (stable ecosystem, 30-day validity)
