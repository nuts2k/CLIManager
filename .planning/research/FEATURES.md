# Feature Landscape

**Domain:** AI CLI Configuration Management (Provider switching for Claude Code, Codex, and future AI CLIs)
**Researched:** 2026-03-10
**Overall confidence:** MEDIUM-HIGH (based on cc-switch reference analysis, PROJECT.md requirements, and domain knowledge of AI CLI tooling)

## Table Stakes

Features users expect. Missing = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Provider CRUD | Core data management -- users need to create, view, edit, and delete provider configs. Without this, nothing else works. | Low | Standard form-based UI. Fields: name, API key, base URL, model. Keep minimal -- avoid cc-switch's 20+ meta fields. |
| One-click provider switching | The entire raison d'etre. Users switch providers dozens of times per day. Must be < 1 second, zero friction. | Medium | This is where surgical patch happens. Read current config, modify only credential/model fields, write back. |
| Surgical config patching | cc-switch's atomic_write replaces entire files, destroying user's other CLI settings. This is THE differentiator that justifies CLIManager's existence. | High | Read-Modify-Write for JSON (Claude Code settings.json) and JSON+TOML (Codex auth.json + config.toml). Must parse, patch specific keys, preserve everything else including comments in TOML. |
| Claude Code adapter | Claude Code is the most popular AI CLI. Not supporting it means losing majority of target users. | Medium | Patch `~/.claude/settings.json` -- modify `apiKey`, `model`, and related credential fields only. Handle legacy `claude.json` fallback. |
| Codex adapter | Codex is the second major AI CLI. Two-file config (auth.json + config.toml) requires coordinated writes. | Medium | Two-phase write with rollback: write auth.json first, then config.toml. If second fails, rollback first. |
| Active provider indicator | Users must see at a glance which provider is currently active for each CLI. Ambiguity = anxiety about billing. | Low | Simple UI state: highlight active provider card, show in header/badge. |
| Error handling on config corruption | If config read/parse fails, user must know immediately and not lose data. Silent failures = trust destruction. | Medium | Validate config before and after patch. If parse fails, show clear error, do NOT write. Backup original before first write. |
| i18n (Chinese + English) | Target user base is heavily Chinese-speaking (cc-switch community). English for global reach. | Medium | Must be baked in from v1. Retrofitting i18n is painful. Default Chinese, switchable to English. |

## Differentiators

Features that set product apart. Not expected (cc-switch doesn't do them well), but highly valued.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| iCloud Drive sync (per-provider files) | Cross-device provider sharing without the SQLite-in-iCloud disaster cc-switch has. Each provider = one JSON file in iCloud Drive dir. No DB sync, no conflict hell. | High | Core architectural differentiator. Design: `~/Library/Mobile Documents/com~apple~CloudDocs/CLIManager/providers/{id}.json`. Single-file granularity means iCloud handles conflicts at provider level, not whole-DB level. |
| Data layer separation (sync vs local) | Device-specific settings (active provider, path overrides) stay local in `~/.cli-manager/`. Provider definitions sync via iCloud. Prevents the "two Macs overwriting each other's active provider" problem. | Medium | Two clear directories: iCloud sync dir for provider data, local dir for device state. Clean separation cc-switch failed to achieve. |
| File watching for live sync | When iCloud syncs a provider change from another device, UI updates automatically. If the active provider was modified, re-patch CLI configs automatically. | Medium | FSEvents on macOS. Watch the iCloud sync directory. Debounce events (iCloud can emit multiple events per file). |
| Auto-import on first launch | Scan existing CLI configs and create initial providers. Zero-config onboarding -- user installs, opens, and their current setup is already there. | Medium | Read `~/.claude/settings.json` and `~/.codex/auth.json` + `config.toml`, extract credentials + model, create provider entries. Must handle missing/partial configs gracefully. |
| Protocol-based provider modeling | Providers bind to API protocol type (Anthropic, OpenAI-compatible) not to a specific CLI. When new CLIs are added, providers are reused -- just add an adapter. | Medium | Future-proofing design. A provider with protocol=anthropic works for Claude Code today and any future Anthropic-compatible CLI. Reduces duplicate provider entries across CLIs. |
| Active provider linkage on sync | When iCloud syncs a change to the currently active provider (e.g., API key rotation on another device), automatically re-patch CLI configs with updated credentials. | Medium | Combines file watching + surgical patch. Critical for the "set it and forget it" cross-device workflow. |
| Lightweight / fast startup | cc-switch starts with DB migrations, vacuum, cleanup -- visible startup lag. CLIManager with flat JSON files should launch near-instantly. | Low | No SQLite, no migrations, no startup maintenance writes. Just read JSON files from disk. Flat file storage is inherently faster to cold-start. |

## Anti-Features

Features to explicitly NOT build. Each is a lesson learned from cc-switch's feature bloat.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| MCP server management | Scope creep. cc-switch's MCP module added significant complexity (separate DB table, per-app toggling, import/export). Provider management is the core value; MCP is a separate concern. | Defer to v2 milestone. Surgical patch already preserves MCP config in CLI files by not touching those fields. |
| Prompts / Skills management | Same as MCP -- cc-switch bolted these on and they became maintenance burdens. Skills alone has GitHub/ZIP install, symlink/copy sync, legacy migration. | Completely out of scope. These are editor/IDE concerns, not provider config concerns. |
| Local proxy / failover / usage tracking | The single biggest source of complexity in cc-switch. proxy_config alone has 50+ schema fields. Failover queues, circuit breakers, stream health checks, request logging, pricing tables -- enormous surface area for marginal value in a config switcher. | Explicitly exclude. If users need proxy/failover, they should use dedicated proxy tools. CLIManager does one thing well: switch providers. |
| SQLite as data store | cc-switch's SQLite DB is the root cause of iCloud sync failures. DB in a sync directory = classic disaster. Migrations add startup complexity. | Use flat JSON files. One file per provider in iCloud sync dir. One local JSON for device settings. No schema migrations ever. |
| WebDAV / custom sync backends | Adds configuration complexity, auth management, error handling for network failures. iCloud Drive "just works" on macOS. | iCloud Drive only for v1. The architecture (flat files in a directory) naturally supports other sync backends later if needed, but don't build the abstraction now. |
| Session manager | Weak coupling with provider switching. cc-switch's session manager scans Codex/Claude Code local session files -- interesting but orthogonal to the config management mission. | Out of scope entirely. |
| Deep link import (ccswitch://) | Nice for sharing but not core. Adds URL scheme registration, parsing, security considerations. | Defer. Manual provider creation and auto-import cover the onboarding story. |
| System tray quick-switch | Adds native menu management, background process lifetime concerns, "minimize to tray" UX decisions. Valuable but not MVP. | v2 feature. The main window with one-click switching is sufficient for v1. |
| Whole-file atomic write (tmp + rename) | cc-switch's `atomic_write` writes a temp file then renames -- this replaces the entire file, destroying user's other settings. It's "atomic" but destructive. | Surgical Read-Modify-Write. Read the current file, parse it, modify only target fields, write back. Accept the tiny race condition (CLI writing at exact same moment) because the alternative (file locking) adds disproportionate complexity. |
| Universal Provider (cross-app abstraction) | cc-switch added this late -- a single provider entry that auto-generates per-CLI configs. Sounds elegant but adds a complex mapping layer and confuses users about what's actually written to each CLI. | Protocol-based modeling achieves reusability without the abstraction tax. A provider has a protocol type; each CLI adapter knows how to write that protocol's credentials. No "universal" indirection. |
| Provider categories / icons / partner badges | cc-switch has 8 provider categories (official, cn_official, cloud_provider, aggregator, third_party, custom, omo, omo-slim), icon pickers, partner promotion keys. This is commercial platform complexity, not config management. | Simple name + optional notes. No categorization, no icon system, no partner flags. Users have 3-10 providers, not hundreds. |
| Usage query scripts | cc-switch embeds JavaScript usage-query scripts per provider to check API balance/quota. Requires a script execution engine, timeout management, template system. | Out of scope. Users can check usage in their provider's web dashboard. |
| Endpoint speed testing | cc-switch has per-provider endpoint latency testing with custom endpoint management. | Out of scope. If an endpoint is slow, users will notice from CLI behavior. |

## Feature Dependencies

```
Provider CRUD ──────────────────┐
                                v
                         Provider Storage (flat JSON files)
                                │
                 ┌──────────────┼──────────────┐
                 v              v              v
         Claude Code      Codex Adapter    [Future CLI
          Adapter                           Adapters]
                 │              │
                 v              v
            Surgical Patch Engine
            (Read-Modify-Write)
                      │
                      v
              Provider Switching
              (one-click activate)
                      │
          ┌───────────┴───────────┐
          v                       v
   Active Provider          iCloud Sync Layer
   Indicator (UI)         (per-provider JSON files)
                                  │
                           ┌──────┴──────┐
                           v             v
                     File Watching    Data Layer
                     (FSEvents)      Separation
                           │        (sync vs local)
                           v
                    Active Provider
                    Linkage on Sync
                    (auto re-patch)

Auto-import ──> Provider CRUD (creates providers from existing CLI configs)
i18n ──> All UI components (must be wired in from start)
```

### Critical Path

1. **Provider Storage** must exist before anything else
2. **Surgical Patch Engine** must work before switching is useful
3. **CLI Adapters** depend on the patch engine and are CLI-specific
4. **iCloud Sync Layer** is independent of switching but depends on storage format
5. **File Watching** depends on iCloud sync layer being in place
6. **Active Provider Linkage** depends on both file watching and switching

### Parallel Work Possible

- i18n setup can happen alongside any feature
- iCloud sync layer can be built in parallel with CLI adapters
- Auto-import can be built after adapters exist (reads same configs adapters write)

## MVP Recommendation

### Phase 1: Core Loop (must ship together)

1. **Provider CRUD** -- Create, read, edit, delete providers with flat JSON storage
2. **Claude Code adapter** -- Surgical patch to `~/.claude/settings.json`
3. **Codex adapter** -- Surgical patch to `~/.codex/auth.json` + `config.toml`
4. **One-click switching** -- Select provider, patch CLI configs, update active indicator
5. **i18n foundation** -- Wire in from day one (zh + en)

Rationale: This is the minimum that delivers the core value proposition. A user can manage providers and switch between them without destroying their CLI settings.

### Phase 2: Cross-Device (the iCloud story)

6. **iCloud sync layer** -- Per-provider JSON files in iCloud Drive
7. **Data layer separation** -- Device-local settings vs synced provider data
8. **File watching** -- FSEvents on iCloud sync directory
9. **Active provider linkage** -- Auto re-patch when synced provider changes

Rationale: Cross-device sync is the second major value prop but requires the core loop to work first.

### Phase 3: Onboarding Polish

10. **Auto-import on first launch** -- Scan existing configs, create providers
11. **Error recovery / config backup** -- Safety net before writes

Rationale: Nice-to-have for onboarding but not blocking daily use.

### Defer Entirely

- MCP management, prompts, skills (v2+ milestones)
- System tray (v2)
- Additional CLI support: Gemini, OpenCode, OpenClaw (v2)
- Local proxy / failover / usage (never, unless user demand proves otherwise)

## Sources

- cc-switch source code analysis (`cc-switch/` reference directory, read-only)
- `cc-switch-ref-notes-zh.md` -- detailed feature breakdown of cc-switch
- `icloud-sync-root-cause-zh.md` -- iCloud sync failure analysis
- `.planning/PROJECT.md` -- CLIManager project requirements and design decisions
- cc-switch type definitions (`cc-switch/src/types.ts`) -- reveals full feature surface
- cc-switch provider commands (`cc-switch/src-tauri/src/commands/provider.rs`) -- switching implementation
- cc-switch config adapters (`cc-switch/src-tauri/src/config.rs`, `codex_config.rs`) -- CLI config format handling

**Confidence note:** No web search was available during this research. Feature landscape is derived from cc-switch reference code analysis, the iCloud sync root-cause document, and training data knowledge of Claude Code / Codex CLI configuration formats. The categorization of table stakes vs differentiators is grounded in the specific problems documented in PROJECT.md (config corruption, iCloud sync failures, feature bloat) rather than broad market analysis. Confidence in the feature list itself is HIGH; confidence in competitive positioning relative to tools outside cc-switch is LOW (no web search to discover other tools in this niche).
