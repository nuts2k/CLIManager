# Milestones

## v2.5 Claude 全局配置 Overlay (Shipped: 2026-03-17)

**Phases completed:** 2 phases, 5 plans
**Delivered:** Claude Code settings.json overlay 端到端可用：编辑/校验/保存（iCloud 同步优先，本地降级）+ 深度合并引擎（null 删除/保护字段优先）+ 三类自动 apply 触发（保存/启动/watcher）+ 全覆盖自动化测试

**Key accomplishments:**
- overlay 存储层（iCloud 优先/本地降级，StorageLocation enum + UI 感知同步位置）
- Settings → Advanced → Claude 小节完整 UI（JSON 编辑/校验/保存/位置显示/保护字段提示，中英双语）
- json_merge 深度合并引擎（merge_with_null_delete + strip_protected_fields 纯函数模块）+ ClaudeAdapter overlay 集成
- 三类自动应用触发（保存即 apply + 启动 best-effort apply + iCloud watcher 自动 apply）+ startup 缓存回放 + 统一错误可见性通知
- 自动化测试全覆盖（深度合并边界/保护字段优先级/ClaudeAdapter overlay 注入，412 tests passing）

**Stats:**
- 29 commits, 34 files changed, +4,633/-97 lines
- Timeline: 2 days (2026-03-16 -> 2026-03-17)
- Tests: 412 passing (45 new overlay-specific tests)
- Audit: 16/16 requirements, 16/16 integration, 3/3 E2E flows

---

## v2.4 Anthropic 模型映射 (Shipped: 2026-03-15)

**Phases completed:** 1 phase, 2 plans
**Delivered:** Anthropic 协议透传路径完整支持模型映射，代理层自动替换请求中的 model 字段并反向映射响应/流式 SSE 结果，Provider 编辑 UI 统一显示模型映射配置

**Key accomplishments:**
- Anthropic /v1/messages 透传分支新增三级模型映射（精确匹配 > 默认模型 > 保留原名），复用现有 apply_upstream_model_mapping
- 非流式响应 model 字段反向映射（reverse_model_in_response），客户端始终看到原始 Claude 模型名
- 流式 SSE 逐行反向映射（reverse_model_in_sse_line），同时处理顶层 model 和 message.model 嵌套字段（message_start 事件格式）
- 无映射配置时走 Passthrough 零开销保持原有透传行为
- ProviderDialog 对所有协议统一显示映射区域（showModelMapping = true），Anthropic 字段均可选

**Stats:**
- 2 source files changed: handler.rs (+631 行), ProviderDialog.tsx (+20/-11 行)
- Timeline: 1 day (2026-03-15)
- 11 个 Anthropic 专属集成测试新增，367 个全量测试 0 失败
- Audit: 4/4 requirements, 4/4 E2E flows

---

## v2.3 前端调整及美化 (Shipped: 2026-03-15)

**Phases completed:** 6 phases, 9 plans
**Delivered:** 全面提升前端交互体验、视觉质感和设计一致性，建立 CSS 变量设计体系，重构核心交互页面，更换全套品牌图标

**Key accomplishments:**
- CSS 变量配色体系 — 品牌橙色 oklch token，全局语义化颜色命名，消除所有硬编码色值，间距/圆角规范统一
- Provider 卡片操作外露 — 编辑/测试/删除图标按钮始终可见，hover 升起效果，空状态精致化设计
- Provider 编辑对话框重构 — 640px 加宽可滚动，三分区平铺表单（基础信息/协议设置/模型配置），aria-invalid 红色边框验证
- 设置页 Tab 化 — 通用/高级/关于三 Tab 分组布局，line 下划线风格，关于页品牌 Logo
- 微动效与 Header 品牌视觉 — 页面切换淡入淡出 150ms ease-out，Header 品牌标识提升（--header-bg 层次分隔）
- 全新应用图标 — SVG 生成全套 icns/ico/png 尺寸，黑白轮廓 template 托盘图标（Pillow 手工绘制）

**Stats:**
- ~27 commits, 82 files changed, +4,647/-464 lines
- Timeline: 1 day (2026-03-15)
- Git range: feat(17-01) to docs(v2.3)
- Audit: 14/14 requirements, 14/14 integration, 5/5 E2E flows

---

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

