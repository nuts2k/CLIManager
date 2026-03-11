# Phase 2: Surgical Patch Engine - Context

**Gathered:** 2026-03-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Read-Modify-Write CLI 适配器，切换 Provider 时只修改凭据和模型字段，保留配置文件其他内容不变。覆盖 Claude Code CLI（settings.json）和 Codex CLI（auth.json + config.toml）。不涉及 UI、文件监听、onboarding 等后续阶段功能。

</domain>

<decisions>
## Implementation Decisions

### 字段映射

- Claude Code CLI (`~/.claude/settings.json`): 写入 `api_key` + `base_url`，暂不写 `model`
- Codex CLI (`~/.codex/auth.json`): 写入 `api_key`
- Codex CLI (`~/.codex/config.toml`): 写入 `base_url`，暂不写 `model`
- `model_config`（haiku/sonnet/opus/reasoning_effort）v1 暂不处理
- 架构上预留扩展能力，未来可灵活添加更多字段映射

### 适配器架构

- 使用 Rust trait 抽象统一适配器接口（read_config / patch / backup / validate）
- ClaudeAdapter 和 CodexAdapter 各自实现 trait
- 未来加新 CLI 只需添加新的 impl

### 备份策略

- 备份文件统一存放在 `~/.cli-manager/backups/` 下，按 CLI 分子目录（claude/、codex/）
- 每次 patch 前都备份，使用时间戳后缀命名（如 `settings.json.2026-03-11T10-30-00.bak`）
- 最多保留 5 份备份，超过时自动删除最旧的
- 配置文件不存在时跳过备份（没有原文件可备份）

### Codex 双文件回滚

- 顺序写入：先 auth.json，再 config.toml
- auth.json 写入失败则直接报错退出（两个文件都未变）
- config.toml 写入失败则从备份恢复 auth.json，然后报错退出
- TOML 注释和原始格式必须保留（使用 TOML 感知的库解析修改）

### 验证规则

- patch 前验证：原文件格式合法性（JSON/TOML 可解析）
- patch 后验证：结果文件格式合法性（确保 patch 没有破坏文件结构）
- 只检查格式合法性，不检查字段语义正确性
- 任一验证失败则中止写入

### 配置文件缺失处理

- 目标 CLI 配置文件不存在时自动创建新文件，只包含需要 patch 的字段
- 不备份（没有原文件）

### Claude's Discretion

- 具体的 Rust TOML 库选择（需支持保留注释和格式）
- trait 的具体方法签名和错误类型设计
- 时间戳格式的精确规范
- 备份清理的触发时机（patch 前清理还是 patch 后清理）

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Provider` struct (`src-tauri/src/provider.rs`): 包含 api_key、base_url、model、protocol_type 等字段，patch 时从这里读取值
- `CliPaths` struct (`src-tauri/src/storage/local.rs`): 已有 claude_config_dir 和 codex_config_dir 可选路径覆盖，适配器可用这些路径定位配置文件
- `AppError` enum (`src-tauri/src/error.rs`): 需扩展以支持 patch 相关错误（如验证失败、回滚失败）
- `atomic_write` utility (`src-tauri/src/storage/mod.rs`): 已有原子写入工具，可复用于 JSON 文件写入
- `_in/_to` 内部函数模式: Phase 1 建立的测试隔离模式，适配器也应遵循

### Established Patterns
- Tauri commands 是 thin wrapper，业务逻辑在独立模块中
- serde 用于所有序列化/反序列化
- tempfile crate 已在依赖中（用于原子写入）

### Integration Points
- 适配器模块将被 Phase 3 的 UI 层通过 Tauri commands 调用（一键切换 Provider）
- `set_active_provider` command 目前只更新 LocalSettings，Phase 2 需要在此基础上触发 patch
- Phase 4 的文件监听在活跃 Provider 变更时也会调用 patch 逻辑

</code_context>

<specifics>
## Specific Ideas

- 用户偏好中文沟通
- TOML 注释保留是硬性要求（PTCH-02），不能使用会丢失注释的 TOML 库
- 字段映射设计为可扩展：虽然 v1 只写 api_key + base_url，但架构不应限制未来添加更多字段

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-surgical-patch-engine*
*Context gathered: 2026-03-11*
