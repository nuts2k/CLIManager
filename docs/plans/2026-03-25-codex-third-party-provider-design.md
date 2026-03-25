# Codex 第三方 Provider 配置重建设计

## 背景

当前 CLIManager 对 Codex 的配置写入采用局部 patch 策略：

- `auth.json` 仅 patch 顶层 `OPENAI_API_KEY`
- `config.toml` 仅 patch 顶层 `base_url`，或在已有 `model_provider` 时 patch `model_providers.<active>.base_url`

这种策略在从官方 OAuth 登录态切换到第三方 API key Provider 时会产生两个问题：

1. `auth.json` 容易残留 OAuth 相关字段（如 `auth_mode = "chatgpt"`、`tokens` 等），形成混合态配置
2. `config.toml` 会停留在一种“不完整的第三方 provider 形态”，缺少完整的 provider 结构定义，导致 Codex 运行时行为不稳定或不符合预期

另外，实测验证表明，当前第三方服务 `dtcch.nuts2k.eu.org` 在 Codex 场景下要求 `base_url` 保留 `/v1` 前缀；将其裁剪为纯 origin 会导致无法正常使用。

## 问题定义

CLIManager 当前对 Codex 第三方 Provider 的写入问题不是单个字段错误，而是整体策略错误：

- 不应继续在历史遗留的 OAuth/官方配置上做局部 patch
- 应在切换到第三方 Provider 时，将 Codex 的 Provider 相关配置整体切换为“第三方 provider 形态”

这里的“Provider 相关配置”包括两部分：

1. `~/.codex/auth.json` 中的认证信息
2. `~/.codex/config.toml` 中的 provider 结构与路由信息

与此同时，用户已有的非 provider 配置仍然需要保留，例如：

- `[projects]`
- `[mcp_servers.*]`
- `[notice.*]`

## 设计目标

在 CLIManager 切换到第三方 Codex Provider 时，实现以下目标：

1. 将 `auth.json` 整体替换为第三方 API key 认证结构
2. 将 `config.toml` 中的 Provider 相关部分整体重建为完整第三方 provider 结构
3. 保留 `config.toml` 中与 provider 无关的用户配置
4. 对 Codex/OpenAI Provider 保留带路径的 `base_url`（尤其是 `/v1`）
5. 避免 OAuth 配置与第三方 Provider 配置混杂

## 非目标

本次设计不处理以下内容：

1. 不改变 Claude/Gemini 的配置写入策略
2. 不设计官方 OAuth Provider 与第三方 Provider 的统一抽象层
3. 不在本次内引入更多通用配置模板系统
4. 不处理 Codex 代理模式（proxy takeover）下的独立配置策略调整

## 推荐方案

采用“第三方 Provider 整体重建模式”：

### 1. `auth.json` 采用整体替换策略

当目标 Provider 为第三方 Codex Provider 时，`auth.json` 不再基于现有文件 patch，而是直接写成最小第三方认证结构：

```json
{
  "OPENAI_API_KEY": "sk-..."
}
```

这样可以保证：

- 不再保留 `auth_mode = "chatgpt"`
- 不再保留 `tokens`
- 不再保留 `refresh_token` / `id_token` / `last_refresh`
- 不再出现 OAuth 与 API key 的混合态

### 2. `config.toml` 采用“保留非 provider 块 + 重建 provider 块”策略

切换到第三方 Provider 时，对 `config.toml` 做结构化处理：

#### 保留的块

以下块原样保留：

- `[projects.*]`
- `[mcp_servers.*]`
- `[notice.*]`

#### 重建的块

以下内容统一重建：

- `model_provider`
- `model`
- `model_reasoning_effort`
- `disable_response_storage`
- `[model_providers.<provider_name>]`

目标形态如下：

```toml
model_provider = "jp_codex"
model = "gpt-5.4"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.jp_codex]
name = "jp_codex"
base_url = "https://dtcch.nuts2k.eu.org/v1"
wire_api = "responses"
requires_openai_auth = true
```

### 3. Provider 名称需要做 sanitize

写入 `model_provider` 和 `[model_providers.<name>]` 时，需要将 Provider 名称转换为安全的 TOML key，例如：

- `JP Codex` → `jp_codex`

建议参考 cc-switch 的清理规则：

- 转小写
- 非 `[a-z0-9_]` 替换为 `_`
- 去掉首尾 `_`
- 空值 fallback 为 `custom`

### 4. `model` 优先取 `upstream_model`

对于 Codex 第三方 Provider，`config.toml` 中的 `model` 应优先使用：

1. `provider.upstream_model`
2. 若为空，则使用默认值（例如 `gpt-5.4`）

原因是当前 Provider 数据中：

- `model` 往往为空或不适合作为真实上游模型
- `upstream_model` 才是第三方 Provider 的实际目标模型

### 5. `base_url` 必须保留 `/v1`

Codex 第三方 Provider 的 `base_url` 不能在写入前裁剪为纯 origin。对于 OpenAI/Codex 类 Provider，必须允许保留 path，例如：

- `https://dtcch.nuts2k.eu.org/v1`
- `https://gateway.example.com/openai/v1`

这是因为实测已验证：

- `https://dtcch.nuts2k.eu.org` 无法正常使用
- `https://dtcch.nuts2k.eu.org/v1` 可以正常使用

因此，CLIManager 现有“规范化为 origin”的策略不适用于 Codex 第三方 Provider 写盘场景。

## 方案对比

### 方案 A：整体重建 Provider 配置（推荐）

特点：

- `auth.json` 整体替换
- `config.toml` 保留非 provider 块，重建 provider 块

优点：

- 配置语义清晰
- 最接近 cc-switch 的成功路径
- 不会残留 OAuth 混合态
- 更适合长期维护

缺点：

- 需要明确定义 provider 相关块与非 provider 块的边界

### 方案 B：继续局部 patch，但补齐缺失字段

特点：

- 在现有 `config.toml` 上增量补齐 `model_provider`、`model_providers`、`wire_api` 等字段
- `auth.json` 仍整体替换

优点：

- 表面改动较小

缺点：

- 容易残留历史脏状态
- 逻辑复杂度持续增加
- 仍难以避免官方/OAuth/第三方配置混杂

### 方案 C：双模板模式（官方与第三方分离）

特点：

- 为 OAuth 官方模式和第三方 API key 模式分别维护两套完整模板

优点：

- 概念清晰

缺点：

- 相比当前问题需求偏重
- 当前阶段没有必要引入额外抽象

## 推荐结论

采用方案 A：

> 在切换到第三方 Codex Provider 时，CLIManager 应整体重建 `auth.json` 和 `config.toml` 的 Provider 相关配置，同时保留用户已有的非 provider 配置。

## 数据流设计

第三方 Codex Provider 切换流程应为：

1. 用户在 CLIManager 中切换 Codex Provider
2. `set_active_provider(...)` 进入 CodexAdapter 写盘逻辑
3. 写盘逻辑生成新的第三方 `auth.json`
4. 读取现有 `config.toml`
5. 提取并保留非 provider 配置块
6. 基于当前 Provider 生成完整第三方 provider 块
7. 合成新的 `config.toml`
8. 写回两个 live 文件

## 代码落点建议

### 主要改动文件

- `src-tauri/src/adapter/codex.rs`

该文件应承担主要改动：

1. `auth.json` 从 patch 改为整体替换
2. `config.toml` 从局部 patch 改为“保留 + 重建”
3. 引入 provider name sanitize 逻辑
4. 处理 `upstream_model` 优先级
5. 保留 `base_url` 中的 `/v1`

### 可能需要调整的辅助文件

- `src-tauri/src/provider.rs`
- `src-tauri/src/commands/provider.rs`

如现有 base_url 规范化逻辑仍会裁剪 `/v1`，则需要明确放宽 Codex/OpenAI Provider 的 URL 规则，避免写盘前破坏有效配置。

## 测试设计

### 1. `auth.json` 替换测试

#### 场景

输入为 OAuth 登录态：

```json
{
  "auth_mode": "chatgpt",
  "OPENAI_API_KEY": null,
  "tokens": { ... },
  "last_refresh": "..."
}
```

输出应为：

```json
{
  "OPENAI_API_KEY": "sk-..."
}
```

#### 验收点

- `auth_mode` 不再保留
- `tokens` 不再保留
- `last_refresh` 不再保留
- 仅保留第三方认证字段

### 2. `config.toml` 重建测试

#### 场景

输入为当前“半 patch 状态”配置：

```toml
model = "gpt-5.4"
model_reasoning_effort = "high"
base_url = "https://dtcch.nuts2k.eu.org"

[projects."/Users/kelin/Workspace/CLIManager"]
trust_level = "trusted"

[mcp_servers.fast-context]
...
```

输出应为完整第三方结构：

```toml
model_provider = "jp_codex"
model = "gpt-5.4"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.jp_codex]
name = "jp_codex"
base_url = "https://dtcch.nuts2k.eu.org/v1"
wire_api = "responses"
requires_openai_auth = true
```

并保留原有：

- `[projects]`
- `[mcp_servers]`
- `[notice]`

### 3. `/v1` 保留测试

输入 Provider：

```json
{
  "base_url": "https://dtcch.nuts2k.eu.org/v1"
}
```

输出 TOML 中必须仍为：

```toml
base_url = "https://dtcch.nuts2k.eu.org/v1"
```

#### 验收点

- `/v1` 不被移除
- 不被裁剪为 origin

### 4. 非 provider 配置保留测试

输入 TOML 含：

- `[projects.*]`
- `[mcp_servers.*]`
- `[notice.*]`

输出后这些块应保持不变。

## 验收标准

当 CLIManager 切换到第三方 Codex Provider 时，应满足：

1. `auth.json` 为纯第三方 API key 结构，不含 OAuth 遗留字段
2. `config.toml` 为完整第三方 provider 结构，而非局部 patch 后的半成品
3. `base_url` 保留 `/v1`
4. 用户原有 `[projects]`、`[mcp_servers]`、`[notice]` 配置不丢失
5. Codex 不再因配置混合态要求重新登录

## 当前结论

本问题的本质不是单个字段 patch 错误，而是 CLIManager 对 Codex 第三方 Provider 使用了错误的配置写入策略。后续实现应从“局部 patch 历史文件”转向“整体重建 Provider 相关配置”。
