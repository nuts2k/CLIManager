# Technology Stack

**Project:** CLIManager
**Researched:** 2026-03-10
**Overall Confidence:** MEDIUM-HIGH (versions derived from reference project cc-switch v3.12.0 + training data; no live registry verification available during research)

## Recommended Stack

### Core Framework

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Tauri 2 | ^2.8 | Desktop app shell (Rust backend + WebView frontend) | PROJECT.md constraint. Proven in cc-switch. Native perf, small binary, macOS FSEvents access from Rust. | HIGH |
| React | ^18.2 | Frontend UI framework | Stick with React 18, not 19. React 19's Server Components and use() hooks are irrelevant to Tauri (no SSR). React 18 is stable, battle-tested in cc-switch, and avoids the React 19 Concurrent Mode edge cases that can cause double-render issues in desktop apps. | HIGH |
| TypeScript | ^5.6 | Type safety for frontend | cc-switch uses ^5.3. Bump to 5.6+ for satisfies keyword improvements and better type narrowing. | HIGH |
| Rust | 1.85+ (edition 2021) | Backend logic, file I/O, config patching | PROJECT.md constraint via Tauri 2. Edition 2021 matches cc-switch. | HIGH |

### Build Tooling

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Vite | ^6.0 or ^7.0 | Frontend bundler + dev server | cc-switch uses ^7.3.0. Use whatever `npm create tauri-app` scaffolds. Vite is the default Tauri 2 frontend bundler -- fast HMR, simple config. | HIGH |
| @vitejs/plugin-react | ^4.2 | React JSX transform for Vite | Standard pairing. SWC-backed for speed. | HIGH |
| @tauri-apps/cli | ^2.8 | Tauri CLI for dev/build | Must match @tauri-apps/api version. | HIGH |
| pnpm | ^10 | Package manager | cc-switch uses pnpm. Faster installs, strict dependency isolation, works well with Tauri monorepo-ish structure. | HIGH |

### Frontend Libraries

| Library | Version | Purpose | Why | Confidence |
|---------|---------|---------|-----|------------|
| @tauri-apps/api | ^2.8 | Tauri IPC bridge (invoke, events, window) | Core Tauri integration. Version must match CLI/core. | HIGH |
| @tanstack/react-query | ^5 | Server state management (IPC data fetching + cache) | Treats Rust backend as "server". Auto-refetch, cache invalidation, optimistic updates. Proven pattern in cc-switch for provider CRUD. | HIGH |
| TailwindCSS | ^3.4 | Utility-first CSS | cc-switch uses 3.4 with shadcn/ui CSS variable pattern. TailwindCSS 4 (released 2025) has breaking changes in config format -- stick with v3 for stability and shadcn/ui compatibility. | MEDIUM |
| shadcn/ui components | (copy-paste, not versioned) | UI component library | Not an npm dependency -- copy-paste Radix-based components. macOS-native feel with proper dark mode. cc-switch uses this exact pattern (Radix primitives + Tailwind). | HIGH |
| Radix UI primitives | ^1.x-^2.x | Accessible headless UI components | Foundation for shadcn/ui. Dialog, Select, Switch, Tooltip, etc. Only install primitives you actually use. | HIGH |
| lucide-react | ^0.540+ | Icon library | Clean, consistent icons. Used by cc-switch. Light tree-shakeable. | HIGH |
| i18next | ^25 | i18n core | Mature, well-documented. cc-switch uses this exact setup with bundled JSON translation files. Simple for 2-language (zh+en) scope. | HIGH |
| react-i18next | ^16 | React bindings for i18next | useTranslation hook pattern. Proven in cc-switch. | HIGH |
| zod | ^3.23 or ^4 | Runtime schema validation | Validate provider configs, form inputs. cc-switch uses ^4.1 (Zod 4). Use ^3.23 for stability or ^4 if already stable at build time. | MEDIUM |
| sonner | ^2.0 | Toast notifications | Lightweight, good DX. Used by cc-switch. Better than react-hot-toast for Tauri. | HIGH |
| class-variance-authority | ^0.7 | Component variant management | Standard shadcn/ui companion. Pairs with clsx + tailwind-merge. | HIGH |
| clsx | ^2.1 | Conditional className utility | Standard. Tiny. | HIGH |
| tailwind-merge | ^3.3 | Tailwind class conflict resolution | Standard shadcn/ui companion. cn() = clsx + tailwind-merge. | HIGH |
| framer-motion | ^12 | Animations | Smooth list transitions for provider cards. Optional but improves UX significantly for CRUD operations. | MEDIUM |

### Rust Backend Crates

| Crate | Version | Purpose | Why | Confidence |
|-------|---------|---------|-----|------------|
| tauri | ^2.8 | Core Tauri runtime | Must match frontend @tauri-apps/api. | HIGH |
| tauri-build | ^2.4 | Build-time Tauri integration | Standard Tauri build dependency. | HIGH |
| serde | 1.0 + derive | Serialization framework | Universal Rust serialization. Every struct needs it. | HIGH |
| serde_json | 1.0 | JSON read/write | Provider JSON files, Claude Code settings.json, Codex auth.json. Core to surgical patch: deserialize, modify field, serialize. | HIGH |
| toml_edit | 0.22 | **Surgical TOML editing** | CRITICAL choice. `toml` crate loses formatting/comments on round-trip. `toml_edit` preserves document structure -- parse to DocumentMut, modify specific keys, serialize back without touching other content. Exactly what cc-switch uses for Codex config.toml patching. | HIGH |
| toml | 0.8 | TOML deserialization (read-only) | Use for typed deserialization when you need structured access. Use `toml_edit` for write-back. | HIGH |
| notify | ^7.0 | Cross-platform file system watching | Uses FSEvents on macOS natively. Watches iCloud Drive directory for provider file changes. Debounced events via `notify-debouncer-full` or manual debounce. This is the standard Rust file watcher -- no viable alternative. | MEDIUM |
| notify-debouncer-full | ^0.4 | Debounced file watcher | Wraps notify with configurable debounce. Prevents event storms from iCloud sync (multiple rapid writes). | MEDIUM |
| tokio | 1 (macros, rt-multi-thread, sync, time) | Async runtime | Tauri 2 uses tokio internally. Needed for async file ops, debounced watchers, channels. | HIGH |
| dirs | 5.0 | Platform-standard directories | `dirs::home_dir()` for `~/.claude/`, `~/.codex/`, etc. Cross-platform path resolution. | HIGH |
| uuid | 1.11 (v4) | Unique IDs for providers | Each provider JSON file needs a unique identifier. UUIDv4 is simple and sufficient. | HIGH |
| chrono | 0.4 (serde) | Timestamps | Provider created_at/updated_at fields. Serde integration for JSON serialization. | HIGH |
| thiserror | 2.0 | Error types | Derive Error for structured error handling. cc-switch uses this. Cleaner than manual impl. | HIGH |
| anyhow | 1.0 | Error propagation in commands | For Tauri command return types where you want ? chaining without custom error types. Use thiserror for library code, anyhow for command handlers. | HIGH |
| indexmap | 2 (serde) | Order-preserving maps | Preserve key order in JSON config files during surgical patch. Important: serde_json's Map is insertion-ordered, but if you need explicit ordering guarantees, indexmap is safer. | MEDIUM |
| log | 0.4 | Logging facade | Standard Rust logging. Pairs with tauri-plugin-log. | HIGH |

### Tauri Plugins

| Plugin | Crate Version | Purpose | Why | Confidence |
|--------|---------------|---------|-----|------------|
| tauri-plugin-log | 2 | Logging | Route Rust logs to frontend console + log files. Essential for debugging. | HIGH |
| tauri-plugin-dialog | 2 | Native dialogs | File picker for config import, confirmation dialogs. | HIGH |
| tauri-plugin-process | 2 | Process management | App restart after settings changes. | MEDIUM |
| tauri-plugin-store | 2 | Key-value storage | Device-local settings (current active provider, window position, language preference). NOT for provider data. | HIGH |
| tauri-plugin-single-instance | 2 | Prevent multiple instances | Only one CLIManager should run at a time to avoid conflicting config writes. | HIGH |

### Dev Dependencies (Frontend)

| Library | Version | Purpose | Why | Confidence |
|---------|---------|---------|-----|------------|
| vitest | ^2.0 | Unit testing | Fast, Vite-native. cc-switch uses this. | HIGH |
| @testing-library/react | ^16 | React component testing | Standard React testing. | HIGH |
| prettier | ^3.6 | Code formatting | Consistent style. | HIGH |

### Dev Dependencies (Rust)

| Crate | Version | Purpose | Why | Confidence |
|-------|---------|---------|-----|------------|
| tempfile | 3 | Temp dirs/files for testing | Test file watcher, config patching without touching real configs. | HIGH |
| serial_test | 3 | Sequential test execution | File system tests that can't run in parallel. | HIGH |

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Frontend framework | React 18 | React 19 | Server Components irrelevant in Tauri. use() hook not needed. Risk of double-render from Strict Mode in Concurrent features. Ecosystem (shadcn/ui, TanStack Query) most tested against React 18. |
| Frontend framework | React | Solid.js / Svelte | PROJECT.md specifies React. cc-switch uses React. Largest ecosystem, easiest to hire/maintain. |
| CSS framework | TailwindCSS 3 | TailwindCSS 4 | TW4 rewrites config format (CSS-based), breaking shadcn/ui v0 patterns. Migrate later when shadcn/ui v2 stabilizes on TW4. |
| Component library | shadcn/ui (copy-paste) | Ant Design / MUI | shadcn/ui is lighter, more customizable, macOS-native feel. Ant/MUI are heavy, opinionated, hard to match macOS aesthetics. |
| State management | TanStack Query | Zustand / Redux | TanStack Query handles "server state" (IPC calls) perfectly. No need for global client state manager -- React context + useReducer handles the minimal UI state. |
| i18n | i18next | react-intl / LinguiJS | i18next is proven in cc-switch, largest ecosystem, simplest bundled-JSON setup for 2 languages. |
| TOML editing | toml_edit | toml crate | `toml` crate loses comments and formatting on round-trip. `toml_edit` preserves document structure -- essential for surgical patching. |
| File watching | notify 7 | inotify/FSEvents directly | `notify` abstracts platform differences. FSEvents-specific code would lock us to macOS with no future portability. |
| File watching | notify (Rust) | chokidar (JS) | File watching must happen in Rust backend (closer to OS, no WebView overhead, can run before/after window is visible). |
| Data storage | JSON files (per-provider) | SQLite | PROJECT.md explicitly excludes SQLite. JSON files are iCloud-safe (no lock conflicts), human-readable, easy to debug. |
| Data storage | JSON files | tauri-plugin-store | plugin-store is for device-local KV settings only. Provider data needs individual files for iCloud sync granularity. |
| TOML parsing (frontend) | smol-toml | @iarna/toml | smol-toml is smaller, ESM-native, actively maintained. Used by cc-switch for frontend TOML display/validation. |
| Package manager | pnpm | npm / yarn | pnpm is faster, stricter (no phantom dependencies), better monorepo support. cc-switch uses it. |
| Animations | framer-motion | CSS transitions | framer-motion handles list reorder animations (AnimatePresence) that pure CSS cannot. Worth the 30KB for provider list UX. |
| Error handling | thiserror + anyhow | manual Error impl | thiserror for library types, anyhow for command handlers. Standard Rust pattern, minimal boilerplate. |

## What NOT to Use

| Technology | Reason |
|------------|--------|
| SQLite / rusqlite | Explicit project decision. SQLite in iCloud Drive is a known disaster (see icloud-sync-root-cause-zh.md). |
| Electron | Tauri 2 is the project constraint. Electron would be 10x larger binary. |
| Redux / MobX / Jotai | Over-engineering for this app's state needs. TanStack Query + React context is sufficient. |
| Next.js / Remix | SSR frameworks are meaningless in Tauri. |
| tauri-plugin-fs | CLIManager needs surgical file operations (read-modify-write specific fields). The plugin-fs API is too high-level. Use direct Rust std::fs for precise control. |
| tauri-plugin-updater | Not needed for v1 MVP. Add when distributing outside dev machine. |
| axum / hyper / tower | cc-switch uses these for its local proxy server. CLIManager has no proxy feature -- strip them. |
| reqwest | No HTTP requests needed in v1 (no proxy, no WebDAV, no API calls). |
| rquickjs | cc-switch uses this for JS eval in proxy. Not needed. |
| zip / serde_yaml | cc-switch uses for skill installation. Not in scope. |
| CodeMirror | cc-switch uses for config editor. CLIManager does simple form-based editing, not raw config editing. |
| recharts | cc-switch uses for usage statistics. Not in scope. |
| react-hook-form | Over-engineering for the simple Provider forms in CLIManager. Use controlled components + Zod validation. Add later if forms get complex. |
| dnd-kit | cc-switch uses for provider reorder drag. Not MVP -- add in v2 if users want custom ordering. |

## Architecture-Significant Stack Decisions

### 1. Surgical JSON Patching Strategy (Rust)

Use `serde_json::Value` as the intermediate representation for read-modify-write:

```rust
// Read existing config
let content = std::fs::read_to_string(&config_path)?;
let mut doc: serde_json::Value = serde_json::from_str(&content)?;

// Surgical patch -- only modify target fields
if let Some(obj) = doc.as_object_mut() {
    obj.insert("apiKey".to_string(), serde_json::Value::String(new_key));
    obj.insert("model".to_string(), serde_json::Value::String(new_model));
    // All other fields preserved
}

// Write back
let output = serde_json::to_string_pretty(&doc)?;
std::fs::write(&config_path, output)?;
```

Note: `serde_json::Value` preserves all fields but does NOT preserve key order or formatting. For JSON configs this is acceptable (JSON spec says objects are unordered). If comment preservation matters (e.g., JSONC), use `jsonc-parser` on the frontend or a Rust JSONC crate.

### 2. Surgical TOML Patching Strategy (Rust)

Use `toml_edit::DocumentMut` to preserve formatting and comments:

```rust
use toml_edit::DocumentMut;

let content = std::fs::read_to_string(&config_path)?;
let mut doc: DocumentMut = content.parse()?;

// Surgical patch -- preserves all formatting, comments, other keys
doc["model"] = toml_edit::value("new-model-name");
doc["api_key"] = toml_edit::value("sk-xxx");

std::fs::write(&config_path, doc.to_string())?;
```

### 3. File Watcher Architecture (Rust)

```rust
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};

// Watch iCloud Drive provider directory
let (tx, rx) = std::sync::mpsc::channel();
let mut debouncer = new_debouncer(
    Duration::from_millis(500), // 500ms debounce for iCloud sync storms
    None,
    tx,
)?;
debouncer.watcher().watch(&icloud_provider_dir, RecursiveMode::NonRecursive)?;

// In a tokio task, receive events and emit to frontend
for result in rx {
    match result {
        Ok(events) => {
            // Filter for .json file changes
            // Emit Tauri event to frontend for UI refresh
            app_handle.emit("providers-changed", payload)?;
        }
        Err(errors) => { /* log */ }
    }
}
```

### 4. Data Storage Layout

```
# iCloud Drive (synced) -- one file per provider
~/Library/Mobile Documents/com~apple~CloudDocs/CLIManager/
  providers/
    {uuid}.json          # One provider per file
  active-provider.json   # Which provider is currently active (per-device override below)

# Device-local (NOT synced)
~/.cli-manager/
  device-settings.json   # Current active provider for THIS device, language, window state
```

### 5. i18n Setup (Frontend)

Follow cc-switch's proven pattern -- bundled JSON, no async loading:

```
src/
  i18n/
    index.ts             # i18n.init() with bundled resources
    locales/
      zh.json            # Chinese (default)
      en.json            # English (fallback)
```

Two languages with ~200 keys each. No need for lazy loading or namespace splitting at this scale. Store language preference in `tauri-plugin-store` (device-local).

## Installation

```bash
# Scaffold Tauri 2 + React + TypeScript project
pnpm create tauri-app cli-manager --template react-ts

# Frontend core
pnpm add @tauri-apps/api @tauri-apps/plugin-dialog @tauri-apps/plugin-process @tauri-apps/plugin-store
pnpm add @tanstack/react-query
pnpm add i18next react-i18next
pnpm add zod
pnpm add sonner
pnpm add lucide-react
pnpm add clsx tailwind-merge class-variance-authority
pnpm add framer-motion

# Frontend dev
pnpm add -D @tauri-apps/cli
pnpm add -D tailwindcss@^3.4 postcss autoprefixer
pnpm add -D vitest @testing-library/react @testing-library/jest-dom
pnpm add -D prettier

# shadcn/ui -- init then add components as needed
pnpm dlx shadcn-ui@latest init
pnpm dlx shadcn-ui@latest add button dialog input label select switch tabs toast
```

```toml
# Cargo.toml [dependencies]
tauri = { version = "2.8", features = ["tray-icon"] }
tauri-plugin-log = "2"
tauri-plugin-dialog = "2"
tauri-plugin-process = "2"
tauri-plugin-store = "2"
tauri-plugin-single-instance = "2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
toml_edit = "0.22"
notify = "7"
notify-debouncer-full = "0.4"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "sync", "time"] }
dirs = "5.0"
uuid = { version = "1.11", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2.0"
anyhow = "1.0"
log = "0.4"
indexmap = { version = "2", features = ["serde"] }

[build-dependencies]
tauri-build = { version = "2.4", features = [] }

[dev-dependencies]
tempfile = "3"
serial_test = "3"
```

## Version Verification Notes

| Item | Source | Confidence | Notes |
|------|--------|------------|-------|
| Tauri 2.8.x | cc-switch Cargo.toml + package.json (actively maintained, v3.12.0) | HIGH | Real dependency from working project |
| React 18.2 | cc-switch package.json | HIGH | Deliberately not React 19 |
| Vite ^7.3 | cc-switch package.json | MEDIUM | cc-switch uses ^7.3.0; verify scaffolded version |
| TailwindCSS 3.4 | cc-switch package.json | HIGH | Explicitly avoid TW4 for shadcn compat |
| i18next ^25 | cc-switch package.json | HIGH | Working i18n implementation exists in reference |
| toml_edit 0.22 | cc-switch Cargo.toml | HIGH | Proven surgical TOML editing in production |
| notify 7.x | Training data (May 2025 knowledge cutoff) | MEDIUM | notify 6 was stable; 7.0 may or may not be released. Fallback: `notify = "6"` with `notify-debouncer-full = "0.3"`. Verify on crates.io at build time. |
| zod ^4 | cc-switch package.json uses ^4.1.12 | MEDIUM | Zod 4 was released mid-2025. Verify stability. Fallback: ^3.23. |

## Sources

- cc-switch `package.json` (v3.12.0) -- `/Users/kelin/Workspace/CLIManager/cc-switch/package.json`
- cc-switch `Cargo.toml` -- `/Users/kelin/Workspace/CLIManager/cc-switch/src-tauri/Cargo.toml`
- cc-switch i18n setup -- `/Users/kelin/Workspace/CLIManager/cc-switch/src/i18n/index.ts`
- cc-switch TOML utils -- `/Users/kelin/Workspace/CLIManager/cc-switch/src/utils/tomlUtils.ts`
- cc-switch toml_edit usage -- `cc-switch/src-tauri/src/services/provider/live.rs`
- PROJECT.md constraints -- `/Users/kelin/Workspace/CLIManager/.planning/PROJECT.md`
- iCloud sync root cause analysis -- `/Users/kelin/Workspace/CLIManager/icloud-sync-root-cause-zh.md`
- cc-switch reference notes -- `/Users/kelin/Workspace/CLIManager/cc-switch-ref-notes-zh.md`
- Training data (Claude, May 2025 cutoff) -- Used for: notify crate version, notify-debouncer-full, general Rust ecosystem knowledge
