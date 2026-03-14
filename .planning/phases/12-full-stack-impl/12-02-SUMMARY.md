---
phase: 12-full-stack-impl
plan: "02"
subsystem: infra
tags: [github-actions, tauri, cicd, release, dmg, signing, updater]

# Dependency graph
requires:
  - phase: 12-01
    provides: "Ed25519 公钥写入 tauri.conf.json、updater/process 插件注册、ad-hoc 签名配置"
provides:
  - "完整 GitHub Actions CI/CD 流水线（.github/workflows/release.yml）"
  - "tag 触发的双架构 macOS DMG 构建（aarch64 + x86_64）"
  - "自动上传 Release Draft + latest.json"
  - "Gatekeeper 安装指引 Release Notes 模板"
affects: [12-03, 12-04, ship-skill, release-engineering]

# Tech tracking
tech-stack:
  added:
    - tauri-apps/tauri-action@v1
    - pnpm/action-setup@v4
    - dtolnay/rust-toolchain@stable
    - actions/setup-node@v4
    - actions/checkout@v4
  patterns:
    - "双架构矩阵策略：aarch64 + x86_64 分离 job，fail-fast: false"
    - "三段式 tag 匹配（v*.*.* ）过滤 GSD 里程碑 tag"
    - "releaseDraft: true + releaseBody 折叠 Gatekeeper 指引"
    - "ad-hoc 签名：APPLE_SIGNING_IDENTITY=-"

key-files:
  created:
    - .github/workflows/release.yml
  modified: []

key-decisions:
  - "使用 tauri-action@v1（非 @v0），生成新版 latest.json 格式（需 updater ≥2.10.0）"
  - "双架构分离矩阵而非 universal-apple-darwin，与 tauri-action 官方示例对齐"
  - "releaseBody 链接 CHANGELOG.md 而非内联日志，与 /ship 技能配合"
  - "不添加 Rust/pnpm 缓存步骤，初版简洁优先"

patterns-established:
  - "Pattern: GitHub tag trigger — on.push.tags: v[0-9]*.[0-9]*.[0-9]* 精确三段式匹配"
  - "Pattern: tauri-action Release — releaseDraft: true + tagName: github.ref_name"

requirements-completed: [CICD-01, CICD-02, CICD-03, SIGN-01, REL-03]

# Metrics
duration: 3min
completed: "2026-03-14"
---

# Phase 12 Plan 02: GitHub Actions CI/CD 流水线 Summary

**tauri-action@v1 双架构矩阵流水线，tag 触发自动构建 aarch64 + x86_64 DMG 并发布 Release Draft，含 Gatekeeper 折叠指引**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-14T08:13:00Z
- **Completed:** 2026-03-14T08:13:57Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- 创建 `.github/workflows/release.yml`，实现完整 tag-触发 CI/CD 流水线
- 双架构矩阵构建（aarch64-apple-darwin + x86_64-apple-darwin），fail-fast: false 保证两架构独立运行
- tauri-action@v1 自动构建 DMG、上传 Release Draft、生成 latest.json
- ad-hoc 签名（APPLE_SIGNING_IDENTITY="-"），无需 Apple 开发者证书
- releaseBody 含 Gatekeeper 折叠安装指引（`<details>` 段落）

## Task Commits

每个任务原子提交：

1. **Task 1: 创建 GitHub Actions release 工作流** - `baefea6` (feat)

**计划元数据提交：** (docs: complete plan) — 待创建

## Files Created/Modified

- `.github/workflows/release.yml` — 完整 CI/CD 流水线，72 行；tag 触发、双架构矩阵、tauri-action 构建与发版

## Decisions Made

- tauri-action@v1 而非 @v0：latest.json 新格式更兼容（12-01 已将 updater 配置到位）
- 不添加 Rust/pnpm 缓存：初版简洁优先，后续根据 CI 耗时再优化
- CHANGELOG 链接（非内联）：与后续 /ship 技能（Plan 12-04）配合，由技能维护 CHANGELOG.md

## Deviations from Plan

无 — 计划按规格逐一实现，未遇到需要修正的问题。

## Issues Encountered

无

## User Setup Required

**外部服务需要手动配置（GitHub Secrets）：**

在使用此 CI 流水线前，必须在 GitHub 仓库设置中添加以下 Secret：

- `TAURI_SIGNING_PRIVATE_KEY`：Plan 12-01 生成的 Ed25519 私钥文件内容（完整两行）

验证方法：Settings → Secrets and variables → Actions → 检查 `TAURI_SIGNING_PRIVATE_KEY` 存在。

## Next Phase Readiness

- CI/CD 流水线就绪，推送三段式 tag 即可触发构建
- Plan 12-03（Updater UI）和 Plan 12-04（/ship 技能）可并行执行（Wave 2）
- 首次发版前需确认 GitHub Secret 已配置，推送 v0.2.0 tag 可验证整条流水线

---
*Phase: 12-full-stack-impl*
*Completed: 2026-03-14*
