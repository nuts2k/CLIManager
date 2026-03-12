# Phase 5: Onboarding - Research

**Researched:** 2026-03-12
**Domain:** CLI config file scanning, import dialog, first-launch UX
**Confidence:** HIGH

## Summary

Phase 5 implements a first-launch onboarding flow that scans existing CLI configuration files (`~/.claude/settings.json`, `~/.codex/auth.json`) and offers to create Provider entries from detected credentials. The technical challenge is modest: read JSON/TOML config files, extract API keys and base URLs, present them in a confirmation dialog, and call the existing `create_provider` Tauri command.

The codebase already has all building blocks: `ClaudeAdapter` demonstrates reading `~/.claude/settings.json`, `CodexAdapter` demonstrates reading `~/.codex/auth.json` and `~/.codex/config.toml`, the `create_provider` command handles Provider creation with self-write tracking, and the shadcn/ui `Dialog` component provides the UI shell. No new dependencies are needed -- this is pure integration work.

**Primary recommendation:** Add a single `scan_cli_configs` Tauri command that reads existing CLI config files and returns detected configurations as a structured list, then build an `ImportDialog` component using the existing Dialog/Checkbox pattern that calls the existing `create_provider` for each selected item.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Trigger timing:** When all CLIs have empty Provider lists, show onboarding dialog (no extra flag). Deleting all Providers and reopening also re-triggers. Show import Dialog first, skip goes to main UI empty state. Settings page has "import from CLI config" button for manual re-trigger.
- **Dialog form:** Centered Dialog, reuse existing shadcn/ui Dialog, consistent with create/edit Provider Dialog style. Plain text title "导入现有配置", no welcome/branding/decoration. Background blur consistent with existing Dialog behavior.
- **Preview display:** Summary confirmation mode with CLI config list. Each item shows: CLI name, masked API Key (e.g., `sk-ant-ap...H7kQ`), Base URL. Checkbox per item, default all selected. Bottom buttons: "导入已选项" and "跳过".
- **Naming and defaults:** Auto-name "{CLI名} 默认配置" (e.g., "Claude 默认配置"). Protocol type by CLI native protocol: Claude -> Anthropic, Codex -> OpenAI Compatible. Minimal field fill: only import API Key and Base URL, model etc. left empty. No auto-activation after import.
- **Dedup:** Compare API Key + Base URL before import, skip if identical Provider already exists.
- **Missing/partial config:** Both missing -> silently skip onboarding, enter main UI. Config exists but no API Key -> still import, mark "缺少 API Key" in preview. Corrupted JSON/TOML -> silently skip that CLI, log error. Only one CLI detected -> show just that one item.
- **Detection scope:** Only check config file existence and readability. Do NOT run `which claude` or similar.

### Claude's Discretion
- Import flow Tauri command design and Rust implementation details
- Dialog internal layout, spacing, animation
- Backend config file parsing implementation
- Dedup exact match logic
- Toast notification wording

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| ONBD-01 | First launch scans existing `~/.claude/` and `~/.codex/` configs and creates initial Providers | Backend `scan_cli_configs` command extracts credentials from adapter config files; frontend ImportDialog displays results and calls `create_provider` for selected items |
| ONBD-02 | User can also manually create Providers from scratch at any time | Already implemented in Phase 3 -- `ProviderDialog` with create mode exists. Settings page button for manual import re-trigger is new. |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri | 2.x | Backend command framework | Already in use, `#[tauri::command]` pattern established |
| serde_json | 1.x | Parse `~/.claude/settings.json` and `~/.codex/auth.json` | Already in use with `preserve_order` feature |
| toml_edit | 0.25 | Parse `~/.codex/config.toml` for base_url extraction | Already in use by CodexAdapter |
| radix-ui Dialog | 1.4.3 | Import confirmation dialog | Already in use via shadcn/ui `Dialog` component |
| react-i18next | 16.x | i18n for import dialog text | Already in use throughout frontend |
| sonner | 2.x | Toast notifications for import results | Already in use throughout frontend |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| lucide-react | 0.577.x | Icons for import dialog (checkbox states) | Already available, use for any icons needed |

### Checkbox Component Note
No `checkbox.tsx` exists in `src/components/ui/`. The import dialog needs checkboxes for item selection. Two options:
1. **Add shadcn/ui Checkbox component** via `npx shadcn@latest add checkbox` -- this adds a proper Radix-based checkbox
2. **Use native HTML checkbox with Tailwind styling** -- simpler, fewer dependencies

**Recommendation:** Add the shadcn/ui Checkbox component. It provides accessible, styled checkboxes consistent with the rest of the UI. This is a one-command addition and aligns with the project's established pattern of using shadcn/ui primitives.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| shadcn/ui Checkbox | Native `<input type="checkbox">` | Native works fine but looks inconsistent with shadcn theme |
| New `scan_cli_configs` command | Reuse adapter `patch()` read logic directly | Adapters are designed for writing, not read-only scanning; a separate command is cleaner |

**Installation:**
```bash
npx shadcn@latest add checkbox
```
No Rust dependencies needed -- all parsing libraries already present.

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
├── commands/
│   ├── provider.rs           # Add scan_cli_configs command here
│   └── mod.rs                # Register new command
├── adapter/
│   ├── claude.rs             # Reference for config file paths/structure
│   └── codex.rs              # Reference for config file paths/structure

src/
├── components/
│   ├── provider/
│   │   └── ImportDialog.tsx  # New: onboarding import dialog
│   ├── settings/
│   │   └── SettingsPage.tsx  # Modified: add import button
│   └── layout/
│       └── AppShell.tsx      # Modified: trigger logic on mount
├── lib/
│   └── tauri.ts              # Add scanCliConfigs wrapper
└── types/
    └── provider.ts           # Add CliConfigScanResult type
```

### Pattern 1: Scan Command (Backend)
**What:** A read-only Tauri command that scans CLI config files and returns structured results without side effects.
**When to use:** First launch detection and manual re-import from Settings.
**Example:**
```rust
// New struct for scan results
#[derive(Debug, Clone, Serialize)]
pub struct DetectedCliConfig {
    pub cli_id: String,       // "claude" or "codex"
    pub cli_name: String,     // "Claude Code" or "Codex"
    pub api_key: String,      // Full key (masking done in frontend)
    pub base_url: String,     // Extracted base URL
    pub protocol_type: ProtocolType,
    pub has_api_key: bool,    // false if key field missing/empty
}

#[tauri::command]
pub fn scan_cli_configs() -> Result<Vec<DetectedCliConfig>, AppError> {
    let mut results = Vec::new();
    // Scan Claude: ~/.claude/settings.json -> env.ANTHROPIC_AUTH_TOKEN, env.ANTHROPIC_BASE_URL
    // Scan Codex: ~/.codex/auth.json -> OPENAI_API_KEY; ~/.codex/config.toml -> base_url
    // Silent skip on missing/corrupted files
    Ok(results)
}
```

### Pattern 2: Import Dialog State Flow (Frontend)
**What:** AppShell checks Provider count on mount. If all empty, show ImportDialog. Dialog fetches scan results, displays checkable list, imports selected items.
**When to use:** App startup and manual trigger from Settings.
```
AppShell mount
  -> listProviders() for all CLIs
  -> if all empty -> scanCliConfigs()
    -> if results non-empty -> show ImportDialog
    -> if results empty -> skip, show main UI (empty state)
  -> ImportDialog:
    -> user checks/unchecks items
    -> "导入已选项" -> for each selected: createProvider(...)
    -> refresh provider list
    -> close dialog
    -> toast success
```

### Pattern 3: API Key Masking (Frontend)
**What:** Display API keys in masked form showing first and last characters.
**When to use:** Import preview dialog.
**Example:**
```typescript
function maskApiKey(key: string): string {
  if (key.length <= 8) return key.substring(0, 2) + "..." + key.substring(key.length - 2);
  // Show first ~8 chars and last ~4 chars
  return key.substring(0, 8) + "..." + key.substring(key.length - 4);
}
// "sk-ant-api03-abc...H7kQ"
```

### Pattern 4: Deduplication Check (Frontend or Backend)
**What:** Before importing, check if a Provider with the same API Key + Base URL already exists.
**When to use:** Prevent duplicate imports on manual re-trigger.
**Example:**
```typescript
// Frontend approach: compare against existing providers
const existingProviders = await listProviders();
const newConfigs = scanResults.filter(config => {
  return !existingProviders.some(p =>
    p.api_key === config.api_key && p.base_url === config.base_url
  );
});
```

### Anti-Patterns to Avoid
- **Running `which claude` or subprocess detection:** The CONTEXT.md explicitly forbids detecting CLI tool installation. Only check config file existence.
- **Auto-activating imported Providers:** CONTEXT.md says imported Providers should NOT be set as active. Create them only.
- **Using a separate `onboarding_completed` flag:** CONTEXT.md says trigger by checking if all Provider lists are empty. No extra state.
- **Importing model or other fields:** Only import API Key and Base URL per CONTEXT.md. Leave model, model_config, notes empty.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Dialog UI | Custom modal/overlay | shadcn/ui `Dialog` component | Already established pattern, accessible, backdrop blur included |
| Checkbox selection | Custom toggle state | shadcn/ui `Checkbox` (Radix-based) | Consistent styling, accessibility built-in |
| Provider creation | Custom file-write logic | Existing `create_provider` Tauri command | Handles self-write tracking, validation, normalization, iCloud path |
| Config file path resolution | Hardcoded paths | `dirs::home_dir()` (Rust) | Cross-platform home directory resolution already used by adapters |
| Toast notifications | Custom notification system | `sonner` toast | Already integrated throughout app |

**Key insight:** This phase is almost entirely integration work. Every building block exists -- the value is in correctly wiring them together and handling edge cases (missing files, corrupted configs, partial data).

## Common Pitfalls

### Pitfall 1: Race Condition Between Empty Check and Dialog Show
**What goes wrong:** App starts, checks providers are empty, shows import dialog. Meanwhile, iCloud sync adds providers. User imports, creating duplicates.
**Why it happens:** Async gap between empty check and user action.
**How to avoid:** Dedup check happens at import time (not just at dialog open time). Compare API Key + Base URL against current providers right before each `create_provider` call.
**Warning signs:** Duplicate providers appearing after import.

### Pitfall 2: Self-Write Tracker Not Called Before Import
**What goes wrong:** Importing providers creates files in iCloud directory. File watcher fires sync events, causing UI flicker or duplicate toasts.
**Why it happens:** `create_provider` Tauri command already calls `tracker.record_write()` before writing. But if importing multiple providers in rapid succession, the watcher may batch them differently.
**How to avoid:** Use the existing `create_provider` command (which handles self-write tracking) rather than writing provider files directly. Import sequentially (await each create before next) to keep self-write tracking simple.
**Warning signs:** Toast notifications about "synced providers" appearing right after import.

### Pitfall 3: Codex Base URL Extraction Complexity
**What goes wrong:** Codex `config.toml` has two possible locations for `base_url`: top-level or under `model_providers.<active_provider>`. Scanning only checks one location.
**Why it happens:** Codex config supports provider-scoped configuration.
**How to avoid:** Reuse the same logic as `patch_codex_toml` but in reverse -- check `model_provider` field first, then look under `model_providers.<active>/base_url`, fall back to top-level `base_url`.
**Warning signs:** Import showing empty base_url for Codex when one exists in config.

### Pitfall 4: Validation Bypass During Import
**What goes wrong:** `create_provider` validates that `api_key` and `base_url` are non-empty. If a CLI config has API key but no base URL (or vice versa), the import fails silently.
**Why it happens:** CONTEXT.md says "config exists but no API Key -> still import". But `validate_provider` requires non-empty api_key.
**How to avoid:** For import specifically, provide sensible defaults for missing fields. If API Key is missing, set it to empty string and let the user edit later. If Base URL is missing, use the CLI's default (e.g., `https://api.anthropic.com` for Claude, empty for Codex). OR relax validation for imported providers.
**Warning signs:** Import failing with validation errors when config files have partial data.

**Resolution strategy:** The backend `scan_cli_configs` returns raw data with `has_api_key: bool` flag. The frontend shows "缺少 API Key" label for items without keys. At import time, provide a placeholder/default base_url if missing, and skip validation for api_key if empty (or use a special import-only create path). The simplest approach: use default base URLs from the adapter patterns (`https://api.anthropic.com` for Claude, `https://api.openai.com/v1` for Codex) when not found in config.

### Pitfall 5: Dialog Dismissal During Import
**What goes wrong:** User clicks "导入已选项", then closes dialog while imports are in progress. Some providers get created, others don't.
**Why it happens:** Dialog closes mid-operation.
**How to avoid:** Disable the close button and skip button while import is in progress. Show a loading state on the import button.
**Warning signs:** Partial imports, user confusion about what was imported.

## Code Examples

### Reading Claude Config for Scanning
```rust
// Source: existing ClaudeAdapter pattern in src-tauri/src/adapter/claude.rs
fn scan_claude_config() -> Option<DetectedCliConfig> {
    let home = dirs::home_dir()?;
    let settings_path = home.join(".claude").join("settings.json");

    let content = std::fs::read_to_string(&settings_path).ok()?;
    let root: serde_json::Value = serde_json::from_str(&content).ok()?;

    let env = root.get("env")?.as_object()?;
    let api_key = env.get("ANTHROPIC_AUTH_TOKEN")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let base_url = env.get("ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str())
        .unwrap_or("https://api.anthropic.com")
        .to_string();

    Some(DetectedCliConfig {
        cli_id: "claude".to_string(),
        cli_name: "Claude Code".to_string(),
        api_key: api_key.clone(),
        base_url,
        protocol_type: ProtocolType::Anthropic,
        has_api_key: !api_key.is_empty(),
    })
}
```

### Reading Codex Config for Scanning
```rust
// Source: existing CodexAdapter pattern in src-tauri/src/adapter/codex.rs
fn scan_codex_config() -> Option<DetectedCliConfig> {
    let home = dirs::home_dir()?;
    let config_dir = home.join(".codex");

    // Read API key from auth.json
    let api_key = std::fs::read_to_string(config_dir.join("auth.json"))
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|root| root.get("OPENAI_API_KEY")?.as_str().map(str::to_string))
        .unwrap_or_default();

    // Read base_url from config.toml (handles provider-scoped config)
    let base_url = std::fs::read_to_string(config_dir.join("config.toml"))
        .ok()
        .and_then(|content| {
            let doc: toml_edit::DocumentMut = content.parse().ok()?;
            // Check provider-scoped first
            if let Some(active) = doc.get("model_provider").and_then(|v| v.as_str()) {
                if let Some(url) = doc.get("model_providers")
                    .and_then(|mp| mp.get(active))
                    .and_then(|p| p.get("base_url"))
                    .and_then(|v| v.as_str()) {
                    return Some(url.to_string());
                }
            }
            // Fall back to top-level
            doc.get("base_url").and_then(|v| v.as_str()).map(str::to_string)
        })
        .unwrap_or_default();

    // Only return if at least auth.json or config.toml existed
    if !config_dir.join("auth.json").exists() && !config_dir.join("config.toml").exists() {
        return None;
    }

    Some(DetectedCliConfig {
        cli_id: "codex".to_string(),
        cli_name: "Codex".to_string(),
        api_key: api_key.clone(),
        base_url,
        protocol_type: ProtocolType::OpenAiCompatible,
        has_api_key: !api_key.is_empty(),
    })
}
```

### Frontend Import Flow
```typescript
// Source: existing create_provider pattern in src/hooks/useProviders.ts
async function handleImport(selectedConfigs: DetectedCliConfig[]) {
  const existingProviders = await listProviders();

  for (const config of selectedConfigs) {
    // Dedup check
    const isDuplicate = existingProviders.some(
      p => p.api_key === config.api_key && p.base_url === config.base_url
    );
    if (isDuplicate) continue;

    await createProvider({
      name: `${config.cli_name} ${t("import.defaultSuffix")}`, // "Claude Code 默认配置"
      protocolType: config.protocol_type,
      apiKey: config.api_key,
      baseUrl: config.base_url,
      model: "",
      cliId: config.cli_id,
    });
  }
}
```

### Trigger Logic in AppShell
```typescript
// Source: existing AppShell mount pattern in src/components/layout/AppShell.tsx
useEffect(() => {
  // Check if onboarding should trigger
  async function checkOnboarding() {
    const claudeProviders = await listProviders("claude");
    const codexProviders = await listProviders("codex");
    if (claudeProviders.length === 0 && codexProviders.length === 0) {
      const configs = await scanCliConfigs();
      if (configs.length > 0) {
        setShowImportDialog(true);
      }
    }
  }
  checkOnboarding();
}, []);
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `onboarding_completed` flag | Check if all Provider lists empty | Phase 5 decision | Simpler, no extra state to manage |
| Detect CLI binary installation | Only check config file existence | Phase 5 decision | More reliable, avoids PATH issues |

**Deprecated/outdated:**
- None specific to this phase. The existing adapter code is current and stable.

## Open Questions

1. **Validation relaxation for empty API Key imports**
   - What we know: CONTEXT.md says "config exists but no API Key -> still import, mark in preview". The existing `create_provider` requires non-empty `api_key` via `validate_provider`.
   - What's unclear: Should we relax the validation or provide a placeholder value?
   - Recommendation: Use a dedicated `import_providers` command that bypasses the `api_key` empty check, OR provide a placeholder empty string and relax the validation to allow empty api_key only during import. The simplest path is to allow empty `api_key` in the provider model (validation is a UX concern, not a data integrity concern).

2. **Default Base URL for missing configs**
   - What we know: Claude's default base URL is `https://api.anthropic.com`. Codex has no single obvious default.
   - What's unclear: What base URL to use when config.toml doesn't specify one.
   - Recommendation: Use `https://api.anthropic.com` for Claude (this is what the CLI defaults to). For Codex, leave base_url empty -- user can fill it in via edit. This may require relaxing the `base_url` validation for imports too.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (built-in, no extra framework) |
| Config file | `src-tauri/Cargo.toml` (dev-dependencies: tempfile) |
| Quick run command | `cd src-tauri && cargo test --lib -q` |
| Full suite command | `cd src-tauri && cargo test --lib -q` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ONBD-01 | scan_cli_configs returns detected configs from Claude settings.json | unit | `cd src-tauri && cargo test scan_claude -q` | Wave 0 |
| ONBD-01 | scan_cli_configs returns detected configs from Codex auth.json + config.toml | unit | `cd src-tauri && cargo test scan_codex -q` | Wave 0 |
| ONBD-01 | scan_cli_configs silently skips missing config directories | unit | `cd src-tauri && cargo test scan_missing -q` | Wave 0 |
| ONBD-01 | scan_cli_configs silently skips corrupted JSON/TOML | unit | `cd src-tauri && cargo test scan_corrupted -q` | Wave 0 |
| ONBD-01 | scan_cli_configs returns has_api_key=false when key is missing | unit | `cd src-tauri && cargo test scan_no_key -q` | Wave 0 |
| ONBD-02 | Manual Provider creation (ProviderDialog) | existing | `cd src-tauri && cargo test create_provider -q` | Exists |

### Sampling Rate
- **Per task commit:** `cd src-tauri && cargo test --lib -q`
- **Per wave merge:** `cd src-tauri && cargo test --lib -q`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] Backend scan tests -- tests for the new `scan_cli_configs` logic (scan_claude_config, scan_codex_config, edge cases)
- [ ] No frontend test framework exists (no vitest/jest config) -- frontend testing is manual-only for this project

*(Frontend import dialog, trigger logic, and settings button are manual verification only since no frontend test infrastructure exists.)*

## Sources

### Primary (HIGH confidence)
- Codebase inspection: `src-tauri/src/adapter/claude.rs` -- exact field paths for Claude config (`env.ANTHROPIC_AUTH_TOKEN`, `env.ANTHROPIC_BASE_URL`)
- Codebase inspection: `src-tauri/src/adapter/codex.rs` -- exact field paths for Codex config (`OPENAI_API_KEY` in auth.json, `base_url` in config.toml with provider-scoped fallback)
- Codebase inspection: `src-tauri/src/commands/provider.rs` -- `create_provider` command signature and self-write tracking pattern
- Codebase inspection: `src/components/provider/ProviderDialog.tsx` -- Dialog component pattern, form handling
- Codebase inspection: `src/components/layout/AppShell.tsx` -- App mount lifecycle, settings restoration pattern
- Codebase inspection: `src/components/settings/SettingsPage.tsx` -- Settings page structure for adding import button
- Codebase inspection: `src/lib/tauri.ts` -- Tauri invoke wrapper pattern
- Codebase inspection: `src/i18n/locales/zh.json` and `en.json` -- i18n key structure

### Secondary (MEDIUM confidence)
- CONTEXT.md decisions -- user-specified behavior for all edge cases

### Tertiary (LOW confidence)
- None -- all findings are from direct codebase inspection

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all libraries already in use, no new dependencies except shadcn/ui Checkbox
- Architecture: HIGH - follows exact patterns established in Phases 1-4
- Pitfalls: HIGH - identified from direct code analysis of validation, self-write tracking, and config parsing
- Code examples: HIGH - derived directly from existing adapter implementations

**Research date:** 2026-03-12
**Valid until:** 2026-04-12 (stable domain, no external API changes expected)
