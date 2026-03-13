---
phase: 6
slug: tray-foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-13
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[cfg(test)]` + `cargo test` |
| **Config file** | None (standard Cargo test runner) |
| **Quick run command** | `cd src-tauri && cargo test` |
| **Full suite command** | `cd src-tauri && cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test`
- **After every plan wave:** Run `cd src-tauri && cargo test` + manual testing checklist
- **Before `/gsd:verify-work`:** Full suite must be green + all manual tests pass
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 06-01-01 | 01 | 1 | TRAY-01 | manual | `cd src-tauri && cargo build` | N/A | pending |
| 06-01-02 | 01 | 1 | TRAY-02 | manual | `cd src-tauri && cargo build` | N/A | pending |
| 06-01-03 | 01 | 1 | TRAY-03 | manual | `cd src-tauri && cargo build` | N/A | pending |
| 06-01-04 | 01 | 1 | MENU-01 | manual | `cd src-tauri && cargo build` | N/A | pending |
| 06-01-05 | 01 | 1 | MENU-02 | manual | `cd src-tauri && cargo build` | N/A | pending |

*Status: pending · green · red · flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/icons/tray/tray-icon-template.png` — 44x44 monochrome template icon asset
- [ ] Verify `cargo build` succeeds after adding `tray-icon` and `image-png` features to Cargo.toml

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Tray icon appears in menu bar, adapts to dark/light mode | TRAY-01 | Requires macOS GUI rendering | 1. `cargo tauri dev` 2. Verify icon in menu bar 3. Toggle dark/light mode, verify icon adapts |
| Close button hides window, app persists in tray | TRAY-02 | Requires window events + tray interaction | 1. Click red X close button 2. Verify window hides 3. Verify tray icon remains |
| Hidden: no Dock/Cmd+Tab; Shown: Dock/Cmd+Tab return | TRAY-03 | Requires macOS Dock + Cmd+Tab verification | 1. Hide window 2. Check Dock icon gone 3. Check Cmd+Tab (not listed) 4. Show window 5. Verify both return |
| "打开主窗口" shows and focuses main window | MENU-01 | Requires tray menu + window interaction | 1. Hide window 2. Click tray icon 3. Click "打开主窗口" 4. Verify window appears and is focused |
| "退出" fully exits application | MENU-02 | Requires running app + process verification | 1. Click tray icon 2. Click "退出" 3. Verify app fully exits (no process, no tray icon) |
| Cmd+Q quits app (or hides as fallback) | TRAY-02 | macOS-specific keyboard event | 1. Press Cmd+Q 2. Verify app quits OR hides to tray (fallback acceptable) |
| Double-click tray icon opens window | MENU-01 | Requires tray event interaction | 1. Double-click tray icon 2. Verify window appears (if supported with show_menu_on_left_click) |
| Release build works | All | Requires full build + manual test | 1. `cargo tauri build` 2. Run release binary 3. Verify tray functionality |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
