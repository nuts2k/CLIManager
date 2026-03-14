---
phase: 12
slug: full-stack-impl
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-14
---

# Phase 12 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust), no standalone frontend test framework |
| **Config file** | src-tauri/Cargo.toml (dev-dependencies) |
| **Quick run command** | `cd src-tauri && cargo check` |
| **Full suite command** | `cd src-tauri && cargo test` |
| **Estimated runtime** | ~30 seconds (check), ~60 seconds (full test) |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo check`
- **After every plan wave:** Run `cd src-tauri && cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 12-01-01 | 01 | 1 | REL-01 | manual-only | check tauri.conf.json + Cargo.toml | N/A | pending |
| 12-01-02 | 01 | 1 | SIGN-02 | manual-only | `ls ~/.tauri/climanager.key*` | N/A | pending |
| 12-01-03 | 01 | 1 | SIGN-03 | manual-only | GitHub Settings UI check | N/A | pending |
| 12-02-01 | 02 | 2 | CICD-01 | smoke | push v-tag and observe CI | Wave 2 | pending |
| 12-02-02 | 02 | 2 | CICD-02 | smoke | CI artifacts check | Wave 2 | pending |
| 12-02-03 | 02 | 2 | CICD-03 | smoke | GitHub Release UI check | Wave 2 | pending |
| 12-02-04 | 02 | 2 | SIGN-01 | smoke | CI job success | Wave 2 | pending |
| 12-03-01 | 03 | 2 | UPD-01 | unit | `cd src-tauri && cargo build` | Wave 2 | pending |
| 12-03-02 | 03 | 2 | UPD-02 | manual-only | launch app, observe logs | N/A | pending |
| 12-03-03 | 03 | 2 | UPD-03 | manual-only | local mock update test | N/A | pending |
| 12-03-04 | 03 | 2 | UPD-04 | manual-only | full CI release verify | N/A | pending |
| 12-04-01 | 04 | 2 | REL-02 | unit | `bash .claude/commands/ship.sh --dry-run patch` | Wave 2 | pending |
| 12-04-02 | 04 | 2 | REL-03 | manual-only | check release.yml template | N/A | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [ ] `.github/workflows/release.yml` — CICD-01/02/03, SIGN-01 (created by Plan 12-02)
- [ ] `src/components/updater/UpdateDialog.tsx` — UPD-03 (created by Plan 12-03)
- [ ] `src/components/updater/useUpdater.ts` — UPD-02 (created by Plan 12-03)
- [ ] `.claude/commands/ship.md` — REL-02 (created by Plan 12-04)

*Wave 0 artifacts created as part of plan execution, no separate pre-creation needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| tauri.conf.json has no version field | REL-01 | Config file inspection | Open tauri.conf.json, verify no top-level `version` key |
| Ed25519 keypair generated | SIGN-02 | File system check | Run `ls ~/.tauri/climanager.key*` |
| GitHub Secret set, pubkey in config | SIGN-03 | External service (GitHub) | Check GitHub repo Settings > Secrets |
| App startup update check | UPD-02 | Requires running app | Launch app, check console for update check logs |
| UpdateDialog renders correctly | UPD-03 | UI visual inspection | Mock an update, verify dialog shows progress bar |
| Download + relaunch flow | UPD-04 | Full integration test | Publish real release, verify update + restart |
| Release Notes include Gatekeeper guide | REL-03 | Template content check | Trigger release, inspect Notes content |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
