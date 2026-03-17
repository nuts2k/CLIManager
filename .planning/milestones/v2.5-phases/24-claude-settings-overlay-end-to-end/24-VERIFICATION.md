---
phase: 24-claude-settings-overlay-end-to-end
verified: 2026-03-17T09:00:00Z
status: gaps_found
score: 9/10 must-haves verified
re_verification: false
gaps:
  - truth: "用户首次进入 Settings → Advanced → Claude 小节时，会通过 getClaudeSettingsOverlay() 读取已保存 overlay 并回填编辑框"
    status: failed
    reason: "后端 GetClaudeSettingsOverlayResponse 字段名为 content，但前端 ClaudeSettingsOverlayState 类型声明 overlay_json，SettingsPage 读取 state.overlay_json 永远为 undefined，编辑框永远为空"
    artifacts:
      - path: "src-tauri/src/commands/claude_settings.rs"
        issue: "GetClaudeSettingsOverlayResponse { pub content: Option<String>, ... } — 字段名 content 与前端约定不一致"
      - path: "src/lib/tauri.ts"
        issue: "ClaudeSettingsOverlayState { overlay_json: string | null } — 前端期待 overlay_json 字段，但后端实际序列化为 content"
    missing:
      - "将后端 GetClaudeSettingsOverlayResponse 的 content 字段重命名为 overlay_content，或添加 #[serde(rename = \"overlay_json\")] 属性，确保与前端 ClaudeSettingsOverlayState.overlay_json 对应"
      - "或将 src/lib/tauri.ts 中的 ClaudeSettingsOverlayState.overlay_json 改为 content，并同步修改 SettingsPage.tsx 中的 state.overlay_json 引用"
human_verification:
  - test: "验证 Claude overlay 编辑/校验/保存/位置显示"
    expected: "Settings → Advanced → Claude 小节正常加载已保存 overlay；非法 JSON 被拒绝；合法 JSON 可保存并回填后端 pretty 化内容；存储位置/路径/同步状态展示正确"
    why_human: "UI 交互行为、toast 可见性、内联错误块显示等无法通过代码静态分析验证；且当前 overlay_json 字段名 bug 会影响加载回填，需人工确认实际运行效果"
  - test: "验证 overlay apply 三类触发点与 startup 缓存回放"
    expected: "保存/启动/watcher 三条路径均触发 apply；startup 结果在 UI 出现后仍可见（通过 startup 队列 take/replay）；非法 JSON toast 正确显示；保护字段忽略 toast 正确显示"
    why_human: "startup 时序行为、watcher 触发（需 iCloud 或本地文件修改）以及 toast 文案随 i18n 切换，均需人工运行 pnpm tauri dev 验证"
---

# Phase 24: Claude Settings Overlay 端到端 验证报告

**Phase 目标：** Claude 全局配置 Overlay — 端到端交付：overlay 存储→编辑器 UI→深度合并引擎→patch 集成→watcher 自动 apply
**验证时间：** 2026-03-17
**状态：** gaps_found
**Re-verification：** No — 初次验证

---

## 目标达成情况

### 可观测真相（Observable Truths）

| # | 真相 | 状态 | 证据 |
|---|------|------|------|
| 1 | overlay 文件可被可靠写入：iCloud 可用优先写入 iCloud，否则自动降级本地 | VERIFIED | `get_icloud_config_dir()` 实现了 mobile_docs 存在性检测 + LocalFallback 降级，`write_claude_settings_overlay()` 使用 atomic_write |
| 2 | 后端可向 UI 返回 overlay 存放位置与具体路径 | VERIFIED | `OverlayStorageInfo { location, dir, file_path, sync_enabled }` 序列化并返回给前端 |
| 3 | 用户首次进入设置时会回填已保存 overlay（COVL-01/04） | FAILED | 后端字段 `content` vs 前端期待 `overlay_json`：字段名不匹配，state.overlay_json 为 undefined，编辑框永远为空 |
| 4 | 前端 JSON 校验 + 后端双重校验：非法 JSON 被拒绝（COVL-02） | VERIFIED | SettingsPage handleOverlaySave() 含 JSON.parse 校验；set_claude_settings_overlay 含 serde_json 校验；两层均拒绝非 object |
| 5 | overlay apply 采用深度合并（COVL-05/06） | VERIFIED | json_merge.rs 中 merge_with_null_delete 实现了 object 递归合并、array 整体替换、scalar 覆盖、null 删除 |
| 6 | 保护字段不可被 overlay 覆盖（COVL-07） | VERIFIED | patch_claude_json 末尾强制回写 ANTHROPIC_AUTH_TOKEN/BASE_URL；apply_overlay_without_provider 中 strip_protected_fields 先 strip 再合并 |
| 7 | 保存 overlay 后立即 apply（COVL-09，强一致） | VERIFIED | set_claude_settings_overlay 成功写入后立即调用 apply_claude_settings_overlay(ApplySource::Save)；apply 失败则整体返回 Err |
| 8 | 应用启动时 best-effort apply（COVL-10） | VERIFIED | lib.rs setup 阶段 spawn async 调用 apply_claude_settings_overlay(Startup)；失败只写日志 + 缓存通知，不阻断启动 |
| 9 | iCloud watcher 变更自动触发 apply（COVL-11） | VERIFIED | watcher/mod.rs 监听 config_dir；config_dir 下 claude-settings-overlay.json 变更触发 process_overlay_change → apply_claude_settings_overlay(Watcher) |
| 10 | startup 缓存通知回放可靠可见（COVL-12/04-plan 核心要求） | VERIFIED | ClaudeOverlayStartupNotificationQueue + take_claude_overlay_startup_notifications；useSyncListener 挂载后主动 take/replay |

**得分：** 9/10 真相通过

---

## 必需产物验证

### Plan 01 产物

| 产物 | 状态 | 详情 |
|------|------|------|
| `src-tauri/src/storage/icloud.rs` | VERIFIED | 包含 StorageLocation、OverlayStorageInfo、get_icloud_config_dir、get_claude_overlay_path、read/write_claude_settings_overlay |
| `src-tauri/src/commands/claude_settings.rs` | VERIFIED (partial) | 包含 get_claude_settings_overlay、set_claude_settings_overlay；但字段名 content 与前端约定不一致 |
| `src-tauri/src/commands/mod.rs` | VERIFIED | `pub mod claude_settings;` 存在 |
| `src-tauri/src/lib.rs` 注册 | VERIFIED | invoke_handler 注册了 get_claude_settings_overlay、set_claude_settings_overlay、apply_claude_settings_overlay_cmd、take_claude_overlay_startup_notifications |

### Plan 02 产物

| 产物 | 状态 | 详情 |
|------|------|------|
| `src/components/ui/textarea.tsx` | VERIFIED | Textarea 组件存在，支持 min-h、w-full、aria-invalid、className 透传 |
| `src/lib/tauri.ts` | PARTIAL | ClaudeSettingsOverlayStorage、ClaudeSettingsOverlayState（含 overlay_json 字段）、getClaudeSettingsOverlay、setClaudeSettingsOverlay、takeClaudeOverlayStartupNotifications 均存在；但 overlay_json 字段与后端 content 字段不匹配 |
| `src/components/settings/SettingsPage.tsx` | PARTIAL | Claude overlay 小节完整实现，含 useEffect 初始加载、Textarea 编辑、前端校验、保存流程、存储位置展示、保护字段说明；但读取 state.overlay_json 因字段名 bug 永远为 undefined |
| `src/i18n/locales/zh.json` | VERIFIED | settings.claudeOverlay.* 及 claudeOverlayApply.* 中文文案齐全 |
| `src/i18n/locales/en.json` | VERIFIED | settings.claudeOverlay.* 及 claudeOverlayApply.* 英文文案齐全 |

### Plan 03 产物

| 产物 | 状态 | 详情 |
|------|------|------|
| `src-tauri/src/adapter/json_merge.rs` | VERIFIED | PROTECTED_ENV_KEYS、StripResult、strip_protected_fields、merge_with_null_delete 全部实现，17 个单元测试存在 |
| `src-tauri/src/adapter/mod.rs` | VERIFIED | `pub mod json_merge;` 存在 |
| `src-tauri/src/adapter/claude.rs` | VERIFIED | ClaudeAdapter 含 overlay_path_override 字段，patch_claude_json 接入 strip + merge + 强制回写保护字段；6 个 overlay 集成测试存在 |

### Plan 04 产物

| 产物 | 状态 | 详情 |
|------|------|------|
| `src-tauri/src/commands/claude_settings.rs` — apply & startup queue | VERIFIED | apply_claude_settings_overlay、ClaudeOverlayStartupNotificationQueue、take_claude_overlay_startup_notifications 全部存在 |
| `src-tauri/src/lib.rs` startup queue 注册 | VERIFIED | `.manage(ClaudeOverlayStartupNotificationQueue::new())` 存在；setup 阶段 spawn startup apply |
| `src-tauri/src/watcher/mod.rs` | VERIFIED | start_file_watcher 同时监听 providers_dir + config_dir；overlay 文件变更触发 process_overlay_change |
| `src/lib/tauri.ts` — takeClaudeOverlayStartupNotifications | VERIFIED | ClaudeOverlayApplyNotification 类型与 takeClaudeOverlayStartupNotifications 均存在 |
| `src/hooks/useSyncListener.ts` | VERIFIED | 监听三类实时事件 + 挂载后 takeClaudeOverlayStartupNotifications() take/replay |

---

## 关键链路验证（Key Links）

| 来源 | 目标 | 方式 | 状态 | 详情 |
|------|------|------|------|------|
| `src-tauri/src/lib.rs` | `commands::claude_settings` | invoke_handler 注册 | VERIFIED | 四个 claude_settings 命令均注册 |
| `src-tauri/src/lib.rs` | `ClaudeOverlayStartupNotificationQueue` | .manage() | VERIFIED | 第 23 行：`.manage(commands::claude_settings::ClaudeOverlayStartupNotificationQueue::new())` |
| `src/components/settings/SettingsPage.tsx` | `src/lib/tauri.ts` | getClaudeSettingsOverlay/setClaudeSettingsOverlay 调用 | VERIFIED (call side) | 调用存在；但 getClaudeSettingsOverlay() 返回值的字段名 bug 导致数据未正确读取 |
| `src/components/settings/SettingsPage.tsx` | `src/i18n/locales/zh.json` | settings.claudeOverlay.* i18n key | VERIFIED | t("settings.claudeOverlay.title") 等均有对应 i18n key |
| `src-tauri/src/adapter/claude.rs` | `src-tauri/src/adapter/json_merge.rs` | patch_claude_json 内调用 merge_with_null_delete | VERIFIED | 第 227 行：`merge_with_null_delete(&mut root, &strip_result.overlay)?;` |
| `src-tauri/src/adapter/claude.rs` | `src-tauri/src/storage/icloud.rs` | read_overlay() 调用 read_claude_settings_overlay | VERIFIED | 第 70 行：`crate::storage::icloud::read_claude_settings_overlay()?` |
| `src-tauri/src/watcher/mod.rs` | `commands::claude_settings::apply_claude_settings_overlay` | overlay 文件变化触发 | VERIFIED | process_overlay_change 内调用 apply_claude_settings_overlay(ApplySource::Watcher) |
| `src/hooks/useSyncListener.ts` | `src/lib/tauri.ts` | takeClaudeOverlayStartupNotifications() | VERIFIED | 第 144 行：takeClaudeOverlayStartupNotifications().then(...) |

---

## 需求覆盖（Requirements Coverage）

| 需求 ID | 来源 Plan | 描述 | 状态 | 证据 |
|---------|-----------|------|------|------|
| COVL-01 | 24-02 | 用户可在 Settings → Advanced → Claude 小节编辑 JSON overlay | BLOCKED | UI 小节存在，Textarea 存在，但 overlay_json 字段名 bug 导致已保存内容无法回填 |
| COVL-02 | 24-02 | 保存前 JSON 校验（root 必须为 object）；不合法拒绝保存 | SATISFIED | SettingsPage handleOverlaySave() 含 JSON.parse + typeof 校验；set_claude_settings_overlay 含后端双重校验 |
| COVL-03 | 24-01 | overlay 持久化优先 iCloud，不可用时降级本地 | SATISFIED | get_icloud_config_dir() 实现 mobile_docs 存在性检测，降级到 ~/.cli-manager/config |
| COVL-04 | 24-01/02 | UI 可感知 overlay 存放位置（iCloud/本地降级）与是否跨设备同步 | PARTIAL | OverlayStorageInfo 返回正确；SettingsPage 展示存储位置/路径/sync_enabled；但 overlay_json 字段 bug 导致初始加载不完整（storageInfo 本身能加载，因为 storage 字段名无问题） |
| COVL-05 | 24-03 | overlay apply 深度合并：object 递归/array 替换/scalar 覆盖 | SATISFIED | merge_with_null_delete 实现并有 11 个单测验证 |
| COVL-06 | 24-03 | overlay 支持 null 删除字段 | SATISFIED | merge_with_null_delete 中 overlay_val.is_null() 分支删除 base 对应 key |
| COVL-07 | 24-03 | overlay 不得覆盖保护字段（ANTHROPIC_AUTH_TOKEN/BASE_URL） | SATISFIED | patch_claude_json 末尾强制回写；apply_overlay_without_provider strip 后合并 |
| COVL-08 | 24-02/04 | overlay 包含保护字段时忽略并 UI 提示 | SATISFIED | strip_protected_fields 返回 stripped_paths；deliver_notification 发送 ProtectedFieldsIgnored 通知；useSyncListener 展示 warning toast；SettingsPage 展示保护字段说明 |
| COVL-09 | 24-04 | 用户保存 overlay 后立即 apply（强一致） | SATISFIED | set_claude_settings_overlay 保存后立即调用 apply_claude_settings_overlay(Save)；apply 失败则整体返回 Err |
| COVL-10 | 24-04 | 应用启动时 best-effort apply；失败不阻断启动 | SATISFIED | lib.rs setup spawn async apply(Startup)；Err 只写日志 + 缓存队列 |
| COVL-11 | 24-04 | iCloud 同步导致 overlay 变更时，watcher 自动触发 apply | SATISFIED | watcher/mod.rs 监听 config_dir；overlay 文件变更触发 apply(Watcher) |
| COVL-12 | 24-04 | overlay 文件 JSON 不合法时，apply 返回错误并在 UI 显示 | SATISFIED | apply_claude_settings_overlay 校验 overlay JSON；通过 deliver_notification 发送 Failed 通知；useSyncListener 展示 error toast |
| COVL-13 | 24-03 | settings.json 不是合法 JSON 时，拒绝写入并返回可见错误 | SATISFIED | ClaudeAdapter::patch() 读取 settings.json 时做 serde_json::from_str 预校验，失败返回 Validation 错误 |

**孤立需求（Orphaned）：** 无 — COVL-14/15/16 属于 Phase 25，未分配到 Phase 24。

---

## 反模式扫描（Anti-Patterns）

| 文件 | 行 | 模式 | 严重程度 | 影响 |
|------|-----|------|---------|------|
| `src-tauri/src/commands/claude_settings.rs` | 91 | 字段名 `content` 与前端约定 `overlay_json` 不一致 | Blocker | overlay 编辑框初始加载永远为空；COVL-01 中的"已保存 overlay 回填"功能失效 |

未发现其他典型 stub 模式（TODO/FIXME、空实现、仅 console.log 的 handler）。

---

## 需要人工验证的项目

### 1. 字段名 bug 修复后的 overlay 读取回填

**测试：** 启动 `pnpm tauri dev`，进入 Settings → Advanced，先保存一个合法 overlay（如 `{ "env": { "FOO": "BAR" } }`），退出设置页，再进入设置页。
**期望：** 编辑框内应回填之前保存的 overlay 内容。
**人工验证原因：** 静态分析已确认字段名 bug；修复后需实际运行确认 Tauri 序列化/反序列化与前端的数据流是否打通。

### 2. overlay apply 三类触发点与 startup 缓存回放

**测试：** 按照 24-04-PLAN.md Task 3 操作步骤（保存即 apply、保护字段忽略 toast、startup 缓存回放、watcher 自动 apply、中英文切换）。
**期望：** 三条路径（save/startup/watcher）均可观测；startup toast 在应用 UI 出现后仍可见；中英文文案随切换。
**人工验证原因：** 时序行为（setup 早于 WebView listener）、iCloud 文件系统 watcher、toast UI 显示效果无法通过静态分析完全验证。

---

## Gap 总结

Phase 24 整体完成度较高，存储层、深度合并引擎、apply 链路、watcher、startup 缓存队列均正确实现且接线。

**唯一 Blocker Gap：** 后端 `GetClaudeSettingsOverlayResponse` 字段名为 `content`，而前端 `ClaudeSettingsOverlayState` 类型及 SettingsPage 均使用 `overlay_json`。由于 Tauri v2 不自动进行 snake_case → camelCase 转换，`state.overlay_json` 在运行时永远为 `undefined`。这导致：
- COVL-01 中"首次进入时回填已保存 overlay"的功能失效
- 用户无法看到已保存的 overlay 内容（尽管 overlay 文件本身确实已写入）

**修复方案（选一）：**
1. 在后端 `GetClaudeSettingsOverlayResponse` 中将 `pub content: Option<String>` 改名为 `pub overlay_json: Option<String>`，并同步修改构造处
2. 或在前端 `ClaudeSettingsOverlayState` 类型及 SettingsPage 中将 `overlay_json` 改为 `content`

其余 9/10 真相已通过验证，gap 修复后即可进入人工验证阶段。

---

_验证时间：2026-03-17_
_验证人：Claude (gsd-verifier)_
