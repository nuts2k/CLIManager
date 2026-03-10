# Phase 1: Storage and Data Model - Context

**Gathered:** 2026-03-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Two-layer存储架构（iCloud 同步层 + 设备本地层）和基于协议类型的 Provider 数据模型。包含 Tauri 2 项目脚手架搭建。不涉及 UI 界面、CLI 适配器、文件监听等后续阶段功能。

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

用户将以下所有实现决策交由 Claude 自行决定：

**Provider JSON Schema**
- 每个 Provider 文件的字段设计（name, protocol_type, api_key, base_url, model 等）
- 协议类型枚举（Anthropic, OpenAI-compatible 等）及其可扩展性
- 文件命名规范（如 UUID.json 或 slug-based）
- JSON schema 版本控制策略

**iCloud 目录位置**
- iCloud Drive 中的具体存放路径
- 子文件夹命名方案
- iCloud Drive 不可用时的降级策略

**本地设置 Schema**
- `~/.cli-manager/local.json` 的字段结构
- 活跃 Provider 追踪方式（按 CLI 分别追踪还是全局统一）
- 路径覆盖和默认值设计

**项目脚手架选型**
- 包管理器选择
- React 配置方案
- 状态管理方案
- 测试框架选择

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- 项目为全新创建（greenfield），无现有代码可复用
- `cc-switch/` 目录包含参考实现（只读），可参考其 Rust 后端数据模型和 Tauri 命令结构

### Established Patterns
- cc-switch 使用 SQLite 存储 Provider 数据 — CLIManager 改为每 Provider 一个 JSON 文件，避免 SQLite + iCloud 的已知问题
- cc-switch 使用 atomic_write 整文件重写 — CLIManager 改为 surgical patch

### Integration Points
- Provider JSON 文件将被 Phase 2（Surgical Patch Engine）的 CLI 适配器读取
- 本地设置将被 Phase 3（UI）和 Phase 4（File Watching）使用
- 数据模型需为 Phase 4 的 FSEvents 文件监听预留合理的文件组织结构

</code_context>

<specifics>
## Specific Ideas

- 用户偏好中文沟通，UI 相关决策在 Phase 3 处理
- 参考 cc-switch 但不照搬其 SQLite + 全家桶架构，保持精简

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-storage-and-data-model*
*Context gathered: 2026-03-10*
