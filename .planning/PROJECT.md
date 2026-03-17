# CLIManager

## What This Is

CLIManager 是一个基于 Tauri 2 的桌面应用，用于统一管理多种 AI CLI（Claude Code、Codex 等）的 Provider 配置。支持两种工作模式：直连模式（surgical patch CLI 配置文件）和代理模式（本地 HTTP 代理实时转发，切换 Provider 无需重启 CLI）。代理模式支持自动协议转换，Claude Code 可通过代理使用 OpenAI 兼容的 Provider（Chat Completions API 和 Responses API 双格式）以及 Anthropic 协议 Provider（含完整模型映射支持）。支持 Claude Code 全局配置 overlay：用户可编辑 JSON overlay 自动深度合并到 `~/.claude/settings.json`，保护字段不被覆盖，iCloud 同步优先存储，三类触发点自动对齐。前端采用暗色主题，品牌橙色设计语言，精致的交互动效和专属应用图标。

目标用户：使用多个 AI CLI 工具并需要在不同 Provider（API 提供商/模型配置）间频繁切换的开发者。

## Core Value

**切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容。**

## Requirements

### Validated

- ✓ Provider CRUD（创建、读取、编辑、删除） — v1.0
- ✓ Provider 切换 — surgical patch 到 CLI 配置文件（只改凭据+模型字段） — v1.0
- ✓ 支持 Claude Code CLI 配置适配（~/.claude/settings.json） — v1.0
- ✓ 支持 Codex CLI 配置适配（~/.codex/auth.json + config.toml） — v1.0
- ✓ iCloud 同步层 — 每个 Provider 一个独立 JSON 文件存放在 iCloud Drive 目录 — v1.0
- ✓ 设备本地层 — 当前激活 Provider、设备路径覆盖等存放在本地（不同步） — v1.0
- ✓ 文件监听 — 监听 iCloud 同步目录变化，自动刷新 UI — v1.0
- ✓ 活跃 Provider 联动 — 同步目录中活跃 Provider 变化时，自动重新 patch CLI 配置 — v1.0
- ✓ 首次启动自动导入 — 扫描现有 CLI 配置创建初始 Provider — v1.0
- ✓ 手动创建 Provider — v1.0
- ✓ i18n 国际化 — 中英双语，默认中文，可扩展 — v1.0
- ✓ 系统托盘图标常驻，主窗口关闭后应用驻留在托盘 — v1.1
- ✓ 托盘菜单按 CLI 分组显示 Provider 列表，一键切换无需打开主窗口 — v1.1
- ✓ 托盘菜单文字跟随应用语言设置（中/英） — v1.1
- ✓ Provider 增删改或 iCloud 同步变化后，托盘菜单自动刷新 — v1.1
- ✓ 本地 HTTP 代理服务，按 CLI 固定端口监听（Claude Code:15800, Codex:15801） — v2.0
- ✓ 双模式切换：直连 vs 代理（全局总开关 + 每 CLI 独立开关），状态持久化 — v2.0
- ✓ 代理模式下切换 Provider 实时生效，无需重启 CLI — v2.0
- ✓ iCloud 同步和 Provider CRUD 自动更新代理上游内存 — v2.0
- ✓ 退出清理与崩溃恢复 — 正常退出还原配置，崩溃后重启自动检测并还原 — v2.0
- ✓ 端口冲突检测与友好错误提示 — v2.0
- ✓ 托盘菜单和 Provider 编辑路径代理感知 — v2.0
- ✓ 代理启动健康自检（GET /health） — v2.0
- ✓ GitHub Actions CI/CD — tag 推送自动触发双架构（aarch64 + x86_64）构建 — v2.1
- ✓ Ad-hoc 代码签名 + Ed25519 更新签名 — v2.1
- ✓ Tauri 自动更新 — tauri-plugin-updater + 自定义 React 更新 UI（进度条 + 稍后提醒） — v2.1
- ✓ 一键发版 /ship 技能 — bump -> CHANGELOG -> commit -> tag -> push — v2.1
- ✓ GitHub Release Notes 含 Gatekeeper 安装指引 — v2.1
- ✓ Anthropic -> OpenAI Chat Completions 双向协议转换（请求/响应/流式 SSE） — v2.2
- ✓ OpenAI Responses API 格式转换（请求/非流式/流式 SSE） — v2.2
- ✓ handler.rs 三分支协议路由（OpenAiChatCompletions / OpenAiResponses / Anthropic） — v2.2
- ✓ 模型映射三级优先级（精确匹配 > 默认模型 > 保留原名） — v2.2
- ✓ Provider 编辑 UI 三协议选择 + 默认模型 + 模型映射对配置 — v2.2
- ✓ ProtocolType 三变体 serde alias 向前兼容 — v2.2
- ✓ CSS 变量全局配色（品牌橙色 oklch + 语义化命名），间距/圆角规范统一 — v2.3
- ✓ Provider 卡片操作按钮外露（图标按钮始终可见） + hover 升起效果 — v2.3
- ✓ 空状态精致设计 + 代理状态绿点脉冲指示 — v2.3
- ✓ Provider 编辑对话框加宽（640px）可滚动，三分区平铺表单，验证红色边框 — v2.3
- ✓ 设置页 Tab 布局（通用/高级/关于），line 下划线风格 — v2.3
- ✓ 微动效过渡（150-300ms） + Header 品牌视觉提升 — v2.3
- ✓ 全新应用图标（全套 icns/ico/png）+ 托盘 template 图标 — v2.3
- ✓ Anthropic 协议透传请求在转发前执行模型映射（三级优先级：精确匹配 > 默认模型 > 保留原名） — v2.4
- ✓ Anthropic 透传响应/流式 SSE 中 model 字段反向映射回原始 Claude 模型名 — v2.4
- ✓ Anthropic 协议 Provider 编辑 UI 显示模型映射配置（默认模型和映射对均可选） — v2.4
- ✓ Claude settings.json overlay 编辑/校验/保存（iCloud 同步优先，本地降级，UI 感知位置） — v2.5
- ✓ Overlay 深度合并（递归合并/数组替换/标量覆盖/null 删除）+ 保护字段永远优先 — v2.5
- ✓ 自动应用：保存即 apply + 启动 best-effort apply + iCloud watcher 变更自动 apply — v2.5
- ✓ Overlay 自动化测试覆盖（深度合并边界/保护字段优先级/ClaudeAdapter overlay 注入） — v2.5

### Active

- [ ] 代理请求实时日志记录（基础信息 + token 用量 + 错误详情 + 摘要字段）
- [ ] SQLite 持久化存储（滚动 24 小时明细 + 7 天统计）
- [ ] 独立流量监控页面（与 Providers、Settings 并列的顶级页面）
- [ ] 实时日志表格展示 + Provider 筛选
- [ ] 统计数据展示（按时间/Provider 聚合，表格为主辅以图表）

## Current Milestone: v2.6 流量监控

**Goal:** 为代理模式增加请求日志记录、实时流量展示和统计分析能力

**Target features:**
- 代理请求实时日志（基础信息 + token 用量 + 错误详情 + 少量摘要字段）
- SQLite 持久化（滚动保留 24 小时明细 + 7 天统计数据）
- 独立流量监控页面（实时日志表格 + Provider 筛选）
- 统计数据展示（按时间/Provider 聚合，表格为主辅以图表）

## Shipped: v2.5 Claude 全局配置 Overlay (2026-03-17)

**Delivered:** Claude Code settings.json overlay 端到端可用：编辑/校验/保存（iCloud 同步优先，本地降级）+ 深度合并引擎（null 删除/保护字段优先）+ 三类自动 apply 触发（保存/启动/watcher）+ 全覆盖自动化测试（412 tests passing）

### Out of Scope

- MCP 服务器管理 — v1 只做 Providers，MCP 后续里程碑
- Prompts / Skills 管理 — 同上
- 系统托盘内 Provider 增删改 — 托盘只做查看和切换，管理操作留在主窗口
- 反向协议转换（OpenAI->Anthropic，Codex 用 Anthropic Provider）— v2.2 只做 Anthropic->OpenAI 方向
- OAuth 桥接（如 OpenAI OAuth 转 Anthropic 协议）— 2.x 全功能网关里程碑
- 流量监控高级功能（实时告警、费用估算、导出报表）— v2.6 只做基础监控
- Proxy Failover / Usage 统计 — 2.x 全功能网关里程碑
- WebDAV / 自定义同步 — iCloud Drive 原生同步足够
- Session Manager — 与 Provider 切换主线弱耦合
- Deep Link 导入 — 非 MVP 必需
- Gemini / OpenCode / OpenClaw 支持 — v1 只做 Claude Code + Codex
- 亮色/暗色主题切换 — v2.3 聚焦暗色精调，双主题后续考虑

## Context

### 参考项目

cc-switch（https://github.com/farion1231/cc-switch）是一个功能完整但臃肿的 AI CLI 配置管理器。参考代码在 `cc-switch/` 目录（只读），详细拆解笔记见 `cc-switch-ref-notes-zh.md`。

### cc-switch 的核心问题

1. **配置损坏**：切换 Provider 时整体重写配置文件，破坏了用户在 CLI 中设置的其他选项
2. **iCloud 冲突**：SQLite + settings.json + live config 多点写入暴露在 iCloud 最终一致语义下，导致同步延迟、半套配置、状态回跳（详见 `icloud-sync-root-cause-zh.md`）
3. **写入面过大**：一次切换触发 provider + MCP + skills 全家桶写入，iCloud 冲突窗口大

### 设计决策

**Surgical Patch 策略**：Read-Modify-Write — 读取当前配置文件，只修改目标字段（API key、model、base URL 等），写回文件。接受极小的并发竞态风险（CLI 同时写入同一文件的概率很低）。

**iCloud 安全存储架构**：
- 可同步层（iCloud Drive）：每个 Provider 一个独立 JSON 文件，单文件更新避免跨文件事务问题
- 设备本地层（~/.cli-manager/）：设备级设置不同步，避免多设备互相覆盖

**Provider 按协议建模**：Provider 绑定 API 协议类型（Anthropic、OpenAI Chat Completions、OpenAI Responses），而非绑定特定 CLI。每个 CLI 通过适配器将对应协议的凭据写入自己的配置格式。这样未来加新 CLI（如 OpenCode 支持多协议）只需加适配器，Provider 可复用。

**文件监听 + 联动**：监听 iCloud 同步目录（FSEvents），当另一台设备同步了 Provider 变更：
1. 刷新 UI 展示最新数据
2. 如果变更的是当前活跃 Provider，自动重新 patch CLI 配置
3. 代理模式下自动更新上游内存参数

**协议转换架构**：纯函数转换模块（translate/）独立于 handler，可单独单元测试。handler.rs 按 ProtocolType 三分支路由：Anthropic 直接透传、OpenAiChatCompletions 走 Chat Completions 转换、OpenAiResponses 走 Responses API 转换。模型映射在转换前执行（handler 层）。

**设计 Token 体系（v2.3）**：品牌橙色 oklch(0.702 0.183 56.518) 通过 --brand-accent CSS 变量全局引用，语义化命名（status-success/warning/active），间距阶梯 CSS 变量作文档锚点，业务组件用 Tailwind 工具类。

## Constraints

- **Tech Stack**: Tauri 2（Rust 后端 + React 前端）— 沿用 cc-switch 技术选型，桌面原生性能
- **Platform**: macOS 优先（iCloud Drive 依赖）
- **CLI 配置格式**: Claude Code 用 JSON（settings.json），Codex 用 JSON + TOML（auth.json + config.toml），适配器需分别处理
- **安全**: API key 明文存储（与 CLI 原生配置文件一致），iCloud 已提供传输和静态加密
- **cc-switch 参考代码只读**: `cc-switch/` 目录不可修改，仅作参考

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Surgical patch 而非整文件重写 | cc-switch 的 atomic_write 整文件替换导致配置损坏，丢失用户其他设置 | ✓ Good |
| 每 Provider 一个 JSON 文件 | iCloud 对单文件更新处理最好，避免 SQLite 放云盘的经典雷区 | ✓ Good |
| 数据分层（同步层 + 本地层） | 设备级设置不应跨设备同步，避免互相覆盖 | ✓ Good |
| Provider 按协议类型建模 | 未来 CLI 可能支持多协议，按协议复用 Provider 比按 CLI 绑定更灵活 | ✓ Good |
| Read-Modify-Write 不加文件锁 | CLI 与 CLIManager 同时写同一配置的概率极低，文件锁增加复杂度收益不大 | ✓ Good |
| i18n 从 v1 开始 | 避免后期改造成本，中英双语默认中文 | ✓ Good |
| axum 0.8 作为代理框架 | 复用 Tauri 内置 tokio runtime，无需额外异步运行时 | ✓ Good |
| 每 CLI 独立固定端口 | Claude Code 15800, Codex 15801，端口确定性便于调试 | ✓ Good |
| 纯函数转换模块独立于 handler | translate/ 子模块可单独 TDD，不依赖 handler 上下文 | ✓ Good |
| ProtocolType 三变体 + serde alias | OpenAiChatCompletions 替代旧 OpenAiCompatible，alias 保持旧 JSON 兼容 | ✓ Good |
| Cargo.toml 唯一版本来源 | tauri.conf.json 省略 version 字段，Tauri 自动回退到 Cargo.toml | ✓ Good |
| CSS 变量品牌色 + oklch | oklch 色彩空间与 shadcn 体系一致，语义化命名未来换色只需改 :root | ✓ Good |
| ProviderCard 操作外露为图标按钮 | 「复制到」因子菜单保留在三点菜单，其余操作始终可见 | ✓ Good |
| 三分区平铺表单（移除 Collapsible） | 避免高级字段折叠隐藏导致验证错误不可见 | ✓ Good |
| AppShell 双视图 opacity 过渡 | 始终渲染两视图用 opacity+pointer-events 切换，避免卸载导致状态丢失 | ✓ Good |
| 托盘图标 Python Pillow 手工绘制 | qlmanage 渲染透明 SVG 为白底不符合 template 要求 | ✓ Good |
| AnthropicPassthrough 响应模式变体 | 独立变体携带 request_model，响应分支时替换回原始名；无映射走 Passthrough 零开销 | ✓ Good |
| showModelMapping = true 常量 | 所有协议统一显示映射区域，新增协议无需修改条件；校验差异放到 isOpenAiProtocol 层 | ✓ Good |
| overlay 存储与 providers 目录分离 | config 目录独立于 providers 目录，避免 overlay 文件被 provider CRUD 影响 | ✓ Good |
| overlay_path_override 注入模式 | adapter 新增可选路径字段，测试时注入 TempDir 路径，生产时走全局存储 | ✓ Good |
| patch_claude_json 末尾强制回写保护字段 | 保证 Provider/Proxy 凭据优先级无法被 overlay 绕过，安全性无条件保障 | ✓ Good |
| startup 通知缓存队列（take 语义） | 解决 setup 阶段 emit 事件前端未就绪的时序问题，前端挂载后 take/replay | ✓ Good |

## Context

**Current State (v2.5 shipped 2026-03-17):**
- ~24,000 LOC (Rust + TypeScript/React)
- Tech stack: Tauri 2.10, React 19, Vite 7, shadcn/ui, Tailwind CSS v4, i18next, axum 0.8
- Rust crates: serde, serde_json (preserve_order), toml_edit, notify (FSEvents), uuid, chrono, reqwest (+stream), axum, tower-http, tokio (net,sync,time), tauri-plugin-updater, tauri-plugin-process, bytes, async-stream
- 25 phases, 53 plans across 8 milestones
- CSS 变量设计体系（品牌色 + 语义色 + 间距阶梯 + 圆角规范）
- Local HTTP proxy with SSE streaming, dual-mode switching, crash recovery, proxy-aware tray and provider editing
- Protocol translation: Anthropic <-> OpenAI Chat Completions + Responses API, with model mapping (both directions: request substitution + response reverse mapping)
- Anthropic passthrough: full model mapping support (request + non-streaming response + SSE streaming), transparent passthrough when no mapping configured
- translate/ 模块: 6 子模块 4,141 LOC（request, response, stream, responses_request, responses_response, responses_stream）
- Claude settings.json overlay: 深度合并引擎 + 保护字段优先 + 三类自动 apply 触发 + iCloud 同步存储
- CI/CD: GitHub Actions 双架构自动构建 + 自动发布 + Ed25519 签名
- Auto updater: 启动自动检查 + 自定义 UI + 进度条
- 412 Rust lib tests, all passing

**Known Issues:**
- UX-01 端口冲突检测依赖脆弱的中文子串匹配（重构 ProxyError 显示字符串可能静默降级）
- Phase 8 的 4 个低级 Tauri 命令(proxy_start/stop/status/update_upstream)前端未调用（Phase 9 高级命令替代）
- Ad-hoc 签名 relaunch() 可能报 os error 1（Issue #2273），已降级为手动重启
- CI DMG 打包随机失败（Bug #13804，AppleScript 超时），重跑 CI 解决

## Next Milestone

候选方向：
- **v2.7+** 流量监控高级功能（告警、费用估算、导出）
- **v3.0 全功能网关**：OAuth 桥接、自动 Failover、熔断器、反向协议转换（OpenAI->Anthropic）

---
*Last updated: 2026-03-17 after v2.6 milestone started*
