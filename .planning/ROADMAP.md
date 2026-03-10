# Roadmap: CLIManager

## Overview

CLIManager delivers a desktop app for managing AI CLI provider configurations with surgical precision. The roadmap builds bottom-up: storage and data model first, then the surgical patch engine (the core value), then the UI layer with i18n, then cross-device sync via iCloud file watching, and finally onboarding polish. Each phase delivers a coherent, testable capability that unblocks the next.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Storage and Data Model** - Two-layer storage (iCloud + local) and protocol-based provider data model
- [ ] **Phase 2: Surgical Patch Engine** - Read-Modify-Write CLI adapters that patch config files without destroying other content
- [ ] **Phase 3: Provider Management UI** - Full provider CRUD interface with one-click switching and i18n
- [ ] **Phase 4: iCloud Sync and File Watching** - FSEvents-based live sync with reactive UI refresh and active provider linkage
- [ ] **Phase 5: Onboarding** - First-launch auto-import from existing CLI configs

## Phase Details

### Phase 1: Storage and Data Model
**Goal**: Provider data can be persisted, read, and managed through a two-layer storage architecture with protocol-based modeling
**Depends on**: Nothing (first phase)
**Requirements**: SYNC-01, SYNC-02, ADPT-03
**Success Criteria** (what must be TRUE):
  1. Provider JSON files can be created, read, updated, and deleted in the iCloud Drive directory (one file per provider)
  2. Device-local settings (active provider ID, path overrides) are stored in `~/.cli-manager/local.json` and never written to the iCloud directory
  3. Provider data model includes protocol type (Anthropic, OpenAI-compatible) and the model is extensible for future protocols
  4. Tauri 2 project scaffolds and builds successfully with React frontend shell
**Plans:** 1/2 plans executed

Plans:
- [ ] 01-01-PLAN.md — Scaffold Tauri 2 project, implement Provider model and iCloud storage CRUD
- [ ] 01-02-PLAN.md — Implement local settings layer and wire Tauri commands

### Phase 2: Surgical Patch Engine
**Goal**: CLI config files are patched surgically -- only credential and model fields change, everything else survives intact
**Depends on**: Phase 1
**Requirements**: PTCH-01, PTCH-02, PTCH-03, PTCH-04, ADPT-01, ADPT-02
**Success Criteria** (what must be TRUE):
  1. Switching a provider modifies only the credential and model fields in `~/.claude/settings.json` -- all other keys, formatting, and structure are preserved
  2. Switching a provider modifies only the credential fields in `~/.codex/auth.json` and model fields in `~/.codex/config.toml` -- TOML comments and unrelated keys survive intact
  3. If a Codex two-file write partially fails, the already-written file is rolled back to its pre-write state
  4. Config files are validated before and after patching; invalid state is never written
  5. A backup of each CLI config file is created before the first write
**Plans**: TBD

Plans:
- [ ] 02-01: TBD
- [ ] 02-02: TBD

### Phase 3: Provider Management UI
**Goal**: Users can manage and switch providers through a complete desktop interface in Chinese or English
**Depends on**: Phase 2
**Requirements**: PROV-01, PROV-02, PROV-03, PROV-04, PROV-05, PROV-06, I18N-01, I18N-02, I18N-03
**Success Criteria** (what must be TRUE):
  1. User can create a new provider by filling in name, API key, base URL, model, and protocol type
  2. User can see all providers listed with clear identification, and the currently active provider for each CLI is visually indicated
  3. User can edit and delete existing providers
  4. User can switch the active provider with one click and the switch completes in under 1 second
  5. UI displays in Chinese by default, user can switch to English in settings, and all visible text is localized
**Plans**: TBD

Plans:
- [ ] 03-01: TBD
- [ ] 03-02: TBD
- [ ] 03-03: TBD

### Phase 4: iCloud Sync and File Watching
**Goal**: Provider changes from other devices appear automatically and trigger CLI config re-patching when needed
**Depends on**: Phase 3
**Requirements**: SYNC-03, SYNC-04, SYNC-05
**Success Criteria** (what must be TRUE):
  1. When a provider JSON file is added, modified, or deleted in the iCloud sync directory by another device, the UI refreshes to show the updated state without user action
  2. When the currently active provider's data is modified via sync, CLI config files are automatically re-patched with the updated values
  3. File watcher handles iCloud event storms gracefully (debounced, no infinite loops from self-writes)
**Plans**: TBD

Plans:
- [ ] 04-01: TBD
- [ ] 04-02: TBD

### Phase 5: Onboarding
**Goal**: New users get started instantly by importing their existing CLI configurations
**Depends on**: Phase 3
**Requirements**: ONBD-01, ONBD-02
**Success Criteria** (what must be TRUE):
  1. On first launch, the app scans `~/.claude/` and `~/.codex/` configurations and offers to create providers from detected credentials
  2. User can skip auto-import and manually create providers from scratch at any time
**Plans**: TBD

Plans:
- [ ] 05-01: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Storage and Data Model | 1/2 | In Progress|  |
| 2. Surgical Patch Engine | 0/TBD | Not started | - |
| 3. Provider Management UI | 0/TBD | Not started | - |
| 4. iCloud Sync and File Watching | 0/TBD | Not started | - |
| 5. Onboarding | 0/TBD | Not started | - |
