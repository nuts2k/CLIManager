# Phase 3: Provider Management UI - Research

**Researched:** 2026-03-11
**Domain:** Tauri v2 React frontend with shadcn/ui, Tailwind CSS, i18n, and Rust backend extensions
**Confidence:** HIGH

## Summary

Phase 3 builds the complete Provider Management UI for CLIManager -- a Tauri v2 desktop app with a React frontend. The current frontend is an empty placeholder (`App.tsx` with static content, vanilla CSS, no component library). The Rust backend already has working Provider CRUD commands and CLI adapter patch logic from Phases 1-2. This phase needs to: (1) set up shadcn/ui + Tailwind CSS v4 infrastructure, (2) build the tabbed Provider management interface, (3) integrate react-i18next for zh/en localization, (4) extend backend to support `cli_id` filtering and per-CLI active providers, and (5) add Provider connectivity testing via reqwest.

The cc-switch reference codebase provides useful patterns for i18n structure, shadcn/ui component usage, and UI layout concepts, but uses React 18 and Tailwind v3 while CLIManager uses React 19 and should use Tailwind v4.

**Primary recommendation:** Set up shadcn/ui with Tailwind CSS v4 via `@tailwindcss/vite` plugin, use dark-only theme by putting dark CSS variables directly in `:root`, structure the app with a simple page-based layout (main view + settings page), and keep state management lightweight with React's built-in state + a thin Tauri invoke wrapper.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Top Tab layout by CLI (Claude Code / Codex), each Tab shows independent Provider list
- Provider cards as row items with hover-reveal action buttons
- Settings via gear icon in top-right corner
- Default window size ~1000x700, dark theme only
- UI library: shadcn/ui + Tailwind CSS
- Create/Edit Provider via Dialog popup
- Simplified required fields (name, API Key, Base URL) + collapsible "Advanced" section (model, notes, protocol_type, model_config)
- API Key masked by default with toggle to show plaintext
- Base URL has no pre-filled default
- One-click switch via hover button on card row
- Active Provider indicated by left blue bar + border highlight
- Success/failure feedback via bottom-right Toast
- Switch triggers adapter surgical patch immediately
- Hover actions: switch, edit, copy (same CLI), copy to other CLI, test, delete
- Copy adds "(copy)" suffix; copy-to-other-CLI adds "(copy from Claude)" etc.
- Test: API connectivity test, result shown as Toast with response time
- Test config in settings (timeout, test model, with defaults)
- Delete shows confirmation Dialog; deleting active provider auto-switches to next available (circular search)
- Loading spinners on buttons during operations, disable repeat clicks
- Empty state with friendly text + "New Provider" button
- Provider gets `cli_id` field; LocalSettings changes `active_provider_id` to `active_providers: { "claude": "xxx", "codex": "yyy" }`
- i18n: react-i18next with JSON files, default Chinese, switchable to English in settings
- Settings page: language switch, test config, about info

### Claude's Discretion
- Specific shadcn/ui component choices and composition
- Card row spacing, font sizes, animation effects
- Toast display duration and position details
- Settings page layout design
- State management approach (React Query, Zustand, etc.)
- Test request implementation details (prompt content, model selection logic)
- Empty state copy and visual design

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PROV-01 | User can create a new Provider with name, API key, base URL, and model | Dialog form with shadcn/ui Input/Select, Tauri invoke to create_provider command (needs cli_id extension) |
| PROV-02 | User can view all Providers in a list with clear display | Tabbed layout with provider card rows, Tauri invoke to list_providers (needs cli_id filter) |
| PROV-03 | User can edit an existing Provider's settings | Same Dialog form in edit mode, Tauri invoke to update_provider |
| PROV-04 | User can delete a Provider | Confirmation Dialog + delete_provider command + auto-switch logic |
| PROV-05 | User can see which Provider is currently active for each CLI | Per-CLI active_providers in LocalSettings, blue indicator bar on active card |
| PROV-06 | User can switch active Provider with one click (< 1s) | Hover button triggers set_active_provider (refactored to also call adapter.patch()) |
| I18N-01 | UI supports Chinese and English with all text externalized | react-i18next with JSON resource files |
| I18N-02 | Default language is Chinese | i18n init with `lng: "zh"` |
| I18N-03 | User can switch language in settings | Settings page with language Select, persist to LocalSettings |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| shadcn/ui | latest (CLI-based) | UI component library | Locked decision; composable Radix primitives + Tailwind |
| Tailwind CSS | v4 | Utility-first CSS | Current version; uses `@tailwindcss/vite` plugin (no PostCSS config needed) |
| react-i18next | ^16.0 | i18n framework | Locked decision; mature, hook-based, JSON resource files |
| i18next | ^25.0 | i18n core | Required peer dependency of react-i18next |
| lucide-react | latest | Icons | Standard icon set for shadcn/ui ecosystem |
| sonner | latest | Toast notifications | shadcn/ui's recommended toast component |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| class-variance-authority | ^0.7 | Component variant management | Installed by shadcn/ui init |
| clsx | ^2.1 | Conditional class names | Installed by shadcn/ui init |
| tailwind-merge | ^3.0 | Tailwind class deduplication | Installed by shadcn/ui init |
| zod | ^3.23 | Form validation schemas | Provider form validation |
| reqwest | 0.12 (Rust) | HTTP client | Provider connectivity testing from backend |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Raw React state | Zustand / React Query | For this app's scale (~10 providers, 2 CLI tabs), React state + useEffect is sufficient. Add Zustand only if state sharing becomes complex |
| react-hook-form | Controlled inputs | react-hook-form adds weight; shadcn/ui Dialog forms are simple enough with controlled state + zod for validation |
| framer-motion | CSS transitions | Hover animations on card rows can be done with Tailwind transitions; framer-motion is overkill |

**Installation (frontend):**
```bash
# Tailwind CSS v4 with Vite plugin
pnpm add tailwindcss @tailwindcss/vite

# i18n
pnpm add i18next react-i18next

# Form validation
pnpm add zod

# shadcn/ui init (interactive -- sets up components.json, installs Radix deps, lucide-react, sonner, etc.)
pnpm dlx shadcn@latest init

# Then add needed components:
pnpm dlx shadcn@latest add button dialog input label select tabs card collapsible badge dropdown-menu sonner scroll-area
```

**Installation (backend -- Cargo.toml):**
```toml
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }  # may already be pulled in by tauri
```

## Architecture Patterns

### Recommended Project Structure
```
src/
  main.tsx                  # Entry point, imports i18n config
  App.tsx                   # Root layout: Toaster + Router (main/settings)
  App.css                   # Remove old CSS, replaced by Tailwind
  lib/
    utils.ts                # cn() helper (created by shadcn init)
    tauri.ts                # Typed invoke wrappers for Tauri commands
  i18n/
    index.ts                # i18n initialization
    locales/
      zh.json               # Chinese translations
      en.json               # English translations
  components/
    ui/                     # shadcn/ui primitives (auto-generated)
    layout/
      AppShell.tsx           # Overall app layout (header + content area)
      Header.tsx             # App title + settings gear icon
    provider/
      ProviderTabs.tsx       # CLI tab switcher (Claude / Codex)
      ProviderList.tsx       # List of provider cards for current tab
      ProviderCard.tsx       # Single provider row with hover actions
      ProviderDialog.tsx     # Create/Edit provider form dialog
      DeleteConfirmDialog.tsx # Delete confirmation
      EmptyState.tsx         # Empty provider list placeholder
    settings/
      SettingsPage.tsx       # Settings view (language, test config, about)
  hooks/
    useProviders.ts          # Hook wrapping Tauri provider CRUD calls
    useSettings.ts           # Hook wrapping LocalSettings calls
  types/
    provider.ts              # TypeScript types mirroring Rust Provider struct
    settings.ts              # TypeScript types mirroring Rust LocalSettings
```

### Pattern 1: Typed Tauri Invoke Wrapper
**What:** Thin typed wrappers around `invoke()` for type safety
**When to use:** Every Tauri command call
**Example:**
```typescript
// src/lib/tauri.ts
import { invoke } from "@tauri-apps/api/core";
import type { Provider, LocalSettings } from "@/types";

export async function listProviders(cliId: string): Promise<Provider[]> {
  return invoke("list_providers", { cliId });
}

export async function createProvider(data: CreateProviderInput): Promise<Provider> {
  return invoke("create_provider", data);
}

export async function setActiveProvider(cliId: string, providerId: string | null): Promise<LocalSettings> {
  return invoke("set_active_provider", { cliId, providerId });
}

export async function testProvider(providerId: string): Promise<TestResult> {
  return invoke("test_provider", { providerId });
}
```

### Pattern 2: Dark-Only Theme Setup
**What:** Skip theme toggling entirely; put dark variables directly in `:root`
**When to use:** This project (locked to dark theme only)
**Example:**
```css
/* src/index.css */
@import "tailwindcss";

@custom-variant dark (&:is(.dark *));

@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  /* ... shadcn color mappings */
}

:root {
  /* Use dark theme values directly -- no .dark class needed */
  --background: 222.2 84% 4.9%;
  --foreground: 210 40% 98%;
  /* ... all other dark variables from shadcn */
}
```
Also add `class="dark"` to `<html>` element for shadcn components that check `.dark` ancestor.

### Pattern 3: Provider Card with Hover Actions
**What:** Card row that reveals action buttons on hover
**When to use:** Each provider in the list
**Example:**
```typescript
// Conceptual structure
function ProviderCard({ provider, isActive, onSwitch, onEdit, onDelete, onCopy, onTest }) {
  return (
    <div className="group relative flex items-center p-3 rounded-lg border transition-colors
                    hover:bg-accent/50
                    data-[active=true]:border-blue-500 data-[active=true]:border-l-4">
      {/* Left blue indicator for active */}
      <div className="flex-1">
        <span className="font-medium">{provider.name}</span>
        <span className="text-sm text-muted-foreground truncate">{provider.base_url}</span>
      </div>
      {/* Hover actions - slide in from right */}
      <div className="opacity-0 group-hover:opacity-100 transition-opacity flex gap-1">
        <Button size="sm" onClick={onSwitch}>Switch</Button>
        <DropdownMenu>{/* edit, copy, copy-to, test, delete */}</DropdownMenu>
      </div>
    </div>
  );
}
```

### Pattern 4: i18n with Persisted Language in LocalSettings
**What:** Language preference stored in Tauri LocalSettings (backend), not just localStorage
**When to use:** Language switch in settings
**Example:**
```typescript
// src/i18n/index.ts
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import zh from "./locales/zh.json";
import en from "./locales/en.json";

i18n.use(initReactI18next).init({
  resources: {
    zh: { translation: zh },
    en: { translation: en },
  },
  lng: "zh",  // default Chinese, will be overridden by stored preference
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;
```

### Anti-Patterns to Avoid
- **Putting business logic in React components:** Keep the switch-and-patch logic entirely in Rust. The frontend just calls `set_active_provider` and the backend handles both LocalSettings update AND adapter patch
- **Global state store for simple CRUD:** With only 2 tabs and <10 providers each, React component state + refetch is simpler than a full state management library
- **Hand-rolling toast system:** Use Sonner via shadcn/ui; it handles positioning, animation, stacking, auto-dismiss
- **Mixing CSS approaches:** Remove all old `App.css` styles; use only Tailwind utility classes once shadcn/ui is set up

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Toast notifications | Custom notification system | sonner (via shadcn/ui) | Handles positioning, animation, stacking, accessibility |
| Form dialogs | Custom modal implementation | shadcn/ui Dialog + form elements | Handles focus trapping, escape key, overlay, animation |
| Tab switching | Custom tab component | shadcn/ui Tabs | Handles keyboard navigation, ARIA roles |
| Dropdown menus | Custom dropdown | shadcn/ui DropdownMenu | Handles positioning, keyboard nav, accessibility |
| CSS utility system | Custom CSS classes | Tailwind CSS v4 | Consistent design tokens, responsive, dark mode |
| i18n | Custom translation function | react-i18next | Handles interpolation, pluralization, language detection |
| API connectivity test | Frontend fetch | Rust reqwest in backend | No CORS issues, can set exact timeouts, runs outside webview sandbox |
| Scroll containers | Custom overflow handling | shadcn/ui ScrollArea | Cross-platform scrollbar styling |

**Key insight:** This UI is a standard CRUD interface with a tabbed layout. Every component needed (Tabs, Dialog, DropdownMenu, Button, Input, Select, Card, ScrollArea, Collapsible, Badge, Sonner) exists in shadcn/ui. The only custom work is composition and business logic.

## Common Pitfalls

### Pitfall 1: Tailwind CSS v4 vs v3 Configuration
**What goes wrong:** Using `tailwind.config.js` or PostCSS-based setup (Tailwind v3 pattern) instead of the Vite plugin approach (v4)
**Why it happens:** Most tutorials and cc-switch reference code use v3
**How to avoid:** Use `@tailwindcss/vite` plugin in `vite.config.ts`; put `@import "tailwindcss"` in CSS; no `tailwind.config.js` needed. Theme customization goes in CSS via `@theme` directive
**Warning signs:** Seeing `postcss.config.js` or `tailwind.config.js` files being created

### Pitfall 2: shadcn/ui Init with Wrong Framework Detection
**What goes wrong:** `npx shadcn@latest init` may not detect the Vite + React setup correctly, especially with Tailwind v4
**Why it happens:** shadcn CLI checks for framework markers; Tauri projects have non-standard structure
**How to avoid:** Run init interactively, verify it detects "Vite" and "Tailwind v4". If issues, use manual installation: create `components.json` manually and add components individually
**Warning signs:** Init creating PostCSS config or wrong CSS file paths

### Pitfall 3: Tauri v2 Invoke Argument Naming
**What goes wrong:** Rust command arguments use `snake_case` but frontend must pass `camelCase`
**Why it happens:** Tauri v2's IPC layer automatically converts between conventions
**How to avoid:** Define typed wrapper functions that use camelCase keys; let Tauri handle conversion
**Warning signs:** "missing argument" errors from Tauri invoke

### Pitfall 4: Tauri Async Commands
**What goes wrong:** Using `reqwest` (async) in synchronous Tauri commands causes runtime panic
**Why it happens:** `reqwest` requires a Tokio runtime; non-async Tauri commands run on the main thread
**How to avoid:** Mark commands that use reqwest as `async`: `#[tauri::command] async fn test_provider(...) -> Result<..., AppError>`
**Warning signs:** "Cannot start a runtime from within a runtime" panic

### Pitfall 5: Breaking Existing Provider JSON Files
**What goes wrong:** Adding `cli_id` field to Provider struct breaks deserialization of existing provider files that lack the field
**Why it happens:** serde requires all non-Optional fields to be present
**How to avoid:** Make `cli_id` use `#[serde(default)]` with a sensible default (e.g., empty string or "claude"), OR make it `Option<String>` with migration logic
**Warning signs:** Existing provider files failing to load after the struct change

### Pitfall 6: React 19 Peer Dependency Conflicts
**What goes wrong:** npm/pnpm may fail to install shadcn/ui dependencies due to React 19 peer dependency issues
**Why it happens:** Some Radix packages declare React 18 as peer dependency
**How to avoid:** Use `pnpm` (more lenient with peer deps by default) or add `--legacy-peer-deps` flag. The project already uses pnpm.
**Warning signs:** "peer dependency" errors during install

### Pitfall 7: Window Size Not Matching Design
**What goes wrong:** Tauri window stays at default 800x600 instead of designed 1000x700
**Why it happens:** `tauri.conf.json` has `width: 800, height: 600` from scaffolding
**How to avoid:** Update `tauri.conf.json` window dimensions to 1000x700

## Code Examples

### Tauri Command: list_providers with cli_id Filter
```rust
// After adding cli_id to Provider struct
#[tauri::command]
pub fn list_providers(cli_id: Option<String>) -> Result<Vec<Provider>, AppError> {
    let all = crate::storage::icloud::list_providers()?;
    match cli_id {
        Some(id) => Ok(all.into_iter().filter(|p| p.cli_id == id).collect()),
        None => Ok(all),
    }
}
```

### Tauri Command: set_active_provider with Adapter Patch
```rust
// Refactored to also trigger adapter patch
#[tauri::command]
pub fn set_active_provider(cli_id: String, provider_id: Option<String>) -> Result<LocalSettings, AppError> {
    let mut settings = read_local_settings()?;

    // Update per-CLI active provider
    settings.active_providers.insert(cli_id.clone(), provider_id.clone());
    write_local_settings(&settings)?;

    // If setting an active provider, trigger adapter patch
    if let Some(pid) = &provider_id {
        let provider = crate::storage::icloud::get_provider(pid)?;
        let adapter = get_adapter_for_cli(&cli_id, &settings)?;
        adapter.patch(&provider)?;
    }

    Ok(settings)
}
```

### Tauri Command: test_provider (Async)
```rust
#[tauri::command]
pub async fn test_provider(provider_id: String) -> Result<TestResult, AppError> {
    let provider = crate::storage::icloud::get_provider(&provider_id)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let start = std::time::Instant::now();
    let response = client
        .post(&format!("{}/v1/chat/completions", provider.base_url))
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .json(&serde_json::json!({
            "model": provider.model,
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 1
        }))
        .send()
        .await;

    let elapsed_ms = start.elapsed().as_millis() as u64;

    match response {
        Ok(resp) if resp.status().is_success() => Ok(TestResult { success: true, elapsed_ms, error: None }),
        Ok(resp) => Ok(TestResult { success: false, elapsed_ms, error: Some(format!("HTTP {}", resp.status())) }),
        Err(e) => Ok(TestResult { success: false, elapsed_ms, error: Some(e.to_string()) }),
    }
}
```

### Frontend: useProviders Hook
```typescript
import { useState, useEffect, useCallback } from "react";
import * as api from "@/lib/tauri";
import type { Provider } from "@/types/provider";

export function useProviders(cliId: string) {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api.listProviders(cliId);
      setProviders(list);
    } finally {
      setLoading(false);
    }
  }, [cliId]);

  useEffect(() => { refresh(); }, [refresh]);

  return { providers, loading, refresh };
}
```

### LocalSettings Schema Change
```rust
// Before (current):
pub struct LocalSettings {
    pub active_provider_id: Option<String>,
    // ...
}

// After (Phase 3):
use std::collections::HashMap;

pub struct LocalSettings {
    // Backward compat: keep old field for migration, skip serializing
    #[serde(default, skip_serializing)]
    pub active_provider_id: Option<String>,

    // New per-CLI active providers
    #[serde(default)]
    pub active_providers: HashMap<String, Option<String>>,

    // New: persisted language preference
    #[serde(default)]
    pub language: Option<String>,

    // New: test configuration
    #[serde(default)]
    pub test_config: Option<TestConfig>,

    // existing fields...
    pub icloud_dir_override: Option<String>,
    pub cli_paths: CliPaths,
    pub schema_version: u32,
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tailwind v3 + PostCSS + tailwind.config.js | Tailwind v4 + `@tailwindcss/vite` plugin | 2025 | No PostCSS config needed; CSS-first configuration via `@theme` |
| shadcn/ui with Tailwind v3 | shadcn/ui with Tailwind v4 support | 2025 | Init CLI auto-detects v4; uses `@custom-variant dark` instead of `darkMode: 'class'` |
| `@tauri-apps/api/tauri` | `@tauri-apps/api/core` | Tauri v2 | Import path change for `invoke` |
| Single active_provider_id | Per-CLI active_providers HashMap | This phase | Supports independent provider management per CLI |

**Deprecated/outdated:**
- `tailwind.config.js` / `postcss.config.js`: Not needed with Tailwind v4 + Vite plugin
- `@tauri-apps/api/tauri`: Tauri v1 import path; use `@tauri-apps/api/core` in v2
- cc-switch's React 18 patterns: This project uses React 19; no need for legacy patterns

## Open Questions

1. **Provider test endpoint differences by protocol type**
   - What we know: Anthropic API uses `/v1/messages`, OpenAI-compatible uses `/v1/chat/completions`
   - What's unclear: Exact minimal request format for each to minimize token usage
   - Recommendation: Branch test logic by `protocol_type` in Rust; use `max_tokens: 1` to minimize cost

2. **Migration of existing Provider files when adding cli_id**
   - What we know: Existing provider JSON files lack `cli_id` field
   - What's unclear: Whether to use serde default or explicit migration
   - Recommendation: Use `#[serde(default = "default_cli_id")]` defaulting to "claude" for backward compat, since the project is pre-release

3. **State management scaling**
   - What we know: Current app has 2 CLI tabs, <10 providers each
   - What's unclear: Whether refresh-on-mutation is sufficient or if optimistic updates are needed
   - Recommendation: Start with refetch-after-mutation. Add Zustand/React Query only if UX feels sluggish. Given local file I/O speeds, refetch should complete in <50ms.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework (backend) | cargo test (existing, working) |
| Framework (frontend) | vitest (needs setup) |
| Config file (frontend) | none -- needs Wave 0 setup |
| Quick run command (backend) | `cd src-tauri && cargo test` |
| Quick run command (frontend) | `pnpm vitest run` |
| Full suite command | `cd src-tauri && cargo test && cd .. && pnpm vitest run` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PROV-01 | Create provider with cli_id | unit (Rust) | `cd src-tauri && cargo test commands::provider::tests::test_create_provider_with_cli_id` | No -- Wave 0 |
| PROV-02 | List providers filtered by cli_id | unit (Rust) | `cd src-tauri && cargo test commands::provider::tests::test_list_by_cli_id` | No -- Wave 0 |
| PROV-03 | Update provider preserves cli_id | unit (Rust) | `cd src-tauri && cargo test commands::provider::tests::test_update_provider` | No -- Wave 0 |
| PROV-04 | Delete provider + auto-switch | unit (Rust) | `cd src-tauri && cargo test commands::provider::tests::test_delete_active_auto_switch` | No -- Wave 0 |
| PROV-05 | Per-CLI active provider in settings | unit (Rust) | `cd src-tauri && cargo test storage::local::tests::test_per_cli_active_providers` | No -- Wave 0 |
| PROV-06 | Switch triggers adapter patch | unit (Rust) | `cd src-tauri && cargo test commands::provider::tests::test_switch_triggers_patch` | No -- Wave 0 |
| I18N-01 | All UI text externalized | manual | Visual inspection of both languages | N/A |
| I18N-02 | Default language is Chinese | unit (frontend) | `pnpm vitest run src/i18n` | No -- Wave 0 |
| I18N-03 | Language switch persists | unit (frontend) | `pnpm vitest run src/hooks/useSettings` | No -- Wave 0 |

### Sampling Rate
- **Per task commit:** `cd src-tauri && cargo test` (Rust changes) or `pnpm vitest run` (frontend changes)
- **Per wave merge:** Full suite: `cd src-tauri && cargo test && cd .. && pnpm vitest run`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `vitest.config.ts` -- frontend test config with jsdom environment
- [ ] `pnpm add -D vitest @testing-library/react @testing-library/jest-dom jsdom` -- test dependencies
- [ ] Rust unit tests for new cli_id filtering and per-CLI active providers
- [ ] Rust unit tests for test_provider async command

## Sources

### Primary (HIGH confidence)
- Existing codebase: `src-tauri/src/provider.rs`, `src-tauri/src/commands/provider.rs`, `src-tauri/src/storage/local.rs`, `src-tauri/src/adapter/mod.rs` -- current data model and command patterns
- cc-switch reference: `cc-switch/src/i18n/index.ts`, `cc-switch/package.json` -- i18n setup pattern and library choices
- [shadcn/ui Vite Installation](https://ui.shadcn.com/docs/installation/vite) -- official installation guide
- [shadcn/ui Theming](https://ui.shadcn.com/docs/theming) -- CSS variable approach for dark-only
- [Tauri v2 Calling Rust](https://v2.tauri.app/develop/calling-rust/) -- invoke pattern from frontend

### Secondary (MEDIUM confidence)
- [shadcn/ui Tailwind v4 docs](https://ui.shadcn.com/docs/tailwind-v4) -- v4 specific configuration
- [Sonner docs](https://ui.shadcn.com/docs/components/radix/sonner) -- toast component API
- [Tauri v2 HTTP Client](https://v2.tauri.app/plugin/http-client/) -- reqwest usage from Rust backend

### Tertiary (LOW confidence)
- State management recommendation (start simple) -- based on app scale assessment, no external source

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- locked decisions (shadcn/ui, react-i18next) verified against official docs
- Architecture: HIGH -- existing codebase patterns (Tauri commands, storage modules) directly inform the structure
- Pitfalls: HIGH -- Tailwind v3/v4 confusion and Tauri v1/v2 import differences are well-documented
- Backend changes: HIGH -- existing Provider struct, LocalSettings, and adapter trait provide clear extension points
- Frontend test setup: MEDIUM -- vitest is standard for Vite projects but not yet configured in this project

**Research date:** 2026-03-11
**Valid until:** 2026-04-11 (30 days -- stable domain, locked decisions)
