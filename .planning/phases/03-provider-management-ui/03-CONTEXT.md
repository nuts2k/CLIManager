# Phase 3: Provider Management UI - Context

**Gathered:** 2026-03-11
**Status:** Ready for planning

<domain>
## Phase Boundary

完整的 Provider CRUD 界面，支持一键切换（切换即 patch CLI 配置）、i18n 中英双语。按 CLI 分 Tab 独立管理各自的 Provider 列表。包含 Provider 可用性测试功能。不涉及 iCloud 文件监听（Phase 4）和首次启动自动导入（Phase 5）。

</domain>

<decisions>
## Implementation Decisions

### 整体布局和视觉风格
- 顶部 Tab 按 CLI 分组（Claude Code / Codex），切换 Tab 显示该 CLI 下独立的 Provider 列表（类似 cc-switch 的 AppSwitcher）
- 每个 Provider 以卡片行形式展示（类 cc-switch 的 ProviderCard），每行显示：名称、base_url（截断）、当前激活标识
- 悬停卡片行时滑出操作按钮
- 右上角设置图标（齿轮），点击进入设置页面
- 默认窗口大小约 1000x700
- 仅深色主题，不支持浅色
- UI 库使用 shadcn/ui + Tailwind CSS

### Provider 表单交互
- 创建/编辑 Provider 使用 Dialog 弹窗
- 精简必填字段 + 高级折叠区域：必填为 name、API Key、Base URL；model、notes、protocol_type、model_config 折叠在"高级"区域
- API Key 字段默认遮盖显示（***），提供切换按钮可显示明文
- Base URL 不预填默认值，用户手动输入

### 切换体验和状态展示
- 悬停卡片行时滑出"切换"按钮，点击即切换（一键切换）
- 当前活跃 Provider 通过左侧蓝色标识条 + 卡片边框高亮标识
- 切换成功/失败通过右下角 Toast 通知反馈
- 切换 Provider 时立即触发当前 Tab CLI 的适配器进行 surgical patch（切换即 patch，一步到位）
- 需要修改 set_active_provider 命令：除了更新 LocalSettings，还要调用对应 CLI 的适配器执行 patch

### Provider 操作
- 悬停操作按钮包含：切换、编辑、复制（当前 CLI 内复制）、复制到其他 CLI、测试、删除
- **复制**：在当前 CLI Tab 内复制一个相同配置的新 Provider，名称加 "(copy)" 后缀
- **复制到其他 CLI**：悬停操作中加"复制到..."按钮，点击后弹出下拉选择目标 CLI，确认后在目标 CLI 下创建副本，名称加 "(copy from Claude)" 等后缀，用户可再编辑协议等
- **测试**：API 连通性测试，后端用 Provider 的 API key 和 base_url 发送简单请求，返回"运行正常"/"失败" + 响应时间。测试结果通过 Toast 显示
- **测试配置**：在设置页中可配置超时时间、测试用模型等，都提供默认值。暂不考虑降级阈值（那属于代理轮换功能）
- **删除**：弹出确认 Dialog 防止误操作
- **删除后自动切换**：删除当前活跃 Provider 时，自动切换到列表中下一个可用 Provider（首尾循环查找，只要列表不为空一定能找到），并触发 patch。删除非活跃 Provider 不影响当前状态。列表为空时清除活跃状态

### 加载和错误状态
- 操作过程中按钮显示 loading spinner，禁用重复点击
- 操作失败通过 Toast 显示错误信息

### 空状态
- 某个 CLI Tab 下没有 Provider 时，显示友好提示文字 + "新建 Provider" 按钮

### Provider 按 CLI 隔离（数据模型变更）
- **重要变更**：原设计是 Provider 按协议类型复用到不同 CLI，现改为每个 CLI 独立管理自己的 Provider 列表
- Provider JSON 文件增加 `cli_id` 字段（如 "claude" / "codex"），读取时按 cli_id 过滤
- LocalSettings 中活跃 Provider 改为按 CLI 分别记录：`active_providers: { "claude": "xxx", "codex": "yyy" }`
- 需要调整 Phase 1 的存储层代码和 Tauri commands

### i18n
- 使用 react-i18next，JSON 文件存储翻译
- 默认中文，可切换英文
- 语言切换入口在设置页下拉菜单中
- 语言选择持久化到 LocalSettings

### 设置页内容
- 语言切换（中/英下拉菜单）
- 测试配置（超时时间、测试用模型，都有默认值）
- 关于信息（版本号、项目信息）

### Claude's Discretion
- 具体的 shadcn/ui 组件选择和组合方式
- 卡片行的具体间距、字体大小、动画效果
- Toast 通知的展示时间和位置细节
- 设置页的具体布局设计
- 状态管理方案（React Query、Zustand 等）
- 测试请求的具体实现（prompt 内容、模型选择逻辑）
- 空状态的具体提示文案和视觉设计

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Provider` struct (`src-tauri/src/provider.rs`): 包含 id, name, protocol_type, api_key, base_url, model, model_config, notes。需要新增 `cli_id` 字段
- Tauri commands (`src-tauri/src/commands/provider.rs`): list_providers, get_provider, create_provider, update_provider, delete_provider, get_local_settings, set_active_provider — 需要扩展以支持 cli_id 过滤和按 CLI 的活跃 Provider 管理
- `CliAdapter` trait + ClaudeAdapter/CodexAdapter (`src-tauri/src/adapter/`): 已实现 surgical patch 逻辑，切换时需要调用
- `LocalSettings` struct (`src-tauri/src/storage/local.rs`): 需要从单一 active_provider_id 改为 active_providers HashMap
- `AppError` enum (`src-tauri/src/error.rs`): 可扩展以支持 UI 相关错误

### Established Patterns
- Tauri commands 是 thin wrapper，业务逻辑在独立模块中
- serde 用于所有序列化/反序列化
- `_in/_to` 内部函数模式用于测试隔离

### Integration Points
- `set_active_provider` command 需要重构：更新 LocalSettings + 调用适配器 patch
- 前端是空壳 React（App.tsx 只是占位符），需要从零搭建 UI
- cc-switch 的 AppSwitcher、ProviderCard、ProviderList 可作为设计参考（只读）

</code_context>

<specifics>
## Specific Ideas

- 用户偏好中文沟通，UI 默认中文
- 类似 cc-switch 的卡片行 + 顶部 Tab 布局，但去掉所有 cc-switch 的臃肿功能（proxy、failover、OMO、usage tracking 等）
- "复制到其他 CLI" 功能方便多协议 Provider 配置复用，避免重复输入
- 删除后自动切换采用首尾循环查找，确保列表不为空时一定能找到可用 Provider

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-provider-management-ui*
*Context gathered: 2026-03-11*
