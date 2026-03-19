---
phase: 31
slug: tech-debt-fix
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-19
---

# Phase 31 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust) + tsc --noEmit (TypeScript) |
| **Config file** | src-tauri/Cargo.toml, tsconfig.json |
| **Quick run command** | `cd src-tauri && cargo test --lib traffic` |
| **Full suite command** | `cd src-tauri && cargo test && cd ../src && npx tsc --noEmit` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test --lib traffic`
- **After every plan wave:** Run `cd src-tauri && cargo test && cd ../src && npx tsc --noEmit`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 31-01-01 | 01 | 1 | DEBT-01 (cache_creation_tokens) | unit | `cargo test --lib traffic` | TBD | ⬜ pending |
| 31-01-02 | 01 | 1 | DEBT-03 (SUMMARY path fix) | manual | N/A (doc fix) | N/A | ⬜ pending |
| 31-01-03 | 01 | 1 | DEBT-04 (DB error handling) | unit+manual | `cargo test --lib traffic` | TBD | ⬜ pending |
| 31-01-04 | 01 | 1 | DEBT-02/05/06 (won't fix comments) | manual | N/A (comment only) | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| DB 初始化失败时前端显示错误提示 | DEBT-04 | 需手动模拟 DB 初始化失败 | 临时修改 DB 路径为不可写目录，启动应用，确认前端显示错误提示而非空页面 |
| SUMMARY 路径已修正 | DEBT-03 | 文档修正，无自动化意义 | 检查 30-03-SUMMARY.md 中路径为 src/i18n/locales/ |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
