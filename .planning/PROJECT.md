# CLIManager

## What This Is

CLIManager 是一个基于 Tauri 2 的桌面应用，用于统一管理多种 AI CLI（Claude Code、Codex 等）的 Provider 配置。支持两种工作模式：直连模式（surgical patch CLI 配置文件）和代理模式（本地 HTTP 代理实时转发，切换 Provider 无需重启 CLI）。代理模式支持自动协议转换，Claude Code 可通过代理使用 OpenAI 兼容的 Provider（Chat Completions API 和 Responses API 双格式）。

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

### Active

## Current Milestone: v2.3 前端调整及美化

**Goal:** 全面提升前端交互体验、视觉质感和设计一致性，解决空间利用率差、操作隐藏、设置页混乱、图标不统一等问题。

**Target features:**
- 首页布局优化 — Provider 卡片操作按钮外露，告别三点菜单隐藏
- Provider 编辑对话框改进 — 加宽、可滚动、字段分组更合理
- 设置页 Tab 化 — 通用/高级/关于三 Tab 分组
- 暗色主题精调 — 配色优化（橙色强调）、间距、圆角、微动效提升质感
- 应用图标全新设计 — 托盘图标从应用图标派生，视觉统一

### Out of Scope

- MCP 服务器管理 — v1 只做 Providers，MCP 后续里程碑
- Prompts / Skills 管理 — 同上
- 系统托盘内 Provider 增删改 — 托盘只做查看和切换，管理操作留在主窗口
- 反向协议转换（OpenAI->Anthropic，Codex 用 Anthropic Provider）— v2.2 只做 Anthropic->OpenAI 方向
- OAuth 桥接（如 OpenAI OAuth 转 Anthropic 协议）— 2.x 全功能网关里程碑
- 流量监控与可视化 — 2.x 全功能网关里程碑
- Proxy Failover / Usage 统计 — 2.x 全功能网关里程碑
- WebDAV / 自定义同步 — iCloud Drive 原生同步足够
- Session Manager — 与 Provider 切换主线弱耦合
- Deep Link 导入 — 非 MVP 必需
- Gemini / OpenCode / OpenClaw 支持 — v1 只做 Claude Code + Codex

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

## Constraints

- **Tech Stack**: Tauri 2（Rust 后端 + React 前端）— 沿用 cc-switch 技术选型，桌面原生性能
- **Platform**: macOS 优先（iCloud Drive 依赖）
- **CLI 配置格式**: Claude Code 用 JSON（settings.json），Codex 用 JSON + TOML（auth.json + config.toml），适配器需分别处理
- **安全**: API key 明文存储（与 CLI 原生配置文件一致），iCloud 已提供传输和静态加密
- **cc-switch 参考代码只读**: `cc-switch/` 目录不可修改，仅作参考

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Surgical patch 而非整文件重写 | cc-switch 的 atomic_write 整文件替换导致配置损坏，丢失用户其他设置 | ✓ Good — serde_json::Value merge + toml_edit 完美保留未知字段和注释 |
| 每 Provider 一个 JSON 文件 | iCloud 对单文件更新处理最好，避免 SQLite 放云盘的经典雷区 | ✓ Good — FSEvents 逐文件触发，无冲突 |
| 数据分层（同步层 + 本地层） | 设备级设置（当前激活 Provider）不应跨设备同步，避免互相覆盖 | ✓ Good — local.json 存设备设置，iCloud 存 Provider 数据 |
| Provider 按协议类型建模 | 未来 CLI（如 OpenCode）可能支持多协议，按协议复用 Provider 比按 CLI 绑定更灵活 | ✓ Good — ProtocolType 三变体已用于 Claude/Codex 适配及协议转换路由 |
| Read-Modify-Write 不加文件锁 | CLI 与 CLIManager 同时写同一配置的概率极低，文件锁增加复杂度收益不大 | ✓ Good — 无并发问题报告 |
| v1 不做托盘 | 聚焦核心功能，托盘快切 v2 再加 | ✓ Good — v1 按时交付，托盘 v1.1 完成 |
| i18n 从 v1 开始 | 避免后期改造成本，中英双语默认中文 | ✓ Good — i18next 运行时切换无缝 |
| _in/_to 内部函数变体 | 测试隔离无需 mock 文件系统路径 | ✓ Good — 全项目一致的测试模式 |
| 500ms FSEvents debounce | iCloud 事件风暴缓冲 | ✓ Good — 配合 SelfWriteTracker 消除无限循环 |
| import_provider 独立命令 | 避免放松 create_provider 全局校验 | ✓ Good — onboarding 松验证不污染正常流程 |
| Programmatic TrayIconBuilder | 不用 tauri.conf.json 的 trayIcon 配置，避免重复图标 bug | ✓ Good — GitHub Issue #10912 验证 |
| Emit providers-changed 复用 | 托盘切换发 providers-changed 事件，复用现有前端监听器 | ✓ Good — 无需新事件类型 |
| Menu-as-Snapshot 重建模式 | 托盘菜单通过 update_tray_menu + set_menu 全量重建 | ✓ Good — 简单可靠，菜单项少无性能问题 |
| TrayTexts::from_language | 轻量 Rust 端 i18n，只需翻译约 5 个菜单字符串 | ✓ Good — 避免引入 i18n 框架 |
| Fire-and-forget refreshTrayMenu | 前端 CRUD 后非阻塞调用 .catch(() => {}) | ✓ Good — 不阻塞 UI 交互 |
| axum 0.8 作为代理框架 | 复用 Tauri 内置 tokio runtime，无需额外异步运行时 | ✓ Good — 零额外依赖，性能优异 |
| 每 CLI 独立固定端口 | Claude Code 15800, Codex 15801，端口确定性便于调试 | ✓ Good — 多 CLI 互不干扰 |
| 绑定 127.0.0.1 | 避免 macOS 防火墙弹窗和安全风险 | ✓ Good — 本地代理无需暴露 |
| takeover 标志持久化崩溃恢复 | proxy_takeover 写入 local.json，重启检测并还原 | ✓ Good — 无数据丢失 |
| PROXY_MANAGED 占位 key | 凭据注入仅在检测到占位值时触发 | ✓ Good — 非代理配置不受影响 |
| ProxyService tokio::sync::Mutex | 启停操作涉及 async，需异步锁 | ✓ Good — 无死锁问题 |
| proxy_enable 失败时回滚 | 不留半成品状态 | ✓ Good — 原子性操作 |
| determine_tray_switch_mode 纯函数 | 与 AppHandle 解耦，便于单元测试 | ✓ Good — 4 个分支测试覆盖 |
| _update_provider_in skip_patch 参数 | 代理模式下跳过 CLI 配置 patch | ✓ Good — 不覆盖 PROXY_MANAGED |
| Cargo.toml 唯一版本来源 | tauri.conf.json 省略 version 字段，Tauri 自动回退到 Cargo.toml | ✓ Good — 版本号单一来源，/ship 只改 Cargo.toml |
| 无密码 Ed25519 密钥 | 规避 tauri-cli Bug #13485（env var 传入密码时 CI 签名失败） | ✓ Good — CI 无密码签名稳定运行 |
| tauri-action@v0 | v1 不存在，v0 为最新稳定版 | ✓ Good — 双架构构建成功 |
| releaseDraft: false | updater endpoint 需要非 Draft Release 才能访问 latest.json | ✓ Good — 自动发布后 updater 立即可用 |
| 动态 import tauri 插件 | 规避开发模式下 tauri 插件未注册的异常 | ✓ Good — dev/prod 均正常 |
| 双 useUpdater 实例 | AppShell 启动检查 + SettingsPage 手动检查独立互不干扰 | ✓ Good — 各自管理状态 |
| 纯函数转换模块独立于 handler | translate/ 子模块可单独 TDD，不依赖 handler 上下文 | ✓ Good — 63+23 个纯函数测试，Wave 2 三路并行无冲突 |
| serde_json::Value 动态映射 | 比 typed struct 兼容未知字段，无需新核心 crate | ✓ Good — 请求/响应格式差异大，动态映射灵活 |
| Deferred Start pending buffer | 工具流式分帧核心机制，id/name 就绪后才发 content_block_start | ✓ Good — 解决 OpenAI 工具调用分帧与 Anthropic 事件语义不匹配问题 |
| 模型映射在 handler 层转换前执行 | request.rs model 字段原样透传，映射必须在转换前完成 | ✓ Good — 关注点分离清晰 |
| ProtocolType 三变体 + serde alias | OpenAiChatCompletions 替代旧 OpenAiCompatible，alias 保持旧 JSON 兼容 | ✓ Good — 零迁移成本 |
| Responses API 无 Deferred Start | output_item.added 携带完整 call_id+name，与 Chat Completions 分帧不同 | ✓ Good — 实现更简单，独立状态机 |

## Context

**Current State (v2.2 shipped 2026-03-15):**
- ~18,000 LOC (Rust + TypeScript/React)
- Tech stack: Tauri 2.10, React 19, Vite 7, shadcn/ui, Tailwind CSS v4, i18next, axum 0.8
- Rust crates: serde, serde_json (preserve_order), toml_edit, notify (FSEvents), uuid, chrono, reqwest (+stream), axum, tower-http, tokio (net,sync,time), tauri-plugin-updater, tauri-plugin-process, bytes, async-stream
- 16 phases, 37 plans across 5 milestones
- Local HTTP proxy with SSE streaming, dual-mode switching, crash recovery, proxy-aware tray and provider editing
- Protocol translation: Anthropic <-> OpenAI Chat Completions + Responses API, with model mapping
- translate/ 模块: 6 子模块 4,141 LOC（request, response, stream, responses_request, responses_response, responses_stream）
- CI/CD: GitHub Actions 双架构自动构建 + 自动发布 + Ed25519 签名
- Auto updater: 启动自动检查 + 自定义 UI + 进度条
- 329 Rust lib tests, all passing

**Known Issues:**
- UX-01 端口冲突检测依赖脆弱的中文子串匹配（重构 ProxyError 显示字符串可能静默降级）
- Phase 8 的 4 个低级 Tauri 命令(proxy_start/stop/status/update_upstream)前端未调用（Phase 9 高级命令替代）
- Ad-hoc 签名 relaunch() 可能报 os error 1（Issue #2273），已降级为手动重启
- CI DMG 打包随机失败（Bug #13804，AppleScript 超时），重跑 CI 解决

## Next Milestone

候选方向：
- **v2.4 流量监控**：代理请求日志、流量统计可视化
- **v3.0 全功能网关**：OAuth 桥接、自动 Failover、熔断器、反向协议转换（OpenAI->Anthropic）

---
*Last updated: 2026-03-15 after v2.3 前端调整及美化 milestone started*
