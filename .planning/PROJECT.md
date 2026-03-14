# CLIManager

## What This Is

CLIManager 是一个基于 Tauri 2 的桌面应用，用于统一管理多种 AI CLI（Claude Code、Codex 等）的 Provider 配置。支持两种工作模式：直连模式（surgical patch CLI 配置文件）和代理模式（本地 HTTP 代理实时转发，切换 Provider 无需重启 CLI）。

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

### Active

(暂无 — 等待下一个里程碑定义)

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- MCP 服务器管理 — v1 只做 Providers，MCP 后续里程碑
- Prompts / Skills 管理 — 同上
- 系统托盘内 Provider 增删改 — 托盘只做查看和切换，管理操作留在主窗口
- 协议转换（Anthropic↔OpenAI）— 2.x 全功能网关里程碑
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

**Provider 按协议建模**：Provider 绑定 API 协议类型（Anthropic、OpenAI 兼容等），而非绑定特定 CLI。每个 CLI 通过适配器将对应协议的凭据写入自己的配置格式。这样未来加新 CLI（如 OpenCode 支持多协议）只需加适配器，Provider 可复用。

**文件监听 + 联动**：监听 iCloud 同步目录（FSEvents），当另一台设备同步了 Provider 变更：
1. 刷新 UI 展示最新数据
2. 如果变更的是当前活跃 Provider，自动重新 patch CLI 配置
3. 架构预留：未来本地代理转发功能可在此基础上实现参数热更新

## Constraints

- **Tech Stack**: Tauri 2（Rust 后端 + React 前端）— 沿用 cc-switch 技术选型，桌面原生性能
- **Platform**: macOS 优先（iCloud Drive 依赖）
- **CLI 配置格式**: Claude Code 用 JSON（settings.json），Codex 用 JSON + TOML（auth.json + config.toml），适配器需分别处理
- **安全**: API key 明文存储（与 CLI 原生配置文件一致），iCloud 已提供传输和静态加密
- **cc-switch 参考代码只读**: `cc-switch/` 目录不可修改，仅作参考

## Key Decisions

<!-- Decisions that constrain future work. Add throughout project lifecycle. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Surgical patch 而非整文件重写 | cc-switch 的 atomic_write 整文件替换导致配置损坏，丢失用户其他设置 | ✓ Good — serde_json::Value merge + toml_edit 完美保留未知字段和注释 |
| 每 Provider 一个 JSON 文件 | iCloud 对单文件更新处理最好，避免 SQLite 放云盘的经典雷区 | ✓ Good — FSEvents 逐文件触发，无冲突 |
| 数据分层（同步层 + 本地层） | 设备级设置（当前激活 Provider）不应跨设备同步，避免互相覆盖 | ✓ Good — local.json 存设备设置，iCloud 存 Provider 数据 |
| Provider 按协议类型建模 | 未来 CLI（如 OpenCode）可能支持多协议，按协议复用 Provider 比按 CLI 绑定更灵活 | ✓ Good — ProtocolType enum 已用于 Claude/Codex 适配 |
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

## Context

**Current State (v2.0 shipped 2026-03-14):**
- ~12,000 LOC (Rust + TypeScript/React)
- Tech stack: Tauri 2.10, React 19, Vite 7, shadcn/ui, Tailwind CSS v4, i18next, axum 0.8
- Rust crates: serde, serde_json (preserve_order), toml_edit, notify (FSEvents), uuid, chrono, reqwest (+stream), axum, tower-http, tokio (net,sync,time)
- 11 phases, 22 plans across 3 milestones
- Local HTTP proxy with SSE streaming, dual-mode switching, crash recovery, proxy-aware tray and provider editing
- 221 Rust lib tests, all passing

**Known Issues:**
- UX-01 端口冲突检测依赖脆弱的中文子串匹配（重构 ProxyError 显示字符串可能静默降级）
- Phase 8 的 4 个低级 Tauri 命令(proxy_start/stop/status/update_upstream)前端未调用（Phase 9 高级命令替代）
- Release build tray behavior may differ from dev build (needs verification)

## Next Milestone

暂未定义。候选方向：
- **v2.1 协议转换**：Anthropic <-> OpenAI 协议格式互转
- **v2.2 流量监控**：代理请求日志、流量统计可视化
- **v3.0 全功能网关**：OAuth 桥接、自动 Failover、熔断器

---
*Last updated: 2026-03-14 after v2.0 milestone*
