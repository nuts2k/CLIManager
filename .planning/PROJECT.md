# CLIManager

## What This Is

CLIManager 是一个基于 Tauri 2 的桌面应用，用于统一管理多种 AI CLI（Claude Code、Codex 等）的 Provider 配置。它是 cc-switch 的精简重构版，解决原版的三个核心痛点：配置文件被整体重写导致损坏、iCloud 同步冲突、以及功能臃肿难以维护。

目标用户：使用多个 AI CLI 工具并需要在不同 Provider（API 提供商/模型配置）间频繁切换的开发者。

## Core Value

**切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容。**

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

(None yet — ship to validate)

### Active

<!-- Current scope. Building toward these. -->

- [ ] Provider CRUD（创建、读取、编辑、删除）
- [ ] Provider 切换 — surgical patch 到 CLI 配置文件（只改凭据+模型字段）
- [ ] 支持 Claude Code CLI 配置适配（~/.claude/settings.json）
- [ ] 支持 Codex CLI 配置适配（~/.codex/auth.json + config.toml）
- [ ] iCloud 同步层 — 每个 Provider 一个独立 JSON 文件存放在 iCloud Drive 目录
- [ ] 设备本地层 — 当前激活 Provider、设备路径覆盖等存放在本地（不同步）
- [ ] 文件监听 — 监听 iCloud 同步目录变化，自动刷新 UI
- [ ] 活跃 Provider 联动 — 同步目录中活跃 Provider 变化时，自动重新 patch CLI 配置
- [ ] 首次启动自动导入 — 扫描现有 CLI 配置创建初始 Provider
- [ ] 手动创建 Provider
- [ ] i18n 国际化 — 中英双语，默认中文，可扩展

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- MCP 服务器管理 — v1 只做 Providers，MCP 后续里程碑
- Prompts / Skills 管理 — 同上
- 系统托盘快速切换 — v2 再做
- 本地代理转发 — 未来功能，但文件监听架构需为此预留扩展点
- Proxy / Failover / Usage 统计 — cc-switch 中最臃肿的模块，明确排除
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
| Surgical patch 而非整文件重写 | cc-switch 的 atomic_write 整文件替换导致配置损坏，丢失用户其他设置 | — Pending |
| 每 Provider 一个 JSON 文件 | iCloud 对单文件更新处理最好，避免 SQLite 放云盘的经典雷区 | — Pending |
| 数据分层（同步层 + 本地层） | 设备级设置（当前激活 Provider）不应跨设备同步，避免互相覆盖 | — Pending |
| Provider 按协议类型建模 | 未来 CLI（如 OpenCode）可能支持多协议，按协议复用 Provider 比按 CLI 绑定更灵活 | — Pending |
| Read-Modify-Write 不加文件锁 | CLI 与 CLIManager 同时写同一配置的概率极低，文件锁增加复杂度收益不大 | — Pending |
| v1 不做托盘 | 聚焦核心功能，托盘快切 v2 再加 | — Pending |
| i18n 从 v1 开始 | 避免后期改造成本，中英双语默认中文 | — Pending |

---
*Last updated: 2026-03-10 after initialization*
