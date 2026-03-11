---
phase: 03-provider-management-ui
plan: 02
subsystem: ui
tags: [shadcn-ui, tailwind-v4, i18n, react-i18next, tauri, typescript]

# Dependency graph
requires:
  - phase: 01-storage-layer
    provides: Provider and LocalSettings Rust structs
  - phase: 03-01
    provides: cli_id on Provider, active_providers on LocalSettings
provides:
  - shadcn/ui component library with Tailwind CSS v4 dark theme
  - TypeScript type definitions mirroring Rust backend structs
  - Typed Tauri invoke wrappers for all 8 backend commands
  - react-i18next with Chinese default and English fallback translations
affects: [03-03, 03-04, 03-05]

# Tech tracking
tech-stack:
  added: [tailwindcss-v4, "@tailwindcss/vite", shadcn-ui, i18next, react-i18next, zod, lucide-react, sonner, clsx, tailwind-merge, class-variance-authority]
  patterns: [dark-only-theme, typed-invoke-wrappers, i18n-side-effect-import]

key-files:
  created:
    - src/index.css
    - src/lib/utils.ts
    - src/lib/tauri.ts
    - src/types/provider.ts
    - src/types/settings.ts
    - src/i18n/index.ts
    - src/i18n/locales/zh.json
    - src/i18n/locales/en.json
    - components.json
    - src/components/ui/*.tsx
  modified:
    - vite.config.ts
    - tsconfig.json
    - package.json
    - index.html
    - src/App.tsx
    - src/main.tsx
    - src-tauri/tauri.conf.json

key-decisions:
  - "Dark-only theme: CSS variables set directly on :root using zinc dark palette, no .dark class toggle needed"
  - "Spread CreateProviderInput in invoke call to satisfy Record<string, unknown> type constraint"
  - "i18n imported as side-effect in main.tsx before App component"

patterns-established:
  - "Dark-only theme: all shadcn CSS variables in :root with dark zinc values"
  - "Typed Tauri wrappers: all backend commands accessed via src/lib/tauri.ts"
  - "@ path alias: import from @/components, @/lib, @/types etc."
  - "i18n keys: nested dot notation (e.g. actions.create, status.active)"

requirements-completed: [I18N-01, I18N-02, I18N-03]

# Metrics
duration: 8min
completed: 2026-03-11
---

# Phase 3 Plan 02: Frontend Infrastructure Summary

**shadcn/ui with Tailwind CSS v4 dark theme, react-i18next zh/en translations, TypeScript types mirroring Rust structs, and typed Tauri invoke wrappers for all 8 backend commands**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-11T06:31:50Z
- **Completed:** 2026-03-11T06:39:53Z
- **Tasks:** 2
- **Files modified:** 26

## Accomplishments
- shadcn/ui initialized with 14 components (button, dialog, input, label, select, tabs, card, collapsible, badge, dropdown-menu, sonner, scroll-area, tooltip, separator)
- Tailwind CSS v4 configured with dark-only zinc theme via @tailwindcss/vite plugin
- TypeScript types for Provider (with cli_id) and LocalSettings (with active_providers) mirroring Rust structs
- 8 typed Tauri invoke wrappers covering all backend commands
- Chinese/English i18n with 50+ UI strings covering all planned UI text

## Task Commits

Each task was committed atomically:

1. **Task 1: Install and configure shadcn/ui with Tailwind CSS v4** - `f10020d` (feat)
2. **Task 2: Create TypeScript types, Tauri wrappers, and i18n** - `cffd5df` (feat)

## Files Created/Modified
- `vite.config.ts` - Added tailwindcss plugin and @ path alias
- `tsconfig.json` - Added baseUrl and paths for @ alias
- `components.json` - shadcn/ui configuration (new-york style, zinc base)
- `src/index.css` - Tailwind v4 import + shadcn dark theme CSS variables
- `src/lib/utils.ts` - cn() utility for className merging
- `src/lib/tauri.ts` - Typed invoke wrappers for all 8 backend commands
- `src/types/provider.ts` - Provider, ModelConfig, CreateProviderInput types
- `src/types/settings.ts` - LocalSettings, TestResult, CliPaths, TestConfig types
- `src/i18n/index.ts` - i18n initialization with zh default, en fallback
- `src/i18n/locales/zh.json` - Chinese translations (50+ keys)
- `src/i18n/locales/en.json` - English translations (50+ keys)
- `src/components/ui/*.tsx` - 14 shadcn/ui components
- `src/App.tsx` - Minimal shell with bg-background/text-foreground
- `src/main.tsx` - Added i18n and index.css imports
- `index.html` - Added class="dark" to html element
- `src-tauri/tauri.conf.json` - Window size 1000x700
- `package.json` - All new dependencies

## Decisions Made
- Dark-only theme: CSS variables set directly on :root using zinc dark palette, no .dark class toggle needed
- Spread CreateProviderInput in invoke call to satisfy Record<string, unknown> type constraint from @tauri-apps/api
- i18n imported as side-effect in main.tsx before App component for initialization order

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed TypeScript error with CreateProviderInput invoke args**
- **Found during:** Task 2 (Tauri invoke wrappers)
- **Issue:** TypeScript interfaces lack index signatures, so passing CreateProviderInput directly to invoke() failed type check against Record<string, unknown>
- **Fix:** Spread the input object: `{ ...input }` to create a plain object
- **Files modified:** src/lib/tauri.ts
- **Verification:** pnpm build succeeds
- **Committed in:** cffd5df (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Minor type compatibility fix, no scope creep.

## Issues Encountered
- No src/index.css existed prior to this plan (App.css was the only CSS file), so shadcn init could not modify it. Created index.css manually with Tailwind v4 dark zinc theme variables.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All UI infrastructure ready for Plan 03 (provider list/card components) and Plan 04 (provider form/dialog)
- 14 shadcn/ui components available for composition
- All Tauri command wrappers typed and ready
- i18n strings pre-defined for all planned UI elements

---
*Phase: 03-provider-management-ui*
*Completed: 2026-03-11*
