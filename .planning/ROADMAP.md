# Roadmap: CLIManager

## Milestones

- v1.0 MVP — Phases 1-5 (shipped 2026-03-12)
- v1.1 System Tray — Phases 6-7 (in progress)

## Phases

<details>
<summary>v1.0 MVP (Phases 1-5) — SHIPPED 2026-03-12</summary>

- [x] Phase 1: Storage and Data Model (2/2 plans) — completed 2026-03-10
- [x] Phase 2: Surgical Patch Engine (2/2 plans) — completed 2026-03-11
- [x] Phase 3: Provider Management UI (4/4 plans) — completed 2026-03-11
- [x] Phase 4: iCloud Sync and File Watching (2/2 plans) — completed 2026-03-11
- [x] Phase 5: Onboarding (2/2 plans) — completed 2026-03-12

</details>

### v1.1 System Tray

**Milestone Goal:** Add system tray for background residence and one-click Provider switching from menu bar

- [ ] **Phase 6: Tray Foundation** - Tray icon, close-to-tray lifecycle, and basic menu controls
- [ ] **Phase 7: Provider Menu and Switching** - Provider list in tray with one-click switching, auto-refresh, and i18n

## Phase Details

<details>
<summary>v1.0 MVP — Phase Details (collapsed)</summary>

### Phase 1: Storage and Data Model
**Goal**: Two-layer storage architecture with Provider CRUD
**Plans**: 2 plans (complete)

### Phase 2: Surgical Patch Engine
**Goal**: Read-modify-write patching for CLI config files
**Plans**: 2 plans (complete)

### Phase 3: Provider Management UI
**Goal**: Full provider management frontend with switching
**Plans**: 4 plans (complete)

### Phase 4: iCloud Sync and File Watching
**Goal**: Live sync with iCloud and automatic re-patching
**Plans**: 2 plans (complete)

### Phase 5: Onboarding
**Goal**: First-launch experience and i18n
**Plans**: 2 plans (complete)

</details>

### Phase 6: Tray Foundation
**Goal**: Application persists in macOS menu bar after window close, with basic tray controls
**Depends on**: Phase 5 (v1.0 complete)
**Requirements**: TRAY-01, TRAY-02, TRAY-03, MENU-01, MENU-02
**Success Criteria** (what must be TRUE):
  1. A tray icon appears in the macOS menu bar when the application launches, adapting correctly to both dark and light mode
  2. Closing the main window hides the window instead of quitting the app -- the tray icon remains active
  3. When the window is hidden, the app does not appear in the Dock or Cmd+Tab switcher; when the window is restored, it reappears in both
  4. Clicking "Open Main Window" in the tray menu shows and focuses the main window
  5. Clicking "Quit" in the tray menu fully exits the application
**Plans**: 1 plan

Plans:
- [x] 06-01-PLAN.md — Tray icon, menu, close-to-tray lifecycle, and ActivationPolicy toggling

### Phase 7: Provider Menu and Switching
**Goal**: Users can view and switch Providers directly from the tray menu without opening the main window
**Depends on**: Phase 6
**Requirements**: PROV-01, PROV-02, PROV-03, MENU-03
**Success Criteria** (what must be TRUE):
  1. The tray menu lists all Providers grouped by CLI, with the currently active Provider per CLI showing a checkmark
  2. Clicking a Provider in the tray menu switches to it immediately (CLI config files are patched) without opening the main window
  3. When a Provider is added, edited, or deleted in the main window, or changed via iCloud sync, the tray menu updates automatically
  4. Tray menu labels display in the correct language matching the application's current language setting (Chinese or English)
**Plans**: TBD

Plans:
- [ ] 07-01: TBD
- [ ] 07-02: TBD

## Progress

**Execution Order:** Phase 6 -> Phase 7

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Storage and Data Model | v1.0 | 2/2 | Complete | 2026-03-10 |
| 2. Surgical Patch Engine | v1.0 | 2/2 | Complete | 2026-03-11 |
| 3. Provider Management UI | v1.0 | 4/4 | Complete | 2026-03-11 |
| 4. iCloud Sync and File Watching | v1.0 | 2/2 | Complete | 2026-03-11 |
| 5. Onboarding | v1.0 | 2/2 | Complete | 2026-03-12 |
| 6. Tray Foundation | v1.1 | 1/1 | Complete | 2026-03-13 |
| 7. Provider Menu and Switching | v1.1 | 0/? | Not started | - |

---
*Roadmap created: 2026-03-12 (v1.0)*
*Last updated: 2026-03-13 (Phase 6 planned)*
