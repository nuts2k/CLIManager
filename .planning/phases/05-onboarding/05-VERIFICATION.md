---
phase: 05-onboarding
verified: 2026-03-12T09:30:00Z
status: human_needed
score: 14/17 must-haves verified
re_verification: false
human_verification:
  - test: "First-launch auto-trigger: delete all providers, restart app with ~/.claude/ or ~/.codex/ configs present"
    expected: "Import dialog appears automatically showing detected CLI configs"
    why_human: "Requires running app and observing startup behavior"
  - test: "Skip button dismisses dialog, main UI shows empty state"
    expected: "Dialog closes, provider list empty, no providers created"
    why_human: "Interactive UI behavior requiring visual confirmation"
  - test: "Settings page import button triggers dialog with fresh scan results"
    expected: "Clicking button in Settings opens import dialog with current CLI configs"
    why_human: "Multi-step navigation flow requiring visual confirmation"
---

# Phase 5: Onboarding Verification Report

**Phase Goal:** New users get started instantly by importing their existing CLI configurations
**Verified:** 2026-03-12T09:30:00Z
**Status:** human_needed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

**Plan 01 (Backend)**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | scan_cli_configs returns detected Claude config when ~/.claude/settings.json exists with ANTHROPIC_AUTH_TOKEN | VERIFIED | `scan_claude_config_in` reads settings.json, extracts env.ANTHROPIC_AUTH_TOKEN; test `test_scan_claude_valid_config` confirms (line 247-271) |
| 2 | scan_cli_configs returns detected Codex config when ~/.codex/auth.json exists with OPENAI_API_KEY | VERIFIED | `scan_codex_config_in` reads auth.json, extracts OPENAI_API_KEY; test `test_scan_codex_valid_config` confirms (line 332-356) |
| 3 | scan_cli_configs extracts base_url from Codex config.toml with provider-scoped fallback | VERIFIED | Lines 104-118 implement `model_provider` -> `model_providers.<active>.base_url` -> top-level `base_url` fallback; test `test_scan_codex_provider_scoped_base_url` confirms (line 359-384) |
| 4 | scan_cli_configs silently skips missing or corrupted config files (returns empty vec, no error) | VERIFIED | `scan_claude_config_in` returns None on missing/corrupt; `scan_codex_config_in` returns None when neither file exists; tests `test_scan_claude_missing_file`, `test_scan_claude_corrupted_json`, `test_scan_codex_missing_files`, `test_scan_codex_corrupted_toml` all confirm |
| 5 | scan_cli_configs returns has_api_key=false when API key field is missing or empty | VERIFIED | Tests `test_scan_claude_missing_api_key` (line 274-289) and `test_scan_codex_missing_api_key` (line 434-446) confirm has_api_key is false |
| 6 | create_provider can accept empty api_key and base_url for imported providers | VERIFIED | `import_provider_to` only validates non-empty name; tests `test_import_provider_with_empty_api_key` and `test_import_provider_with_empty_base_url` both pass (lines 451-489) |

**Plan 02 (Frontend)**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 7 | When all CLI Provider lists are empty on app launch, and CLI configs are detected, the import dialog appears automatically | NEEDS HUMAN | AppShell.tsx lines 36-51: `checkOnboarding()` calls `listProviders("claude")` and `listProviders("codex")`, scans if both empty, sets `showImportDialog=true`. Code logic verified but runtime behavior needs human confirmation. |
| 8 | When all CLI Provider lists are empty but no CLI configs exist, app enters main UI silently (no dialog) | VERIFIED | AppShell.tsx line 42-44: only sets `showImportDialog(true)` when `configs.length > 0`; silently proceeds otherwise |
| 9 | Import dialog shows detected configs with CLI name, masked API key, Base URL, and checkboxes (default all selected for complete configs) | NEEDS HUMAN | ImportDialog.tsx lines 124-162: renders each config as a labeled row with Checkbox, cli_name, masked API key (or warning), base_url (or warning). `isConfigComplete` gates default selection. Code structure verified; visual appearance needs human confirmation. |
| 10 | User can deselect items and import only selected configs | VERIFIED | ImportDialog.tsx lines 59-61: `handleToggle` updates `selected` state per index; line 71: `if (!effectiveSelected[i]) continue` skips unselected |
| 11 | Import button creates providers via import_provider command and refreshes provider list | VERIFIED | ImportDialog.tsx lines 82-88: calls `importProvider()` for each selected config; line 98: calls `onImportComplete()` which triggers `setSyncKey(k => k+1)` in AppShell (line 55) |
| 12 | Skip button dismisses dialog and shows main UI (empty state) | NEEDS HUMAN | ImportDialog.tsx lines 107-109: `handleSkip` calls `onOpenChange(false)`. Logic is trivial and correct but visual behavior needs human confirmation. |
| 13 | Settings page has an import button that triggers the same import dialog | VERIFIED | SettingsPage.tsx lines 165-177: renders import button section when `onShowImport` is provided; AppShell.tsx lines 78-79: passes `onShowImport={handleShowImport}` to SettingsPage |
| 14 | Duplicate configs (same API Key + Base URL) are skipped during import | VERIFIED | ImportDialog.tsx lines 67-79: fetches `listProviders()` then checks `existing.some(p => p.api_key === config.api_key && p.base_url === config.base_url)` before import |
| 15 | Items with missing API key show a warning label in preview | VERIFIED | ImportDialog.tsx lines 140-148: conditional rendering -- `has_api_key` true shows masked key, false shows `t("import.missingApiKey")` in `text-yellow-500` |
| 16 | Import button is disabled and shows loading state while import is in progress | VERIFIED | ImportDialog.tsx line 175: `disabled={importing || !hasSelection}`; line 178: shows `t("import.importing")` text when importing; line 177: renders Loader2 spinner when importing |
| 17 | All dialog text is localized in Chinese and English | VERIFIED | zh.json and en.json both contain `import` section with 11 keys: title, importSelected, skip, missingApiKey, missingBaseUrl, importSuccess, importError, noNewConfigs, settingsButton, defaultSuffix, importing |

**Score:** 14/17 truths verified (3 need human confirmation of runtime/visual behavior)

### Required Artifacts

**Plan 01**

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/commands/onboarding.rs` | DetectedCliConfig struct and scan_cli_configs Tauri command (min 60 lines) | VERIFIED | 530 lines, exports scan_cli_configs, import_provider, DetectedCliConfig; 15 tests included |
| `src-tauri/src/commands/mod.rs` | Module registration for onboarding commands (contains "pub mod onboarding") | VERIFIED | Line 1: `pub mod onboarding;` |
| `src-tauri/src/lib.rs` | Tauri command handler registration for scan_cli_configs (contains "scan_cli_configs") | VERIFIED | Lines 24-25: `commands::onboarding::scan_cli_configs` and `commands::onboarding::import_provider` registered in generate_handler |

**Plan 02**

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/components/provider/ImportDialog.tsx` | Import dialog with config preview, checkbox selection, import/skip actions (min 80 lines) | VERIFIED | 184 lines, renders dialog with checkboxes, masked API keys, import/skip buttons, dedup logic |
| `src/components/ui/checkbox.tsx` | shadcn/ui Checkbox component | VERIFIED | 30 lines, proper Radix UI Checkbox with styling |
| `src/lib/tauri.ts` | scanCliConfigs and importProvider invoke wrappers (contains "scanCliConfigs") | VERIFIED | Lines 46-58: both wrappers properly invoke Tauri commands |
| `src/types/provider.ts` | DetectedCliConfig TypeScript interface (contains "DetectedCliConfig") | VERIFIED | Lines 34-41: DetectedCliConfig interface with all 6 fields matching Rust struct |

### Key Link Verification

**Plan 01**

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `onboarding.rs` | `~/.claude/settings.json` | serde_json::Value parsing | WIRED | Line 21: `home_dir.join(".claude").join("settings.json")`; line 27: `serde_json::from_str` |
| `onboarding.rs` | `~/.codex/auth.json` | serde_json::Value parsing | WIRED | Line 64: `codex_dir.join("auth.json")`; line 79: `serde_json::from_str` |
| `onboarding.rs` | `~/.codex/config.toml` | toml_edit::DocumentMut parsing | WIRED | Line 65: `codex_dir.join("config.toml")`; line 101: `content.parse::<toml_edit::DocumentMut>()` |

**Plan 02**

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `AppShell.tsx` | `tauri.ts` | listProviders + scanCliConfigs on mount | WIRED | Lines 6, 38-41: imports and calls `listProviders("claude")`, `listProviders("codex")`, `scanCliConfigs()` in startup useEffect |
| `ImportDialog.tsx` | `tauri.ts` | importProvider for each selected config | WIRED | Line 14: imports `importProvider`; line 82: calls it in handleImport loop |
| `SettingsPage.tsx` | `ImportDialog.tsx` | state toggle to show ImportDialog | WIRED | SettingsPage receives `onShowImport` prop (line 20-21), AppShell passes `handleShowImport` (line 78) which sets state triggering ImportDialog render (line 82-87) |
| `ImportDialog.tsx` | `tauri.ts` | listProviders for dedup check before import | WIRED | Line 14: imports `listProviders`; line 67: `const existing = await listProviders()` before import loop |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| ONBD-01 | 05-01, 05-02 | First launch scans existing `~/.claude/` and `~/.codex/` configs and creates initial Providers | SATISFIED | Backend: scan_cli_configs reads both config directories with full edge case handling. Frontend: AppShell auto-triggers on empty provider state, ImportDialog creates providers via import_provider. |
| ONBD-02 | 05-02 | User can also manually create Providers from scratch at any time | SATISFIED | Pre-existing create_provider command and ProviderDialog from Phase 3 remain intact. import_provider is additive, does not interfere. 121 tests pass with no regression. |

**Orphaned requirements check:** REQUIREMENTS.md maps ONBD-01 and ONBD-02 to Phase 5. Both are claimed by plans (05-01 claims ONBD-01; 05-02 claims ONBD-01, ONBD-02). No orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | - | - | - | No anti-patterns detected in any phase-modified files |

No TODOs, FIXMEs, placeholders, empty implementations, or stub handlers found in any of the 8 files modified by this phase.

### Human Verification Required

### 1. First-launch auto-trigger

**Test:** Delete all existing providers (or start with empty provider store). Restart the app with `~/.claude/settings.json` or `~/.codex/auth.json` present on the machine.
**Expected:** Import dialog appears automatically showing detected CLI configs with checkboxes, masked API keys, and import/skip buttons.
**Why human:** Requires running the app and observing startup behavior with real config files.

### 2. Skip button dismissal

**Test:** In the auto-triggered import dialog, click "Skip" (or the Chinese equivalent).
**Expected:** Dialog closes, main UI shows empty provider state, no providers were created.
**Why human:** Interactive UI behavior requiring visual confirmation.

### 3. Settings page re-trigger

**Test:** Navigate to Settings page, click the "Import from CLI Config" button.
**Expected:** Import dialog opens with freshly scanned CLI configs from the current machine.
**Why human:** Multi-step navigation flow requiring visual confirmation of dialog content.

### Gaps Summary

No gaps found. All automated verifications pass:

- **Backend:** 530-line onboarding.rs with DetectedCliConfig struct, scan_claude_config_in, scan_codex_config_in, scan_cli_configs, import_provider_to, import_provider. All 15 onboarding unit tests pass. Full suite of 121 tests passes (no regression).
- **Frontend:** 184-line ImportDialog.tsx with checkbox selection, masked API key display, dedup checking, loading state, and i18n. AppShell properly wires onboarding check on startup. SettingsPage properly wires import button. Both locale files contain all 11 import i18n keys.
- **Compilation:** Rust cargo test passes. TypeScript tsc --noEmit passes with zero errors.
- **Requirements:** ONBD-01 and ONBD-02 both satisfied with full traceability.
- **Commits:** All 5 commits (c27745b, 24d0452, 9b50b8e, bc1f6b4, 74f84c0) verified in git log.

The 3 items flagged for human verification are runtime/visual behaviors that cannot be verified programmatically but have correct underlying code logic.

---

_Verified: 2026-03-12T09:30:00Z_
_Verifier: Claude (gsd-verifier)_
