# 设计文档：Claude Code 全局配置（settings.json overlay）

**日期：** 2026-03-16

## 背景与动机

CLIManager 当前对 Claude Code 的配置写入遵循核心价值：**切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容。**

目前 ClaudeAdapter 仅修改 `~/.claude/settings.json` 中的两个字段：

- `env.ANTHROPIC_AUTH_TOKEN`
- `env.ANTHROPIC_BASE_URL`

但实际使用中还存在一类与 Provider 无关、希望在所有 Provider 下都保持一致的配置，例如：

```json
{
  "env": {
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
    "ENABLE_TOOL_SEARCH": "true"
  }
}
```

用户希望：
1. 在应用设置页配置这些“Claude 全局配置”；
2. 在合适的时机自动写入/修正 `~/.claude/settings.json`；
3. 多台电脑间同步（iCloud 可用时）；
4. 严格不破坏既有的 surgical patch 行为。

## 目标

- 在 Settings → Advanced 中新增 **Claude 小节**，允许用户维护一段 JSON 片段（overlay）。
- overlay 可覆盖 `settings.json` 的任意板块（不限于 env）。
- overlay 应用规则为 **深度合并**，并支持 **null 删除字段**。
- **Provider/Proxy patch 永远优先**：overlay 不允许覆盖 `env.ANTHROPIC_AUTH_TOKEN` / `env.ANTHROPIC_BASE_URL`。
- overlay 存储支持跨设备同步：
  - iCloud 可用：存到 iCloud 目录
  - iCloud 不可用：降级存到本地目录（功能可用但不同步）

## 非目标（v2.5 不做）

- 不实现通用机制（先只针对 Claude Code，不为 Codex 等 CLI 抽象通用 overlay 系统）。
- 不做 UI 表单化（v2.5 采用 JSON 编辑为主）。
- 不引入复杂的规则引擎（如 JSONPath 列表规则）；只做 JSON 片段 overlay。

## 术语

- **Provider patch**：现有 ClaudeAdapter 对 `ANTHROPIC_AUTH_TOKEN/BASE_URL` 的写入。
- **Proxy patch**：代理模式下将 Claude 配置写成 `PROXY_MANAGED + localhost`。
- **Overlay**：用户维护的 JSON 片段，通过深度合并应用到 `~/.claude/settings.json`。
- **保护字段**：由 Provider/Proxy 管理、overlay 不得覆盖的字段（当前为 `env.ANTHROPIC_AUTH_TOKEN`、`env.ANTHROPIC_BASE_URL`）。

## 存储设计（同步优先、降级本地）

### 存储位置

- **首选（可同步）**：`<iCloud>/CLIManager/config/claude-settings-overlay.json`
- **降级（本地）**：`~/.cli-manager/config/claude-settings-overlay.json`

iCloud 不可用时：
- 仍允许编辑与应用 overlay
- 但提示用户“当前为本地存储，不会跨设备同步”

### 文件内容

文件内容为一个 JSON 对象（serde_json::Value root 必须为 object）。

- 文件不存在：视为没有 overlay（noop）
- 文件存在但 JSON 不合法：视为配置错误（需要 UI 提示；后端 apply 返回错误）

## 合并规则

采用深度合并，规则如下：

- **对象（object）**：递归合并（key-by-key）
- **标量（string/number/bool）**：overlay 覆盖原值
- **数组（array）**：整体替换
- **null**：删除该 key（例如 `{ "permissions": null }` 删除整个 permissions）

## Provider/Proxy 永远优先（保护字段）

### 保护字段集合（v2.5）

- `env.ANTHROPIC_AUTH_TOKEN`
- `env.ANTHROPIC_BASE_URL`

### 强制规则

- overlay 中若包含保护字段：**忽略该字段**（可以在 UI 中提示“该字段由 Provider/Proxy 管理，不可覆盖”）。
- 应用 overlay 后，仍需 **最终强制写回** Provider/Proxy 的两个字段，保证“Provider/Proxy 永远优先”。

## 写入/应用时机（触发点）

### 1) 保存设置时（强一致）

- 用户在 Settings → Advanced → Claude 小节保存 overlay：
  1. 写 overlay 文件（iCloud or 本地 fallback）
  2. 立即调用 apply，将 overlay 合并进 `~/.claude/settings.json`

### 2) 应用启动时（自愈对齐，best-effort）

- 后端启动（Tauri `.setup()`）阶段执行一次 apply（overlay 存在时）。
- apply 失败只记录日志/发送事件，不阻断应用启动。

### 3) iCloud 同步变更时（跨设备自动生效）

- 扩展文件 watcher：监听 `<iCloud>/CLIManager/config` 目录。
- 当 `claude-settings-overlay.json` 变更时自动 apply。

> 若只依赖“启动时对齐”，同步后的生效需要用户重启应用，不符合直觉，因此 v2.5 推荐加 watcher。

## 后端模块划分（建议）

### storage

- `storage/icloud.rs`：新增 config dir 解析与 overlay 文件读写
  - `get_icloud_config_dir()`：iCloud 可用则返回 iCloud config dir；不可用则 fallback `~/.cli-manager/config`
  - `read_claude_settings_overlay()` / `write_claude_settings_overlay()`

### adapter

- `adapter/claude.rs`：升级 patch 流程
  - provider/proxy patch → overlay merge（剔除保护字段）→ 最终强制写回保护字段

- 新增纯函数合并模块（建议 `adapter/json_merge.rs`）
  - `merge_with_null_delete(base, overlay)`
  - `strip_protected_paths(overlay, protected_paths)`

### commands

- 新增 `commands/claude_settings.rs`（或放入 provider.rs，但推荐单独文件）
  - `get_claude_settings_overlay`
  - `set_claude_settings_overlay`
  - `apply_claude_settings_overlay`

### watcher

- 扩展 watcher：新增对 config 目录的监听
  - overlay 变化 → apply → emit 事件（用于 toast / UI 状态）

## 前端 UI（Settings → Advanced）

- Advanced tab 增加 Claude 小节
  - 多行 JSON 编辑框（textarea）
  - 保存前 JSON 校验，不合法则 toast 错误并拒绝保存
  - 保存成功后立即调用 apply
  - 提示文案：`env.ANTHROPIC_AUTH_TOKEN` / `env.ANTHROPIC_BASE_URL` 不可在此覆盖

## 错误处理与安全性

- `~/.claude/settings.json` 若存在但不是合法 JSON：保持现有行为（Validation error）。
- overlay 文件 JSON 不合法：apply 返回错误；UI toast 提示。
- apply 过程使用 atomic_write，保留备份（沿用 ClaudeAdapter 备份策略）。

## 测试计划（Rust）

1. 合并算法纯函数测试：对象递归、数组替换、标量覆盖、null 删除。
2. 保护字段测试：overlay 写入 `env.ANTHROPIC_BASE_URL` 不应覆盖 provider/proxy 写入结果。
3. ClaudeAdapter patch 集成测试：overlay 注入额外 env 字段，patch 后存在且不影响其他字段。
4. iCloud fallback 测试：iCloud 不可用时 overlay 读写走本地 config 目录。

## 验收标准（Done）

- [ ] Settings → Advanced 中出现 Claude 小节，可编辑 JSON overlay
- [ ] JSON 不合法时无法保存/应用，且有错误提示
- [ ] 保存后立即应用到 `~/.claude/settings.json`
- [ ] 应用启动时自动对齐一次（overlay 存在时）
- [ ] iCloud 同步 overlay 变更后自动应用（watcher）
- [ ] Provider/Proxy 永远优先：`env.ANTHROPIC_AUTH_TOKEN/BASE_URL` 不可被 overlay 覆盖
- [ ] 相关单测覆盖合并规则与保护字段
