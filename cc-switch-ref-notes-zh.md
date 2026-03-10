# CC Switch 项目理解文档（用于“重构精简版”参考）

> 参考仓库：https://github.com/farion1231/cc-switch
>
> 本文目标：把 cc-switch 的**功能、技术架构、目录结构、关键数据与关键流程**整理成一份可复用的“项目说明/拆解笔记”，便于后续做“重构精简版”时快速取舍。

---

## 1. 一句话概览

CC Switch 是一个基于 **Tauri 2（Rust 后端 + WebView 前端）** 的桌面应用，用 **SQLite 作为 SSOT（单一事实源）** 管理多种 AI CLI（Claude Code / Codex / Gemini / OpenCode / OpenClaw）的配置与扩展（Providers/MCP/Prompts/Skills），并在切换时把选择**同步回各 CLI 的 live 配置文件**。

- SSOT：`~/.cc-switch/cc-switch.db`（README_ZH.md:306）
- 设备级设置：`~/.cc-switch/settings.json`（README_ZH.md:194）

---

## 2. 产品功能域（按模块）

### 2.1 Providers（供应商配置管理）

核心能力：
- 供应商（Provider）增删改查
- 一键切换（写入各 CLI 的 live config）
- 编辑当前激活供应商时，支持从 live 回填（README_ZH.md:308）
- 排序（sortIndex）、备注、图标、分类（见 `src/types.ts`）

前端调用入口：`src/lib/api/providers.ts`
- `invoke("get_providers") / invoke("switch_provider")` 等（src/lib/api/providers.ts:24-55）

后端命令注册：`src-tauri/src/lib.rs:860+`
- `commands::get_providers`、`commands::switch_provider`、`commands::update_providers_sort_order` 等（src-tauri/src/lib.rs:860-942）

### 2.2 MCP（统一 MCP 服务器管理）

能力：
- MCP server 统一数据结构管理（入库）
- 按应用启用/禁用，并同步到不同 CLI 的 MCP 配置格式

数据库表：`mcp_servers`（src-tauri/src/database/schema.rs:56-65）

后端命令：
- `commands::get_mcp_servers / upsert_mcp_server / toggle_mcp_app / import_mcp_from_apps`（src-tauri/src/lib.rs:918-924）

### 2.3 Prompts（提示词文件管理）

能力：
- Prompts 入库、启用/禁用、导入本地文件

数据库表：`prompts`（src-tauri/src/database/schema.rs:67-73）

后端命令：`commands::get_prompts / upsert_prompt / enable_prompt ...`（src-tauri/src/lib.rs:924-931）

### 2.4 Skills（统一技能管理，v3.10+）

能力：
- Skills 以 SSOT 目录为准（`~/.cc-switch/skills/`），并按应用同步（symlink/copy）
- 支持从 GitHub/ZIP 安装、扫描未管理技能、从各应用导入

数据库表：`skills`、`skill_repos`（src-tauri/src/database/schema.rs:74-104）

后端命令：`get_installed_skills / install_skill_unified / toggle_skill_app ...`（src-tauri/src/lib.rs:969-987）

### 2.5 Proxy / Failover / Usage（可裁剪的大模块）

能力大致包括：
- 本地代理服务、live takeover（接管 live config）
- 故障转移队列、熔断器配置
- 请求日志、用量统计、价格表维护、stream health check

数据库表（部分）：
- `proxy_config`（src-tauri/src/database/schema.rs:112-160）
- `proxy_request_logs`（src-tauri/src/database/schema.rs:172-205）
- `model_pricing`（src-tauri/src/database/schema.rs:206-216）
- `proxy_live_backup`（src-tauri/src/database/schema.rs:235-243）
- `usage_daily_rollups`（src-tauri/src/database/schema.rs:244-263）

后端命令：见 `src-tauri/src/lib.rs:991-1040`（Proxy + Failover + Usage + Stream check）

### 2.6 Session Manager（会话管理器）

目标与范围详见 PRD：`session-manager.md`（session-manager.md:1-27）。
- v1 目标：扫描并展示 Codex / Claude Code 本地会话记录，支持复制恢复命令/目录，macOS 可一键终端恢复。

后端命令入口：`src-tauri/src/lib.rs:1040-1045`（list/get/delete/launch session terminal）。

### 2.7 Deep Link（ccswitch:// 导入）

能力：
- 通过自定义 URL scheme `ccswitch://...` 解析导入 provider/mcp/prompt/skill 等

后端：`handle_deeplink_url`（src-tauri/src/lib.rs:93-152）
- 并通过 `app.emit("deeplink-import", ...)` 通知前端（src-tauri/src/lib.rs:121-125）

### 2.8 Tray（托盘快速切换）

能力：
- 托盘菜单展示当前 providers（按 app 分区），点击触发切换
- 支持 silent startup / minimize-to-tray 行为

后端注册命令：`update_tray_menu`（src-tauri/src/lib.rs:154-174）

---

## 3. 技术栈与构建形态

### 3.1 前端
- React 18 + TypeScript
- Vite（root 在 `src/`）
  - `root: "src"`、`outDir: "../dist"`（cc-switch/vite.config.ts:7-19）
  - dev server `port: 3000`（cc-switch/vite.config.ts:20-23）
- TailwindCSS（shadcn/ui 风格变量）
  - `content: ["./src/index.html", "./src/**/*.{js,ts,jsx,tsx}"]`（cc-switch/tailwind.config.cjs:3）
- TanStack React Query（前端数据同步/缓存）
  - Provider 入口 `QueryClientProvider`（src/main.tsx:91-99）

### 3.2 后端
- Rust + Tauri 2
- SQLite（rusqlite，连接用 Mutex 包裹）
  - `pub struct Database { conn: Mutex<Connection> }`（src-tauri/src/database/mod.rs:71-73）
- update_hook 用于触发 WebDAV 自动同步
  - `conn.update_hook(Some(... notify_db_changed(table)))`（src-tauri/src/database/mod.rs:75-84）

---

## 4. 核心架构：分层与 SSOT

README_ZH 的架构图将后端分为：Commands → Services → Models/Config（README_ZH.md:295-301）。

### 4.1 SSOT（Single Source of Truth）

- SSOT 数据库：`~/.cc-switch/cc-switch.db`（README_ZH.md:306；src-tauri/src/database/mod.rs:89-92）
- 设备级 UI/本机偏好：`~/.cc-switch/settings.json`（README_ZH.md:194）

### 4.2 “双向同步”

- 切换 provider 时，把 SSOT 中的 provider 配置写到目标 CLI live 文件
- 编辑当前 provider 时，可从 live 文件回填（README_ZH.md:308）

### 4.3 原子写（避免配置损坏）

统一实现：`atomic_write(path, data)`（src-tauri/src/config.rs:183-239）
- 写入 `file.tmp.<ts>`，再 rename 替换
- Windows 目标存在时先 remove 再 rename（src-tauri/src/config.rs:219-229）

### 4.4 后端命令面（IPC API Surface）

Tauri 命令集中注册于：`src-tauri/src/lib.rs:860-1096`。

可把命令大致按领域分组：
- Providers：get/add/update/delete/switch/sort/live 回填
- MCP：统一管理 + 从各 app 导入/同步
- Prompts：CRUD + 启用 + 从文件导入
- Skills：统一技能管理 + legacy 兼容 API
- Proxy/Failover/Usage/StreamCheck：代理、日志、统计、健康检查
- Session manager：会话列表/详情/恢复
- DeepLink：解析与导入
- Settings：设备级设置、目录覆盖、终端偏好

---

## 5. 数据模型（SQLite Schema 速览）

### 5.1 Providers
表：`providers`（src-tauri/src/database/schema.rs:20-36）
- 主键：`(id, app_type)`
- `settings_config`：JSON 字符串（各 CLI 的配置结构不同，统一当作 JSON blob）
- `meta`：扩展元数据（JSON，默认 `{}`）
- `is_current`：当前激活

前端类型：`src/types.ts`
- `Provider.settingsConfig: Record<string, any>`（src/types.ts:14-15）
- `ProviderMeta` 承载 usage_script、proxyConfig、apiFormat 等扩展字段（src/types.ts:125+）

### 5.2 MCP / Prompts / Skills
- `mcp_servers`（src-tauri/src/database/schema.rs:56-65）
- `prompts`（src-tauri/src/database/schema.rs:67-73）
- `skills`（src-tauri/src/database/schema.rs:74-93）

### 5.3 Proxy & Usage（重模块）
- `proxy_config`（src-tauri/src/database/schema.rs:112-160）
- `proxy_request_logs`（src-tauri/src/database/schema.rs:172-205）
- `usage_daily_rollups`（src-tauri/src/database/schema.rs:244-263）

---

## 6. 各 CLI 的 live 配置落点（后端路径与文件）

cc-switch 对不同 CLI 的“配置文件格式/落点”做了分别适配：

### 6.1 Claude Code
- 配置目录：`~/.claude/`（可被 Settings 覆盖，src-tauri/src/config.rs:35-42）
- 主配置：`settings.json`（兼容旧 `claude.json`，src-tauri/src/config.rs:72-86）
- MCP：默认 `~/.claude.json`（src-tauri/src/config.rs:44-70）

### 6.2 Codex
- 配置目录：`~/.codex/`（src-tauri/src/codex_config.rs:13-20）
- `auth.json` + `config.toml`
- 写入采用“两阶段 + 回滚”策略：`write_codex_live_atomic`（src-tauri/src/codex_config.rs:62-109）

### 6.3 Gemini CLI
- 配置目录：`~/.gemini/`（src-tauri/src/gemini_config.rs:8-15）
- `.env` 为主要载体，提供宽松/严格解析（src-tauri/src/gemini_config.rs:22-123）
- 写入时设置 Unix 权限（目录 700、文件 600）（src-tauri/src/gemini_config.rs:159-188）

### 6.4 OpenCode
- 配置目录：`~/.config/opencode/opencode.json`（src-tauri/src/opencode_config.rs:14-21）
- “累加模式”：provider 是一个 map，多个 provider 同存（src-tauri/src/opencode_config.rs:49-83）

### 6.5 OpenClaw
- 配置目录：`~/.openclaw/openclaw.json`（JSON5）
  - 路径函数：`get_openclaw_config_path`（src-tauri/src/openclaw_config.rs:46-51）
  - 解析：`json5::from_str`（src-tauri/src/openclaw_config.rs:199-211）

---

## 7. 前端启动与“配置加载失败”处理

入口：`src/main.tsx`

关键点：
- 监听后端事件 `configLoadError` 并弹系统对话框 + 强制退出（src/main.tsx:63-67；60-61）
- 同时在启动早期主动 `invoke("get_init_error")` 拉取初始化错误，避免事件竞态（src/main.tsx:73-87）

这说明后端初始化（特别是 DB/schema/migration）失败时，应用不尝试“带病运行”，而是明确阻断。

---

## 8. 目录结构（以“理解/改造”为目的的视图）

### 8.1 前端 `src/`
- `src/App.tsx`：主页面聚合（功能入口很集中）
- `src/components/*`：按业务域分目录：providers/mcp/prompts/skills/proxy/sessions/settings/universal...
- `src/lib/api/*`：所有 invoke API 封装（前端到后端的契约入口）
- `src/lib/query/*`：React Query 的 queries/mutations 与 queryClient
- `src/types.ts`：前端数据契约（Provider/Mcp/UniversalProvider/Settings 等）

### 8.2 后端 `src-tauri/src/`
- `commands/`：Tauri command 层（对前端暴露的 API）
- `services/`：业务逻辑（provider/mcp/skill/proxy/…）
- `database/`：SQLite 初始化、schema、DAO、backup、migration
- `*_config.rs`：不同 CLI live config 的路径与读写适配（codex/gemini/opencode/openclaw...）
- `tray.rs`：托盘菜单
- `session_manager/`：会话管理器实现

---

## 9. 做“重构精简版”的最小闭环建议（可直接作为拆功能清单）

下面给出一个**最小可用闭环（MVP）**，以及每个功能块的“裁剪收益/代价”。

### 9.1 MVP（建议先保留）

1) **Providers SSOT + 切换写 live**
- 必需：providers 表、基本 CRUD、switch 命令
- 必需：原子写（atomic_write）
- 建议先仅支持 1~2 个 CLI（例如 Claude Code + Codex 或 Claude Code + Gemini），降低适配面

2) **基础设置（设备级）**
- 配置目录覆盖（例如 claudeConfigDir/codexConfigDir）
- UI 语言/主题（非核心也可简化）

3) （可选但体验提升大）托盘快速切换
- 保留 `update_tray_menu` + 基本 tray 菜单即可（src-tauri/src/lib.rs:154-174）

### 9.2 明显可裁剪（建议第二阶段再做）

- Proxy / Failover / Usage / Stream check：体量最大（命令、表、UI 都多），可先去掉
- WebDAV sync / auto sync：涉及 update_hook 与网络同步策略，可后置
- Session Manager：独立价值高，但和“切换配置”主线弱耦合，可作为单独里程碑
- Deep Link：对传播/导入方便，但不是 MVP 必需
- Universal Provider（跨应用共享 provider）：属于“高级抽象层”，可后置

### 9.3 “精简版”可能的架构收缩点

- 后端命令面：从 `src-tauri/src/lib.rs:860+` 的大量命令中，只保留 provider + settings + 必要的 path/status
- 数据库：只保留 `providers` + `settings` 两张表即可起步
- 前端：只保留 provider 列表 + 编辑弹窗 + 切换按钮（React Query 可以保留也可以先不用）

---

## 10. 快速定位关键代码（索引）

- 前端启动与错误处理：`src/main.tsx:63-103`
- Vite/Tailwind：`vite.config.ts`、`tailwind.config.cjs`
- 后端命令总表：`src-tauri/src/lib.rs:860-1096`
- SSOT 数据库初始化：`src-tauri/src/database/mod.rs:86-154`
- Schema & 表定义：`src-tauri/src/database/schema.rs:16-330`
- 原子写实现：`src-tauri/src/config.rs:183-239`
- Codex 双文件原子写 + 回滚：`src-tauri/src/codex_config.rs:62-109`
- Gemini .env 解析与权限：`src-tauri/src/gemini_config.rs:22-190`
- OpenCode config 路径与 provider map：`src-tauri/src/opencode_config.rs:9-83`
- OpenClaw JSON5 读写：`src-tauri/src/openclaw_config.rs:199-211`
- Session Manager PRD：`session-manager.md`

---

## 11. 备注：本仓库当前文档体系

- `docs/user-manual/*`：多语言用户手册
- `docs/release-notes/*`：版本说明
- `docs/proxy-guide-zh.md`：代理模式专用指南

本文是工程侧“项目拆解笔记”，更偏向开发/重构用途。
