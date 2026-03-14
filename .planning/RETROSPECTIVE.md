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

## Milestone: v2.0 — Local Proxy

**Shipped:** 2026-03-14
**Phases:** 4 | **Plans:** 7 | **Commits:** 54

### What Was Built
- axum 0.8 HTTP 代理引擎 — 全路径转发、SSE 流式透传、凭据动态替换、健康自检、多端口管理
- 直连/代理双模式切换 — 全局总开关 + 每 CLI 独立开关 + 状态持久化
- 退出清理与崩溃恢复 — 正常退出同步还原 + takeover 标志异常检测 + 启动自动恢复
- iCloud 同步与 Provider CRUD 代理联动 — watcher/update_provider/delete_provider 全部代理感知
- 前端代理模式 UI — Switch 组件、useProxyStatus hook、绿色状态点、端口冲突 toast
- 托盘菜单与编辑路径代理感知修复（Phase 11 gap closure）

### What Worked
- Phase 8 → 9 → 10 依赖链清晰：代理引擎 → 模式切换 → UI 集成，每层建立在上一层之上
- 审计驱动的 gap closure：Phase 11 由 milestone audit 发现的 2 个集成差距直接驱动，精准修复
- _in 内部函数变体 + skip_patch 参数模式：代理/直连分支无需改变公共 API 签名
- determine_tray_switch_mode 纯函数提取：解耦测试与 AppHandle async 上下文
- ProxyGlobalToggleLock 防止全局开关竞态条件

### What Was Inefficient
- Phase 9 ROADMAP.md 计划复选框未更新（显示 `[ ]` 但实际已完成）
- 早期 SUMMARY.md（08-01 到 10-01）缺少 requirements-completed 前言字段 — 该字段在 10-02 才引入
- Phase 11 Nyquist 验证未完成（draft 状态）
- 代理模式 provider 同步时序问题导致 3 次 hotfix commits（竞态条件调试）

### Patterns Established
- `TraySwitchMode` 枚举 + 纯函数判断代理/直连路径 — 可测试的路径选择
- `skip_patch: bool` 参数模式 — 调用层控制是否 patch CLI 配置
- `tauri::async_runtime::spawn` 代替 `spawn_blocking` — 支持 async 代理函数调用
- `proxy-mode-changed` 事件驱动 UI 刷新 — useProxyStatus hook 自动监听
- `ProxySettings + ProxyTakeover` 本地持久化分离 — 开关状态与接管标志独立存储
- `update_proxy_upstream_if_needed` — watcher 中代理上游联动的标准入口

### Key Lessons
1. 代理模式下所有修改 CLI 配置的路径都必须检查代理状态 — 遗漏一个就是 bug（tray、edit provider 两处遗漏由审计发现）
2. 竞态条件在代理模式切换中很容易出现 — ProxyGlobalToggleLock 和操作顺序（先同步后异步）是关键
3. takeover 标志持久化是崩溃恢复的正确方案 — 比依赖 drop/析构函数可靠
4. 审计驱动的 gap closure 模式有效 — 独立 phase 精准修复比在已有 phase 追加更清晰
5. 端口冲突检测依赖字符串匹配是脆弱的 — 应考虑结构化错误类型传递

### Cost Observations
- Model mix: balanced profile (sonnet-based agents, opus orchestration)
- Total execution: ~35 min for 7 plans (avg 5min/plan)
- Notable: 2-day milestone from research to shipped — proxy feature self-contained within existing architecture

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 85 | 5 | Initial project — established patterns and conventions |
| v1.1 | ~10 | 2 | Tray feature — clean extension of existing architecture |
| v2.0 | 54 | 4 | Local proxy — new subsystem (axum) + audit-driven gap closure |

### Cumulative Quality

| Milestone | LOC | Files | Avg Plan Duration |
|-----------|-----|-------|-------------------|
| v1.0 | 7,986 | 143 | 6min |
| v1.1 | 8,441 | 16 modified | 8min |
| v2.0 | ~12,000 | 56 modified | 5min |

### Top Lessons (Verified Across Milestones)

1. Design for iCloud from the start — per-file JSON + two-layer storage eliminates sync conflicts
2. Surgical patching via structured data merge preserves what whole-file writes destroy
3. All state change sources must trigger tray refresh — missing one is a bug (verified v1.1)
4. All CLI config modification paths must check proxy state — missing one breaks proxy mode (verified v2.0)
5. Audit-driven gap closure is effective — independent phases for precise fixes (verified v2.0)
