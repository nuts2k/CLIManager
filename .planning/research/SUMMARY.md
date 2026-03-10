# Project Research Summary

**Project:** CLIManager
**Domain:** Desktop AI CLI Configuration Manager (Tauri 2 + iCloud Sync)
**Researched:** 2026-03-10
**Confidence:** MEDIUM-HIGH

## Executive Summary

CLIManager is a macOS desktop application that manages provider configurations for AI CLI tools (Claude Code, Codex). The core problem it solves is twofold: (1) cc-switch's whole-file rewrite approach destroys user CLI settings on every provider switch, and (2) cc-switch's SQLite-in-iCloud-Drive architecture causes database corruption and state bounce across devices. The research strongly recommends building on Tauri 2 with React 18 frontend, using a **surgical Read-Modify-Write** patching strategy for CLI config files and a **per-provider flat JSON file** storage model in iCloud Drive. This combination directly addresses both root causes. The cc-switch reference codebase (v3.12.0) provides proven patterns for the stack (Tauri 2, TanStack Query, i18next, toml_edit, shadcn/ui) while its documented failures provide a clear anti-pattern catalog.

The recommended approach is a layered architecture with thin Tauri IPC commands delegating to fat service modules, a protocol-based provider model (Anthropic vs OpenAI-compatible rather than per-CLI), and a strict two-layer storage split: iCloud Drive for synced provider definitions (one JSON file per provider) and `~/.cli-manager/local.json` for device-specific state (active provider, locale, path overrides). CLI adapters implement a common trait and handle the per-tool specifics of reading and surgically patching config files. This architecture is inherently extensible -- adding a new CLI means adding one adapter module with zero changes to the switch logic, commands, or frontend.

The primary risks are: (1) the surgical Read-Modify-Write race condition with CLI tools concurrently modifying the same config file (accepted as low-probability with mitigations), (2) iCloud file eviction and FSEvents debouncing complexity, and (3) CLI config format changes breaking adapters silently. The first risk is deliberately accepted per project requirements. The second requires careful implementation of the file watcher with self-write suppression and debouncing. The third requires defensive parsing (dynamic `serde_json::Value`, not typed structs) and integration tests against real config samples. The research also identified a strong anti-feature list derived from cc-switch's bloat -- proxy, MCP, skills, sessions, and usage tracking are all explicitly excluded from scope.

## Key Findings

### Recommended Stack

The stack is anchored on Tauri 2 (Rust backend + WebView frontend), with all major technology choices validated against the cc-switch v3.12.0 reference project. React 18 is preferred over React 19 because Server Components and Concurrent Mode features are irrelevant in a Tauri desktop context. TailwindCSS 3.4 with shadcn/ui (copy-paste Radix-based components) provides macOS-native styling. The Rust backend uses `serde_json` for surgical JSON patching and `toml_edit` (not `toml`) for comment-preserving TOML patching -- this is a critical choice since the `toml` crate destroys formatting on round-trip.

**Core technologies:**
- **Tauri 2 (^2.8):** Desktop app shell -- project constraint, proven in cc-switch, native macOS FSEvents access from Rust
- **React 18 + TypeScript 5.6+:** Frontend framework -- stable, avoids React 19's irrelevant SSR features
- **TanStack Query v5:** Server-state management -- treats Rust backend as "server" with auto-refetch and cache invalidation
- **serde_json + toml_edit:** Surgical config patching -- read-modify-write that preserves all untouched fields and TOML comments
- **notify v7 + notify-debouncer-full:** File watching -- FSEvents on macOS, debounced to handle iCloud sync event storms
- **i18next + react-i18next:** i18n -- bundled JSON for Chinese (default) + English, proven pattern from cc-switch
- **shadcn/ui + Radix + TailwindCSS 3.4:** UI components -- lightweight, customizable, macOS-native aesthetic
- **tauri-plugin-store:** Device-local key-value settings only (NOT for provider data)
- **tauri-plugin-single-instance:** Prevent concurrent instances that could cause conflicting config writes

**What NOT to use:** SQLite (iCloud disaster), Electron, Redux/MobX (over-engineering), react-hook-form (premature), CodeMirror (unnecessary), reqwest/axum (no network features in v1), tauri-plugin-fs (too high-level for surgical operations).

### Expected Features

**Must have (table stakes):**
- Provider CRUD with flat JSON storage (one file per provider)
- One-click provider switching (< 1 second, zero friction)
- Surgical config patching via Read-Modify-Write (THE core value proposition)
- Claude Code adapter (patch `~/.claude/settings.json` -- env fields only)
- Codex adapter (two-file patch: `~/.codex/auth.json` + `config.toml` with rollback)
- Active provider indicator per CLI
- Error handling on config corruption (validate before write, never write invalid state)
- i18n from day one (Chinese default + English)

**Should have (differentiators over cc-switch):**
- iCloud Drive sync via per-provider JSON files (eliminates SQLite sync disaster)
- Data layer separation: synced provider defs vs device-local state (eliminates state bounce)
- File watching with FSEvents for live sync (UI auto-updates when another device changes providers)
- Active provider linkage on sync (auto re-patch CLI configs when synced provider changes)
- Auto-import on first launch (scan existing CLI configs, zero-config onboarding)
- Protocol-based provider modeling (Anthropic/OpenAI-compatible, not per-CLI)

**Defer (v2+):**
- MCP server management, prompts, skills management
- System tray quick-switch
- Additional CLIs (Gemini, OpenCode, OpenClaw)
- Local proxy / failover / usage tracking (possibly never)
- Deep link import, drag-to-reorder, provider categories/icons
- WebDAV / custom sync backends

### Architecture Approach

A layered architecture: React frontend with TanStack Query talks via Tauri IPC to thin Rust command handlers, which delegate to fat service modules (ProviderService, SyncService, WatcherService, ImportService, SettingsService). Services interact with a two-layer storage system (iCloud sync dir + local.json) and CLI-specific adapters implementing a common `CliAdapter` trait. The file watcher monitors the iCloud provider directory and emits Tauri events to the frontend for reactive cache invalidation.

**Major components:**
1. **Storage Layer** -- Two-layer file I/O: iCloud sync dir (`~/Library/Mobile Documents/.../CLIManager/providers/`) for per-provider JSON files, `~/.cli-manager/local.json` for device state
2. **CLI Adapters** -- Per-CLI Read-Modify-Write surgical patching via `CliAdapter` trait (Claude Code adapter, Codex adapter, extensible for future CLIs)
3. **SyncService** -- Orchestrates provider switch: load provider from storage, patch all applicable CLI adapters, update local active state
4. **WatcherService** -- FSEvents-based file watcher on iCloud sync dir with 200ms debounce, self-write suppression, and Tauri event emission
5. **Frontend Query Layer** -- TanStack Query hooks wrapping IPC calls, event-driven cache invalidation on backend state changes

### Critical Pitfalls

1. **Full-file rewrite destroys user settings** -- The number one cc-switch bug. Prevention: Read-Modify-Write with field allowlist; never serialize internal model directly to CLI config file; snapshot diff tests that assert non-target fields survive.
2. **iCloud sync of SQLite or device-local state** -- SQLite corruption, state bounce, half-file propagation all observed in cc-switch. Prevention: strict two-layer separation enforced from day one; per-provider JSON files only in iCloud; active provider stays in local.json.
3. **Multi-file config writes without transactional semantics** -- Codex requires auth.json + config.toml. Prevention: two-phase write with rollback; validate consistency on read; accept tiny crash-window risk.
4. **Read-Modify-Write race condition with CLI** -- TOCTOU race between CLIManager and CLI tool. Prevention: minimize read-write gap (tight sequence); re-read file fresh for every write; optional mtime check; do NOT add file locking.
5. **CLI config format changes break adapters silently** -- External CLIs can change schemas without notice. Prevention: defensive parsing with `serde_json::Value` (not typed structs); graceful degradation on missing fields; integration tests against real config samples.

## Implications for Roadmap

Based on combined research, the architecture has a clear dependency chain that dictates build order. The feature research and pitfall analysis both converge on the same phasing.

### Phase 1: Foundation and Data Model
**Rationale:** Everything depends on the storage layer and provider data model. Getting the two-layer split (iCloud vs local) and protocol-based provider modeling right is foundational. Errors here cascade into every subsequent phase.
**Delivers:** Rust storage module (read/write per-provider JSON files in iCloud dir + local.json), Provider data model with ProtocolType enum, path resolution utilities, error types, and project scaffolding (Tauri 2 + React 18 + all dependencies).
**Addresses:** Provider CRUD storage, data layer separation, protocol-based modeling.
**Avoids:** Pitfall 2 (iCloud sync of wrong data), Pitfall 9 (coupling provider to CLI instead of protocol), Pitfall 14 (hardcoded home directory).

### Phase 2: Surgical Patch Engine and CLI Adapters
**Rationale:** The surgical Read-Modify-Write is the project's raison d'etre and must be built and tested thoroughly before any UI work. The CliAdapter trait and two concrete adapters (Claude Code, Codex) form the core business logic.
**Delivers:** CliAdapter trait, Claude Code adapter (JSON surgical patch), Codex adapter (two-file JSON+TOML patch with rollback), SyncService (orchestrates switch), SettingsService.
**Addresses:** Surgical config patching, Claude Code adapter, Codex adapter, one-click switching (backend).
**Avoids:** Pitfall 1 (full-file rewrite), Pitfall 3 (multi-file partial failure), Pitfall 4 (race condition), Pitfall 5 (format changes), Pitfall 11 (TOML comment destruction).

### Phase 3: Frontend Shell and Provider Management UI
**Rationale:** With the backend complete, the frontend can wire up TanStack Query hooks to IPC commands. This phase delivers the first usable end-to-end experience.
**Delivers:** React UI with provider list, provider create/edit form, one-click switch button, active provider indicator, i18n setup (zh + en), settings page (locale, path overrides), toast notifications.
**Addresses:** Provider CRUD (UI), one-click switching (UI), active provider indicator, i18n, error handling UX.
**Avoids:** Pitfall 12 (API key exposure in frontend), Pitfall 13 (blocking main thread).

### Phase 4: iCloud Sync and File Watching
**Rationale:** Cross-device sync is the second major value proposition but requires the core CRUD and switching to work first. File watching and reactive refresh depend on having a working UI to refresh.
**Delivers:** WatcherService (FSEvents + debounce), Tauri event emission for provider changes, frontend event listeners (TanStack Query cache invalidation), iCloud file eviction handling, active provider linkage on sync (auto re-patch when synced provider changes).
**Addresses:** File watching for live sync, active provider linkage, iCloud Drive sync.
**Avoids:** Pitfall 6 (FSEvents infinite loops), Pitfall 7 (evicted files), Pitfall 10 (write surface amplification).

### Phase 5: Onboarding and Polish
**Rationale:** Auto-import and error recovery improve the first-run experience but are not blocking daily use. Can be built after the core loop is solid.
**Delivers:** ImportService (first-launch scan of existing CLI configs), config backup before writes, import preview UI, error recovery flows.
**Addresses:** Auto-import on first launch, error recovery, config backup.
**Avoids:** Pitfall 8 (import corrupts existing config).

### Phase Ordering Rationale

- **Bottom-up build order mirrors architecture layers:** Storage -> Services -> Commands -> Frontend -> Reactive features. Each layer has its foundation before the layer above starts.
- **Pitfall-driven sequencing:** The five critical pitfalls all relate to Phases 1-2 (data model and surgical patch). Solving these first de-risks the entire project.
- **Feature dependency chain:** Provider CRUD -> CLI Adapters -> Switching -> UI -> File Watching -> Auto-Import. This matches both FEATURES.md's critical path and ARCHITECTURE.md's suggested build order.
- **i18n must be wired in Phase 3** (not deferred) -- FEATURES.md and PITFALLS.md both flag retrofitting i18n as costly. The i18n framework is set up when the first UI components are built.
- **iCloud sync (Phase 4) is deliberately after the core loop (Phases 1-3)** even though it is a key differentiator, because: (a) the core loop must work locally first, (b) file watching adds significant complexity that should not block initial usability, (c) the storage format (per-provider JSON) is designed for iCloud from Phase 1 so no rework is needed.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 2 (CLI Adapters):** Needs research into exact Claude Code and Codex config file schemas, field paths, and edge cases. The `settings.json` env-var-based configuration pattern needs validation against current Claude Code versions. Codex two-file write ordering and rollback semantics need detailed specification.
- **Phase 4 (iCloud Sync):** iCloud file eviction detection (`NSURLUbiquitousItemIsDownloadedKey`), placeholder `.icloud` file handling, and atomic rename behavior in iCloud directories need macOS-specific API research. The `notify` crate's interaction with iCloud's extended attributes needs testing.

Phases with standard patterns (skip research-phase):
- **Phase 1 (Foundation):** Tauri 2 scaffolding, JSON file I/O, data model design -- well-documented, cc-switch provides direct reference code.
- **Phase 3 (Frontend):** React + TanStack Query + shadcn/ui + i18next -- all thoroughly documented, cc-switch demonstrates the exact pattern.
- **Phase 5 (Onboarding):** Auto-import is a read-only operation with no novel technical challenges.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Nearly every technology validated against cc-switch v3.12.0 reference code. Only `notify` v7 and `zod` v4 have version uncertainty (fallbacks identified). |
| Features | MEDIUM-HIGH | Feature list grounded in cc-switch analysis and PROJECT.md requirements. Competitive positioning relative to tools outside cc-switch is LOW (no web search). |
| Architecture | HIGH | Layered architecture, adapter pattern, two-layer storage, and event-driven refresh are all proven patterns. cc-switch provides direct reference for the Tauri-specific integration. |
| Pitfalls | HIGH | All five critical pitfalls directly observed in cc-switch source code with specific file/line references. Prevention strategies are concrete and actionable. |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **Claude Code config schema validation:** The exact field paths in `~/.claude/settings.json` (env-based vs top-level) need verification against the current Claude Code version at build time. Research used cc-switch's adapter code as reference, but Claude Code may have changed its schema.
- **Codex config schema validation:** Same concern for `~/.codex/auth.json` and `config.toml`. Field names and structure should be verified against current Codex documentation.
- **notify crate version:** Research recommends v7 but noted it may not be released yet. Fallback to v6 with `notify-debouncer-full` v0.3 is identified. Verify on crates.io at project start.
- **iCloud Drive APIs from Rust:** Checking file eviction status requires macOS `NSFileManager` APIs. Calling these from Rust may require `objc2` bindings or a Swift bridge. This needs investigation during Phase 4 planning.
- **TailwindCSS 3 vs 4:** Research recommends TailwindCSS 3.4 for shadcn/ui compatibility. If shadcn/ui v2 has stabilized on TailwindCSS 4 by project start, reconsider.
- **Competitive landscape:** No web search was available. Other tools in this niche (beyond cc-switch) may exist. This does not affect architecture but may affect feature prioritization.

## Sources

### Primary (HIGH confidence)
- cc-switch v3.12.0 source code (`cc-switch/` reference directory) -- stack versions, architecture patterns, adapter implementations, observed bugs
- cc-switch `package.json` and `Cargo.toml` -- exact dependency versions
- `icloud-sync-root-cause-zh.md` -- detailed iCloud sync failure analysis with SQLite corruption specifics
- `cc-switch-ref-notes-zh.md` -- feature breakdown and architectural observations
- `.planning/PROJECT.md` -- project requirements and design constraints

### Secondary (MEDIUM confidence)
- Claude training data (May 2025 cutoff) -- Tauri 2 architecture, `notify` crate API, Rust ecosystem conventions, iCloud Drive behavior
- cc-switch i18n setup (`src/i18n/index.ts`) -- bundled JSON i18n pattern
- cc-switch TOML utilities and `toml_edit` usage -- surgical TOML patching reference

### Tertiary (LOW confidence)
- `notify` v7 version availability -- may still be v6 at project start; verify on crates.io
- `zod` v4 stability -- cc-switch uses ^4.1; may want ^3.23 for stability
- iCloud file eviction API specifics -- Rust bindings for NSFileManager need investigation

---
*Research completed: 2026-03-10*
*Ready for roadmap: yes*
