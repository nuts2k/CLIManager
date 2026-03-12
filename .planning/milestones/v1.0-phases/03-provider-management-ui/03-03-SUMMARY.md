---
phase: 03-provider-management-ui
plan: 03
subsystem: ui
tags: [react, shadcn-ui, tailwind, i18n, tauri, provider-crud, zod]

# Dependency graph
requires:
  - phase: 03-01
    provides: Backend provider commands with cli_id scoping, switch+patch, delete+auto-switch, test_provider
  - phase: 03-02
    provides: shadcn/ui components, TypeScript types, Tauri invoke wrappers, i18n translations
provides:
  - AppShell layout with Header and settings navigation
  - ProviderTabs with Claude Code and Codex tab switching
  - ProviderCard with hover-reveal actions and active blue indicator bar
  - ProviderList with ScrollArea and EmptyState fallback
  - ProviderDialog for create/edit with zod validation and collapsible Advanced section
  - DeleteConfirmDialog with loading state
  - useProviders hook for CRUD + switch + copy + test with Toast notifications
  - useSettings hook for LocalSettings and active provider lookup
affects: [03-04-settings-ui]

# Tech tracking
tech-stack:
  added: []
  patterns: [hook-per-domain, dialog-state-lifted-to-parent, group-hover-reveal-actions]

key-files:
  created:
    - src/hooks/useProviders.ts
    - src/hooks/useSettings.ts
    - src/components/layout/AppShell.tsx
    - src/components/layout/Header.tsx
    - src/components/provider/ProviderTabs.tsx
    - src/components/provider/ProviderList.tsx
    - src/components/provider/ProviderCard.tsx
    - src/components/provider/EmptyState.tsx
    - src/components/provider/ProviderDialog.tsx
    - src/components/provider/DeleteConfirmDialog.tsx
  modified:
    - src/App.tsx

key-decisions:
  - "Dialog state managed in ProviderTabs parent, passed down as props to dialogs"
  - "useProviders hook accepts refreshSettings callback to sync settings after switch/delete"
  - "ProviderDialog handles both create and edit via mode prop with form reset on open"
  - "Model config and notes set via updateProvider after createProvider since CreateProviderInput lacks those fields"

patterns-established:
  - "Hook-per-domain: useProviders for provider CRUD, useSettings for LocalSettings"
  - "Dialog state lifted: parent component manages open/close and passes mode + data"
  - "Group-hover reveal: action buttons hidden until card hover via group-hover:opacity-100"

requirements-completed: [PROV-01, PROV-02, PROV-03, PROV-04, PROV-05, PROV-06, I18N-01]

# Metrics
duration: 4min
completed: 2026-03-11
---

# Phase 3 Plan 03: Provider Management UI Summary

**Full provider management interface with tabbed CLI view, card list with hover actions, create/edit dialog with zod validation, delete confirmation, and all CRUD operations wired to Tauri backend via typed hooks**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-11T06:43:16Z
- **Completed:** 2026-03-11T06:47:40Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments
- Complete provider management UI with AppShell layout, Header with settings navigation
- Tabbed interface (Claude Code / Codex) with per-CLI provider lists and one-click switching
- Provider cards with active blue indicator bar, hover-reveal actions (switch, edit, copy, copy-to, test, delete)
- Create/Edit dialog with zod validation, API key visibility toggle, and collapsible Advanced section
- Delete confirmation dialog with loading state and auto-switch behavior from backend
- All visible text uses i18n translation keys (no hardcoded strings)

## Task Commits

Each task was committed atomically:

1. **Task 1: App shell, hooks, and provider list with cards** - `bbbdc3a` (feat)
2. **Task 2: Provider create/edit dialog, delete confirmation, and full action wiring** - `b0600cc` (feat)

## Files Created/Modified
- `src/hooks/useProviders.ts` - Hook for provider CRUD operations with Toast notifications
- `src/hooks/useSettings.ts` - Hook for LocalSettings management and active provider lookup
- `src/components/layout/AppShell.tsx` - App shell with header, main/settings view switching
- `src/components/layout/Header.tsx` - Header with CLIManager title and settings gear icon
- `src/components/provider/ProviderTabs.tsx` - CLI tabs (Claude Code / Codex) with dialog state management
- `src/components/provider/ProviderList.tsx` - ScrollArea provider list with EmptyState fallback
- `src/components/provider/ProviderCard.tsx` - Card row with active indicator, hover actions, dropdown menu
- `src/components/provider/EmptyState.tsx` - Empty state with icon, message, and create button
- `src/components/provider/ProviderDialog.tsx` - Create/edit form dialog with zod validation and Advanced section
- `src/components/provider/DeleteConfirmDialog.tsx` - Confirmation dialog with loading state
- `src/App.tsx` - Renders AppShell and Sonner Toaster

## Decisions Made
- Dialog state managed in ProviderTabs parent, passed down as props to dialogs (standard React lifting pattern)
- useProviders hook accepts refreshSettings callback to keep settings in sync after switch/delete operations
- ProviderDialog handles both create and edit via mode prop, resets form on open
- Model config and notes set via updateProvider after createProvider since CreateProviderInput lacks those fields

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All provider management UI components complete, ready for Plan 04 (Settings UI)
- Settings page placeholder exists in AppShell, ready to be implemented
- useSettings hook ready for settings page consumption

---
*Phase: 03-provider-management-ui*
*Completed: 2026-03-11*
