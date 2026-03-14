---
phase: 12-full-stack-impl
plan: "04"
subsystem: infra
tags: [ship, changelog, release, version-bump, conventional-commits, cargo]

# Dependency graph
requires:
  - phase: 12-01
    provides: "Cargo.toml 作为唯一版本来源（version = 0.2.0），无 tauri.conf.json version 字段"
provides:
  - ".claude/commands/ship.md — /ship 一键发版技能（bump/CHANGELOG/commit/tag/push）"
  - "CHANGELOG.md — 初始变更日志文件，待 /ship 首次运行时追加版本条目"
affects: [release-engineering, ci-workflow, 12-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "/ship Claude 自定义命令模式：.claude/commands/ 下 .md 文件即命令"
    - "零外部依赖 CHANGELOG 生成：纯 git log + Python 文本处理"
    - "Conventional Commits 中文分类：feat/fix/refactor/docs/chore → 新功能/修复/重构/文档/其他"

key-files:
  created:
    - ".claude/commands/ship.md"
    - "CHANGELOG.md"
  modified: []

key-decisions:
  - "版本来源唯一：仅修改 src-tauri/Cargo.toml，tauri.conf.json 无 version 字段无需修改（延续 12-01 决策）"
  - "零外部依赖：CHANGELOG 生成通过 git log 解析 + Python 内置处理，不引入 git-cliff 等工具"
  - "中文 CHANGELOG：分类标题全中文（新功能/修复/重构/文档/其他），符合项目语言规范"

patterns-established:
  - "Claude 命令文件：.claude/commands/*.md，内容为 Claude 执行指令，通过 /命令名 调用"
  - "发版前检查：工作区状态检查 + tag 重复检查，防止意外覆盖"

requirements-completed: [REL-02, REL-03]

# Metrics
duration: 2min
completed: "2026-03-14"
---

# Phase 12 Plan 04: /ship 一键发版技能 Summary

**Claude 自定义命令 /ship 实现：零外部依赖的 Cargo.toml 版本 bump + 中文 Conventional Commits CHANGELOG 生成 + git commit/tag/push 全流程**

## Performance

- **Duration:** 约 2 分钟
- **Started:** 2026-03-14T08:13:19Z
- **Completed:** 2026-03-14T08:14:56Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- 创建 `.claude/commands/ship.md`，实现 `/ship patch|minor|major` 一键发版技能（193 行指令）
- 实现零外部依赖的 CHANGELOG 生成：git log 解析 + Conventional Commits 中文分类
- 创建初始 `CHANGELOG.md`，包含格式说明，待 `/ship` 首次运行时追加版本条目

## Task Commits

每个任务均独立提交：

1. **Task 1: 创建 /ship 发版技能** - `ef115e4` (feat)
2. **Task 2: 创建初始 CHANGELOG.md** - `c1571db` (chore)

## Files Created/Modified

- `.claude/commands/ship.md` — Claude 自定义命令，包含版本 bump、CHANGELOG 生成、git 操作全流程指令
- `CHANGELOG.md` — 初始变更日志，含标题和 Conventional Commits 格式说明

## Decisions Made

- **唯一版本来源**：仅修改 `src-tauri/Cargo.toml`，注释明确说明 `tauri.conf.json` 无需修改（延续 12-01 架构决策）
- **零外部依赖**：不引入 git-cliff 或 conventional-changelog，使用 git log + Python 内置处理，降低维护成本
- **中文分类**：CHANGELOG 分类标题全中文（新功能/修复/重构/文档/其他），符合项目语言规范

## Deviations from Plan

无 — 计划严格按规范执行。

## Issues Encountered

无。

## User Setup Required

计划 frontmatter 中指明用户需要配置 GitHub Secret：

- `TAURI_SIGNING_PRIVATE_KEY`：将 `~/.tauri/climanager.key` 文件内容设置为 GitHub repo Settings > Secrets and variables > Actions > New repository secret

此步骤为 CI 构建签名所需，与 12-01 生成的密钥对应。

## Next Phase Readiness

- `/ship` 技能和 `CHANGELOG.md` 已就绪，Phase 12 完成后用户可执行 `/ship patch` 触发首次发版
- CI release.yml（12-03）与 `/ship` 推送的 `v*.*.*` tag 联动，完整发版链路已建立
- 所有 Phase 12 计划（12-01 至 12-04）均已完成

---
*Phase: 12-full-stack-impl*
*Completed: 2026-03-14*
