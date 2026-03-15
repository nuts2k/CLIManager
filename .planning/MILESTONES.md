# Milestones

## v2.2 协议转换 (Shipped: 2026-03-15)

**Phases completed:** 3 phases, 10 plans
**Delivered:** Claude Code 通过本地代理使用 OpenAI 兼容 Provider，代理层自动完成 Anthropic Messages API <-> OpenAI API 双向协议转换

**Key accomplishments:**
- Anthropic -> OpenAI Chat Completions 双向协议转换（请求/响应/流式 SSE），纯函数 TDD，63 个单元测试
- 流式 SSE Deferred Start 工具调用缓冲 + 多工具并发追踪 + 跨 chunk 截断处理
- handler.rs 三分支协议路由（OpenAiChatCompletions / OpenAiResponses / Anthropic）+ 模型映射三级优先级
- Responses API 完整转换层（请求/非流式响应/SSE 流式），23 个测试
- Provider 编辑 UI 三协议选择 + 默认模型 + 模型映射对可视化配置
- ProtocolType 三变体 serde alias 向前兼容，旧配置文件无需迁移

**Stats:**
- 44 commits, 29 files changed, +6,682/-168 lines
- Timeline: 2 days (2026-03-14 -> 2026-03-15)
- Codebase: ~18,000 LOC (Rust + TypeScript/React)
- Key module: 4,141 LOC translate engine (6 子模块)
- Git range: feat(14-01) to fix(provider)
- 329 tests, all green (1 pre-existing env port conflict)

---

## v2.1 Release Engineering (Shipped: 2026-03-14)

**Phases completed:** 2 phases, 5 plans, 1 tasks

**Key accomplishments:**
- (none recorded)

---

## v2.0 Local Proxy (Shipped: 2026-03-14)

**Phases completed:** 4 phases, 7 plans, 0 tasks

**Key accomplishments:**
- axum 0.8 HTTP 代理引擎 — 全路径请求转发、SSE 流式透传、凭据动态替换、健康自检
- 直连/代理双模式切换 — 全局总开关 + 每 CLI 独立开关，状态持久化到本地设备层
- 退出清理与崩溃恢复 — 正常退出还原 CLI 配置，异常崩溃重启自动检测 takeover 并还原
- iCloud 同步与 Provider CRUD 代理联动 — 内容变更自动更新代理上游内存
- 前端代理模式 UI — 设置页全局开关、Tab 内 CLI 独立开关、绿色状态点、端口冲突 toast
- 托盘菜单与 Provider 编辑路径代理感知修复（Phase 11 gap closure）

**Stats:**
- 54 commits, 56 files changed, +9,484/-205 lines
- Timeline: 2 days (2026-03-13 → 2026-03-14)
- Codebase: ~12,000 LOC (Rust + TypeScript/React)
- Key module: 3,451 LOC proxy engine + commands
- Git range: feat(08-01) to style: rustfmt proxy
- Audit: 18/18 requirements, 18/18 integration, 8/8 E2E flows
- 221 tests, all green

---

## v1.1 System Tray (Shipped: 2026-03-13)

**Phases completed:** 2 phases, 3 plans, 7 tasks

**Key accomplishments:**
- macOS 系统托盘常驻，关闭窗口自动隐藏并切换 Accessory 模式（不显示 Dock/Cmd+Tab）
- 动态托盘菜单按 CLI 分组显示 Provider 列表，CheckMenuItem 原生勾选标记
- 一键切换 Provider — 点击托盘菜单项即可切换，无需打开主窗口
- 全方位自动刷新 — iCloud 同步、前端 CRUD、导入、语言切换后托盘菜单自动更新
- 托盘菜单 i18n — TrayTexts 结构体支持中英文，品牌名保持不变
- 133 个测试（含 8 个新增托盘测试），零回归

**Stats:**
- 16 files changed, +537/-25 lines (code only)
- Timeline: 1 day (2026-03-12 evening to 2026-03-13 afternoon)
- Codebase: 8,441 LOC (5,411 Rust + 3,030 TypeScript/React)
- Git range: feat(06-01) to fix(07-02)
- Audit: 9/9 requirements, 11/11 integrations, 6/6 E2E flows

---

## v1.0 MVP (Shipped: 2026-03-12)

**Phases completed:** 5 phases, 12 plans, 0 tasks

**Key accomplishments:**
- Two-layer storage architecture (iCloud + local) with per-provider JSON files for conflict-free sync
- Surgical patch engine with CliAdapter trait — JSON and TOML patching that preserves comments and unknown keys
- Full provider management UI with CLI-tabbed interface, CRUD dialogs, and one-click switching
- FSEvents-based file watcher with self-write tracking for live iCloud sync and auto re-patching
- First-launch onboarding with automatic CLI config detection and selective import
- Chinese/English i18n from day one with runtime language switching

**Stats:**
- 85 commits, 143 files, 7,986 LOC (Rust + TypeScript/React)
- Timeline: 3 days (2026-03-10 to 2026-03-12)
- Execution time: ~1.12 hours total (avg 6min/plan)
- Git range: feat(01-01) to feat(05-02)

---

