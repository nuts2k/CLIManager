# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-03-12
**Phases:** 5 | **Plans:** 12 | **Commits:** 85

### What Was Built
- Two-layer storage architecture (iCloud + local) with per-provider JSON files
- Surgical patch engine — CliAdapter trait with JSON merge and TOML comment-preserving editing
- Full provider management UI with CLI-tabbed interface, CRUD dialogs, one-click switching
- FSEvents-based file watcher with self-write tracking for live iCloud sync
- First-launch onboarding with automatic CLI config detection and selective import
- Chinese/English i18n with runtime language switching

### What Worked
- Bottom-up phase ordering (storage -> engine -> UI -> sync -> onboarding) kept each phase cleanly buildable on the previous
- _in/_to internal function variant pattern provided consistent test isolation across all modules
- serde_json preserve_order + toml_edit::DocumentMut perfectly solved the surgical patch core value
- Per-file JSON in iCloud Drive eliminated sync conflict concerns entirely
- 12 plans averaging 6 minutes each — very efficient execution

### What Was Inefficient
- Some ROADMAP plan checkboxes were left unchecked despite plans being executed (Phase 2, 3 plan items showed `[ ]` instead of `[x]`)
- Research flagged potential issues (config schema verification, iCloud eviction detection) that weren't resolved during v1.0 — deferred as known issues

### Patterns Established
- `_in/_to` internal function variants for filesystem test isolation (used in storage, adapters, onboarding)
- Thin Tauri command wrappers delegating to pure-logic modules
- SelfWriteTracker pattern for preventing watcher infinite loops
- Dark-only theme with CSS variables on :root (zinc palette)
- Hook-per-domain pattern (useProviders, useSettings, useSyncListener)

### Key Lessons
1. Surgical patching via Value-level merge (not string manipulation) is the right approach for preserving config structure
2. iCloud sync is easy when you design for it from the start (per-file JSON, two-layer separation)
3. Self-write tracking with expiry windows is essential for FSEvents-based watchers on macOS
4. Separating import_provider from create_provider keeps validation strict for normal flows while allowing onboarding flexibility

### Cost Observations
- Model mix: quality profile (opus-based)
- Total execution: ~1.12 hours for 12 plans
- Notable: 3-day MVP from requirements to shipped — efficient phase decomposition and parallel-ready architecture

---

## Milestone: v1.1 — System Tray

**Shipped:** 2026-03-13
**Phases:** 2 | **Plans:** 3 | **Tasks:** 7

### What Was Built
- macOS 系统托盘常驻，模板图标自适应暗色/亮色模式
- Close-to-tray 生命周期 — 关闭窗口隐藏并切换 Accessory 模式
- 动态托盘菜单按 CLI 分组显示 Provider，CheckMenuItem 原生勾选
- 一键切换 Provider — spawn_blocking 后台处理，不打开主窗口
- 全方位自动刷新 — iCloud 同步、前端 CRUD、语言切换、导入后自动重建菜单
- 托盘菜单 i18n — TrayTexts 结构体支持中/英

### What Worked
- Phase 6 → Phase 7 依赖链清晰：基础设施先行，功能层在上
- update_tray_menu 作为单一入口点，3 个调用方（tray handler, watcher, command）统一重建
- 复用 providers-changed 事件而非新增事件类型，减少前端监听器修改
- fire-and-forget 模式避免托盘刷新阻塞 UI
- 8 个单元测试覆盖 TrayTexts i18n 和 parse_provider_event，无需运行时环境

### What Was Inefficient
- Nyquist validation 创建了 VALIDATION.md 但未完成合规检查（draft 状态）
- onboarding import 的 refreshTrayMenu 调用在 Plan 2 的 E2E 验证阶段才发现遗漏

### Patterns Established
- `TrayTexts::from_language` — 轻量 Rust 端菜单 i18n（无需 i18n 框架）
- `parse_provider_event` + strip_prefix — 安全的菜单 ID 解析
- Menu-as-Snapshot 全量重建模式 — 菜单项少时简单可靠
- `refreshTrayMenu().catch(() => {})` — 前端非阻塞托盘同步
- `#[cfg(desktop)]` guard — 平台条件编译保护

### Key Lessons
1. 系统托盘逻辑必须全部在 Rust 端 — webview 隐藏时 JS 不可靠
2. Programmatic TrayIconBuilder 优于 tauri.conf.json 配置 — 避免重复图标 bug
3. 所有状态变更源都要触发菜单刷新 — 遗漏一个就是 bug（如 onboarding import）
4. spawn_blocking 处理托盘事件中的 I/O 操作是正确做法 — 不阻塞 UI 线程

### Cost Observations
- Model mix: quality profile (opus-based)
- Total execution: ~25 min for 3 plans (avg 8min/plan)
- Notable: 1-day milestone from research to shipped — tray features are self-contained

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 85 | 5 | Initial project — established patterns and conventions |
| v1.1 | ~10 | 2 | Tray feature — clean extension of existing architecture |

### Cumulative Quality

| Milestone | LOC | Files | Avg Plan Duration |
|-----------|-----|-------|-------------------|
| v1.0 | 7,986 | 143 | 6min |
| v1.1 | 8,441 | 16 modified | 8min |

### Top Lessons (Verified Across Milestones)

1. Design for iCloud from the start — per-file JSON + two-layer storage eliminates sync conflicts
2. Surgical patching via structured data merge preserves what whole-file writes destroy
3. All state change sources must trigger tray refresh — missing one is a bug (verified v1.1)
