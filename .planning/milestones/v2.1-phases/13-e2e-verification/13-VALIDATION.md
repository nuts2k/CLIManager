---
phase: 13
slug: e2e-verification
status: draft
nyquist_compliant: false
wave_0_complete: true
created: 2026-03-14
---

# Phase 13 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Manual E2E verification (no automated test framework) |
| **Config file** | N/A |
| **Quick run command** | `gh run list --workflow=release.yml --limit=2` |
| **Full suite command** | `curl -sL "https://github.com/nuts2k/CLIManager/releases/latest/download/latest.json" \| python3 -m json.tool` |
| **Estimated runtime** | ~10 minutes per release cycle |

---

## Sampling Rate

- **After every task commit:** Run `gh run list --workflow=release.yml --limit=2`
- **After every plan wave:** Full manual verification per checklist
- **Before `/gsd:verify-work`:** All manual checklist items passed
- **Max feedback latency:** N/A (manual verification phase)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 13-01-01 | 01 | 1 | REL-02 | manual | `/ship patch` output check | N/A | pending |
| 13-01-02 | 01 | 1 | CICD-01/02/03, SIGN-01 | manual | `gh run list` + Release UI | N/A | pending |
| 13-01-03 | 01 | 2 | UPD-01/02/03/04 | manual | App launch + update flow | N/A | pending |
| 13-01-04 | 01 | 2 | CICD-02 | manual | `gh release view` x86_64 assets | N/A | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. This is a verification phase — no new test infrastructure needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| /ship patch creates correct tag | REL-02 | CLI interaction required | Run `/ship patch`, verify output |
| CI triggers on tag push | CICD-01 | External service (GitHub Actions) | Check Actions tab after push |
| Dual-arch DMG in Release | CICD-02 | External service (GitHub Releases) | `gh release view v0.2.1` |
| Release auto-published | CICD-03 | External service config | Verify release is not Draft |
| App detects update | UPD-02 | Requires running app | Install v0.2.1, launch after v0.2.2 published |
| Update dialog shows | UPD-03 | UI visual inspection | Verify dialog appears with progress bar |
| Download + install completes | UPD-04 | Full integration | Click update, verify new version runs |
| Gatekeeper instructions present | REL-03 | Content inspection | Check Release Notes on GitHub |
| latest.json has both architectures | SIGN-02/03 | API response check | curl latest.json, verify both platforms |

---

## Validation Sign-Off

- [x] All tasks have verification method (manual for this phase)
- [x] Wave 0 not needed (verification phase)
- [ ] No watch-mode flags
- [ ] All manual checklist items passed
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
