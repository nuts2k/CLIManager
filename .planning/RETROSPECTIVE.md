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

## Milestone: v2.2 — 协议转换

**Shipped:** 2026-03-15
**Phases:** 3 | **Plans:** 10 | **Commits:** 44

### What Was Built
- Anthropic -> OpenAI Chat Completions 双向协议转换（请求/响应/流式 SSE），纯函数 TDD
- 流式 SSE Deferred Start 工具调用缓冲 + 多工具并发追踪 + 跨 chunk 截断处理
- handler.rs 三分支协议路由 + 模型映射三级优先级（精确匹配 > 默认模型 > 保留原名）
- Responses API 完整转换层（请求/非流式响应/SSE 流式）
- Provider 编辑 UI 三协议选择 + 默认模型 + 模型映射对可视化配置
- ProtocolType 三变体 serde alias 向前兼容

### What Worked
- 纯函数先行策略：translate/ 子模块独立实现和测试，与 handler 解耦，Wave 2 三路并行零冲突
- serde_json::Value 动态映射：无需 typed struct，灵活处理 Anthropic/OpenAI 两种截然不同的 JSON 格式
- TDD 驱动：86 个转换测试（63 Chat Completions + 23 Responses API）作为回归保护
- 3 phases 最大并行度设计：Phase 14 内部 Wave 2 三路并行，Phase 16 内部三路并行
- cc-switch 参考代码有效缩短调研时间（streaming.rs Deferred Start 逻辑直接参考）

### What Was Inefficient
- ROADMAP.md 的 plan 复选框未在执行时更新（显示 `[ ]` 但实际已完成）— 连续三个里程碑的同一问题
- STATE.md 的 Current Position 未随执行更新（仍显示 Phase 14 ready to plan）
- 多个 SUMMARY 中 stream.rs 编译阻塞修复重复出现（Plans 02、03、04 各自独立添加占位 stub）— Wave 2 并行的必然代价

### Patterns Established
- `translate/` 六子模块结构：request/response/stream（Chat Completions）+ responses_request/responses_response/responses_stream（Responses API）
- handler.rs 步骤 C → match protocol_type → 步骤 J 三分支响应处理模式
- apply_upstream_model_mapping 纯函数在转换前执行（handler 层关注点分离）
- Deferred Start pending buffer：Chat Completions 工具流式分帧的标准处理模式
- 无 Deferred Start 的 Responses API 流式：output_item.added 携带完整信息，立即发 content_block_start

### Key Lessons
1. 纯函数转换 + TDD 是协议转换层的最佳实践 — 86 个测试让后续 hotfix（4 个 fix commits）修改有信心
2. serde_json::Value 比 typed struct 更适合协议转换 — 两种 API 格式差异过大，typed struct 会引入过多中间类型
3. 并行 Wave 计划中的跨模块依赖应提前考虑 — stream.rs 占位 stub 问题在三个 Plan 中重复出现
4. cc-switch 参考代码最大价值在流式 SSE 处理 — Deferred Start 逻辑直接可借鉴
5. ProtocolType serde alias 向前兼容是正确做法 — 新增变体时零迁移成本

### Cost Observations
- Model mix: balanced profile (sonnet-based agents, opus orchestration)
- Total execution: ~57 min for 10 plans (avg 6min/plan)
- Notable: 2-day milestone — 协议转换复杂度高但纯函数分治有效降低集成风险

---

## Milestone: v2.3 — 前端调整及美化

**Shipped:** 2026-03-15
**Phases:** 6 | **Plans:** 9 | **Commits:** ~27

### What Was Built
- CSS 变量配色体系 — 品牌橙色 oklch token + 语义化颜色命名 + 间距阶梯 + 圆角规范统一
- Provider 卡片操作外露 — 编辑/测试/删除图标按钮始终可见 + hover 升起效果 + 空状态精致化
- Provider 编辑对话框重构 — 640px 加宽可滚动 + 三分区平铺表单 + aria-invalid 红色边框验证
- 设置页 Tab 布局 — 通用/高级/关于三 Tab + line 下划线风格 + 关于页品牌 Logo
- 微动效与 Header 品牌视觉 — 页面切换淡入淡出 150ms + Header 品牌标识 --header-bg 层次分隔
- 全新应用图标 — SVG 生成全套 icns/ico/png + 黑白轮廓 template 托盘图标

### What Worked
- Phase 17 设计 token 先行策略正确：后续 5 个 Phase 全部复用 CSS 变量，无重复定义
- 直接利用 shadcn/ui 内置的 aria-invalid 样式：不需要额外的条件 className 逻辑
- 三分区平铺替代 Collapsible：消除了验证错误被折叠隐藏的 UX 问题
- AppShell 双视图 opacity 过渡：比条件渲染更平滑，状态不丢失
- Python Pillow 手工绘制托盘图标：绕过 qlmanage 渲染透明 SVG 为白底的问题

### What Was Inefficient
- SUMMARY.md 的 one_liner 和 requirements_completed 字段在 3 个 phase 中为空 — 执行器未始终填充 SUMMARY frontmatter
- ROADMAP.md 的 plan 复选框仍未在执行时更新（连续四个里程碑的同一问题）
- STATE.md 的 Current Position 未随执行更新（仍显示 Phase 17 ready to plan）

### Patterns Established
- oklch 色彩空间 CSS 变量 + @theme inline 注册：Tailwind v4 与 shadcn 变量体系统一入口
- 语义化颜色命名：status-success/warning/active 替代具体色相名，换色只改 :root
- TooltipProvider 包裹一次模式：按钮组外层包裹避免重复嵌套
- variant="line" Tab 下划线风格：居左对齐，视觉轻量
- --header-bg 色值介于 background 与 card 之间：微深即可，不需强对比度

### Key Lessons
1. 设计 token 先行是前端美化类里程碑的正确起手 — 后续所有 Phase 直接复用变量，零重复工作
2. Input 组件内置 aria-invalid 样式应优先利用 — 减少自定义验证 UI 逻辑
3. Collapsible 表单对验证反馈有害 — 隐藏字段验证失败用户看不到
4. qlmanage 不适合渲染需要透明背景的 SVG — 需要 Pillow 等工具手工绘制
5. ROADMAP plan 复选框更新问题持续四个里程碑 — 应考虑执行器自动化

### Cost Observations
- Model mix: balanced profile (sonnet-based agents, opus orchestration)
- Total execution: ~1 day for 9 plans
- Notable: 6 phases 单日完成 — 纯前端 CSS/UI 工作执行速度快

---

## Milestone: v2.4 — Anthropic 模型映射

**Shipped:** 2026-03-15
**Phases:** 1 | **Plans:** 2 | **Commits:** ~8

### What Was Built
- Anthropic /v1/messages 透传分支新增三级模型映射，复用现有 apply_upstream_model_mapping（精确匹配 > 默认模型 > 保留原名）
- 非流式响应 model 字段反向映射（reverse_model_in_response），客户端始终看到原始 Claude 模型名
- 流式 SSE 逐行反向映射（reverse_model_in_sse_line），同时处理顶层 model 和 message.model 嵌套字段（message_start 事件格式）
- 无映射配置时走 Passthrough 零开销，AnthropicPassthrough 变体仅在有映射时激活
- ProviderDialog 对所有协议统一显示映射区域，Anthropic 协议字段均可选

### What Worked
- 2 plans 并行设计（后端 + 前端无依赖）最大化执行效率，单日完成
- AnthropicPassthrough 响应模式变体携带 request_model 的设计干净：请求层记录、响应层替换，职责分离清晰
- showModelMapping = true 常量化（而非枚举协议类型）符合开放封闭原则，后续新增协议无需修改
- TDD 驱动：RED 测试先行立即发现 message.model 嵌套字段问题，GREEN 阶段修复代价小

### What Was Inefficient
- SUMMARY.md one_liner 字段为空（gsd-tools summary-extract 无法提取）— 执行器 frontmatter 填充不一致问题延续
- ROADMAP.md plan 复选框（Plans: - [ ]）在完成后仍为未勾选状态 — 第五个里程碑出现同一问题
- Nyquist VALIDATION.md 未创建 — Phase 23 缺失验证文件

### Patterns Established
- `AnthropicPassthrough { request_model }` 响应模式变体：携带请求上下文到响应处理分支的标准模式
- `reverse_model_in_sse_line` 双字段处理：同时检查顶层 model 和 message.model，适用于 Anthropic SSE 事件格式多样性
- `isOpenAiProtocol` 门控校验：UI 统一显示，校验因协议类型差异化 — 比 showModelMapping 条件更细粒度

### Key Lessons
1. 反向映射需要在请求阶段记录原始 model 名 — AnthropicPassthrough 变体是比全局变量更干净的携带方案
2. SSE 事件格式因事件类型而异（message_start vs content_block_delta）— 处理 SSE 反向映射需分析所有可能有 model 字段的事件类型
3. 无映射配置时走 Passthrough 而非 AnthropicPassthrough 是正确做法 — 避免对请求体不必要的 JSON 解析/序列化
4. TDD 在发现隐含格式假设方面有效 — SSE message.model 嵌套字段问题在 GREEN 阶段第一次测试就暴露并修复

### Cost Observations
- Model mix: balanced profile (sonnet-based agents, opus orchestration)
- Total execution: ~15 min for 2 plans (avg 7.5min/plan)
- Notable: 1-phase 单日里程碑 — 精确需求范围 + 复用现有基础设施使执行极高效

---

## Milestone: v2.5 — Claude 全局配置 Overlay

**Shipped:** 2026-03-17
**Phases:** 2 | **Plans:** 5 | **Commits:** 29

### What Was Built
- overlay 存储层（iCloud 优先/本地降级，StorageLocation enum + OverlayStorageInfo + UI 感知同步位置）
- Settings → Advanced → Claude 小节完整 UI（JSON 编辑/校验/保存/位置显示/保护字段提示，中英双语）
- json_merge 深度合并引擎（merge_with_null_delete + strip_protected_fields 纯函数模块）
- ClaudeAdapter patch 强制集成 overlay 存储读取 + 深度合并 + 末尾保护字段回写
- 三类自动应用触发：保存即 apply + 启动 best-effort apply + iCloud watcher 自动 apply
- startup 缓存回放机制（ClaudeOverlayStartupNotificationQueue take 语义）解决 setup 时序问题
- 自动化测试全覆盖：45 个 overlay 相关测试，全量 412 tests passing

### What Worked
- 单 Phase 端到端（Phase 24）4 个 Plan 递进式依赖链清晰：存储 → UI → 合并引擎 → 触发点
- overlay_path_override 注入模式：生产代码走全局存储，测试注入 TempDir 路径，零生产代码改动
- patch_claude_json 末尾强制回写保护字段：无论 overlay 如何设置，Provider/Proxy 凭据优先级无条件保障
- startup 缓存队列（take 语义）彻底解决 Tauri setup 阶段 emit 事件 vs 前端就绪的时序竞态
- Phase 25 测试补充直接 GREEN：现有实现已正确处理所有边界场景，未发现回归

### What Was Inefficient
- VERIFICATION.md 未更新：Phase 24 字段名 bug 已修复，但 VERIFICATION 仍记录 gaps_found 状态
- Nyquist VALIDATION.md 两个 Phase 均未创建
- 审计发现的 3 项技术债务（人工验证待执行）未在里程碑内闭环

### Patterns Established
- `overlay_path_override: Option<PathBuf>` 注入模式：adapter 层可选路径覆盖，测试与生产路径分离
- `ClaudeOverlayStartupNotificationQueue`：Tauri State + Mutex<Vec<T>> + take 语义 — setup 时序问题的标准解法
- `apply_claude_settings_overlay(source: ApplySource)` 枚举区分 Save/Startup/Watcher 三类来源
- `ClaudeOverlayApplyNotification` 统一通知模型：kind/source/settings_path/error/paths，前端 useSyncListener 统一处理
- `merge_with_null_delete` + `strip_protected_fields` 纯函数组合：合并与安全性正交分离

### Key Lessons
1. 保护字段应在最终写入前强制回写而非在合并阶段过滤 — 末尾回写保证任何代码路径都不会遗漏
2. startup 阶段 emit 事件不可靠 — 必须用缓存队列 + take 语义保证前端挂载后可回放
3. 测试注入模式（path_override）优于 mock 全局存储 — 代码侵入性最小且测试真实
4. 端到端 Phase（单 Phase 多 Plan）适合功能边界清晰的特性 — 避免跨 Phase 集成风险
5. 空 overlay 应跳过校验直接保存 — 用户清空 overlay 是合法操作

### Cost Observations
- Model mix: balanced profile (sonnet-based agents, opus orchestration)
- Total execution: ~2 days for 5 plans
- Notable: 16 个需求 2 phases 交付，纯函数测试直接 GREEN 说明实现质量高

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 85 | 5 | Initial project — established patterns and conventions |
| v1.1 | ~10 | 2 | Tray feature — clean extension of existing architecture |
| v2.0 | 54 | 4 | Local proxy — new subsystem (axum) + audit-driven gap closure |
| v2.1 | ~15 | 2 | Release engineering — CI/CD + auto updater |
| v2.2 | 44 | 3 | Protocol translation — pure function TDD + max parallelism |
| v2.3 | ~27 | 6 | Frontend polish — design tokens first + UI/UX refinement |
| v2.4 | ~8 | 1 | Anthropic model mapping — reuse existing infra + parallel plans |
| v2.5 | 29 | 2 | Claude overlay — E2E feature + test coverage, startup cache queue |

### Cumulative Quality

| Milestone | LOC | Files | Avg Plan Duration |
|-----------|-----|-------|-------------------|
| v1.0 | 7,986 | 143 | 6min |
| v1.1 | 8,441 | 16 modified | 8min |
| v2.0 | ~12,000 | 56 modified | 5min |
| v2.1 | ~12,000 | — | 8min |
| v2.2 | ~18,000 | 29 modified | 6min |
| v2.3 | ~19,000 | 82 modified | — |
| v2.4 | ~19,600 | 2 modified | 7.5min |
| v2.5 | ~24,000 | 34 modified | — |

### Top Lessons (Verified Across Milestones)

1. Design for iCloud from the start — per-file JSON + two-layer storage eliminates sync conflicts
2. Surgical patching via structured data merge preserves what whole-file writes destroy
3. All state change sources must trigger tray refresh — missing one is a bug (verified v1.1)
4. All CLI config modification paths must check proxy state — missing one breaks proxy mode (verified v2.0)
5. Audit-driven gap closure is effective — independent phases for precise fixes (verified v2.0)
6. 纯函数 + TDD 是协议转换层最佳实践 — 86 个测试让 hotfix 修改有信心 (verified v2.2)
7. serde alias 向前兼容是 enum 变体重命名的正确做法 — 零迁移成本 (verified v2.2)
8. 设计 token 先行是前端美化类里程碑的正确起手 — 后续全部复用变量 (verified v2.3)
9. ROADMAP plan 复选框更新是持续性问题 — 执行器应自动化 (observed v1.0-v2.4)
10. 反向映射需在请求阶段记录原始值 — 响应模式变体携带上下文比全局状态更干净 (verified v2.4)
11. startup 阶段事件发送不可靠 — 缓存队列 + take 语义是 Tauri setup vs 前端就绪时序问题的标准解法 (verified v2.5)
12. 测试注入模式（path_override）优于 mock 全局存储 — 代码侵入性最小且测试真实路径 (verified v2.5)
