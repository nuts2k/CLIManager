# Phase 5: Onboarding - Context

**Gathered:** 2026-03-12
**Status:** Ready for planning

<domain>
## Phase Boundary

首次启动自动导入 -- 扫描现有 CLI 配置文件（~/.claude/settings.json、~/.codex/auth.json），检测已有的 API 凭据，让用户确认后创建初始 Provider。用户也可以跳过导入，手动创建 Provider（Phase 3 已实现）。不涉及新的 CLI 适配器、新的 CRUD 功能或同步逻辑。

</domain>

<decisions>
## Implementation Decisions

### 触发时机
- 当所有 CLI 的 Provider 列表都为空时触发 onboarding 弹窗（不使用额外的 onboarding_completed 标记）
- 用户删除所有 Provider 后重新打开 app 也会重新触发
- 先弹出导入 Dialog，用户跳过后进入主界面空状态页面
- 设置页提供"从 CLI 配置导入"按钮，用户可随时手动触发导入流程

### 弹窗形式
- 居中 Dialog，复用现有 shadcn/ui Dialog 组件，与创建/编辑 Provider 的 Dialog 风格一致
- 纯文字标题"导入现有配置"，无欢迎语、品牌元素或额外装饰
- 背景模糊（与现有 Dialog 行为一致）

### 检测配置预览
- 摘要确认模式：展示检测到的 CLI 配置列表
- 每个检测到的 CLI 配置显示：CLI 名称、脱敏 API Key、Base URL
- API Key 脱敏：首尾可见（如 sk-ant-ap...H7kQ），中间用 ... 省略
- 每项前有勾选框，默认全选，用户可取消不需要的项
- 底部两个按钮："导入已选项"和"跳过"

### 命名和默认值
- 自动命名："{CLI名} 默认配置"（如 "Claude 默认配置"、"Codex 默认配置"）
- protocol_type 按 CLI 原生协议：Claude -> Anthropic，Codex -> OpenAI 兼容
- 最小字段填充：只导入 API Key 和 Base URL，model 等其他字段留空由用户编辑补充
- 导入后不自动激活：Provider 创建后不设为活跃，用户需手动点击切换激活

### 去重策略
- 导入前比较 API Key + Base URL，如果已存在相同配置的 Provider 则跳过该项
- 防止手动触发重新导入时创建重复 Provider

### 缺失/部分配置处理
- 全部不存在（~/.claude/ 和 ~/.codex/ 都没有）：静默跳过 onboarding，直接进入主界面
- 配置文件存在但缺少 API Key：仍然导入，在预览中标注"缺少 API Key"，用户导入后可编辑补充
- 配置文件格式损坏（非法 JSON/TOML）：静默跳过该 CLI，后台 log 记录错误
- 只检测到一个 CLI 的配置：正常展示该项，不提及未检测到的 CLI

### CLI 检测范围
- 只检测配置文件是否存在并可读取（~/.claude/settings.json、~/.codex/auth.json）
- 不检测 CLI 工具本身是否安装（不执行 which claude 等命令）

### Claude's Discretion
- 导入流程的 Tauri command 设计和 Rust 实现细节
- Dialog 内部的具体布局、间距、动画
- 后端配置文件解析的具体实现
- 去重比较的精确匹配逻辑
- Toast 通知的具体文案

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ClaudeAdapter` (`src-tauri/src/adapter/claude.rs`): 已有读取 ~/.claude/settings.json 的逻辑，可参考其路径和字段提取方式
- `CodexAdapter` (`src-tauri/src/adapter/codex.rs`): 已有读取 ~/.codex/auth.json 和 config.toml 的逻辑
- `create_provider` command (`src-tauri/src/commands/provider.rs`): 创建 Provider 的 Tauri command，导入时可复用
- `Provider` struct (`src-tauri/src/provider.rs`): 包含 cli_id, api_key, base_url 等字段
- Dialog 组件 (src/components/provider/): 现有 CreateProviderDialog 的 Dialog 样式可参考
- `useProviders` hook (src/hooks/useProviders.ts): 可用于导入后刷新 Provider 列表
- i18n 翻译文件: 新增导入相关的中英文翻译 key

### Established Patterns
- Tauri commands 是 thin wrapper，业务逻辑在独立模块中
- Dialog 状态由父组件管理，通过 props 传递
- Toast 通知用于操作反馈
- shadcn/ui 组件 + Tailwind CSS 深色主题

### Integration Points
- 前端 App 组件或 ProviderTabs 需要在 mount 时检测 Provider 是否为空，触发 onboarding Dialog
- 设置页 (SettingsPage) 需要新增"从 CLI 配置导入"按钮
- 后端需要新增 scan_cli_configs Tauri command 用于检测和提取配置
- 导入使用现有 create_provider command 逐个创建

</code_context>

<specifics>
## Specific Ideas

- 与现有 Dialog 风格保持一致，不引入新的 UI 范式
- 导入是非侵入性的：不自动激活、不自动 patch CLI 配置，只创建 Provider 数据

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 05-onboarding*
*Context gathered: 2026-03-12*
