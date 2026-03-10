# cc-switch + iCloud 同步异常：写入点与根因分析（记录）

> 目的：把本次对“cc-switch 使用 iCloud 同步客户端目录导致延迟/冲突/怪异状态”的结论固化下来，方便后续排查与制定改造方案。

## 1. 关键结论（一句话）

把 **live 配置目录**（如 `~/.claude` / `~/.codex` / `~/.gemini`）或 **cc-switch 自己的状态目录**（尤其 `~/.cc-switch/cc-switch.db`、`~/.cc-switch/settings.json`）直接放到 iCloud Drive 做“热同步”，会把 cc-switch 的“多点写入 + 多文件写入 + SQLite 启动维护写入”暴露到 iCloud 的 **最终一致（非事务、无跨设备锁）** 语义下，从而自然产生：同步延迟、半套配置、冲突副本、状态回跳。

---

## 2. cc-switch 到底什么时候会写文件？（按类型）

### 2.1 写 live 配置（各 CLI 真正在用的配置文件）

#### 触发点 1：切换 Provider（必写）
- IPC 命令：`src-tauri/src/commands/provider.rs:89-97`（`switch_provider`）
- 业务：`ProviderService::switch(...)`（内部会写对应 app 的 live config）

#### 触发点 2：后置同步（可写很多东西）
- IPC 命令：`src-tauri/src/commands/import_export.rs:62-76`（`sync_current_providers_live`）
- 实际行为：`src-tauri/src/services/provider/live.rs:830-864`
  - 对每个 `AppType` 写 provider 到 live
  - **随后同步 MCP**：`McpService::sync_all_enabled(state)?`（`live.rs:853-855`）
  - **随后同步 Skills**：对每个 app `SkillService::sync_to_app`（`live.rs:856-863`）

#### 触发点 3：导入 SQL 后自动后置同步（写 live + reload settings）
- `import_config_from_file`：`src-tauri/src/commands/import_export.rs:40-60`
- `run_post_import_sync`：`src-tauri/src/commands/sync_support.rs:10-14`
  - `ProviderService::sync_current_to_live(...)`
  - `settings::reload_settings()`

#### 触发点 4：修改目录覆盖后立即后置同步（前端触发）
- `src/hooks/useSettings.ts:361-379`
  - 当 `claudeConfigDir/codexConfigDir/geminiConfigDir/opencodeConfigDir` 发生变化时，调用 `syncCurrentProvidersLiveSafe()` → 后端 `sync_current_providers_live`。

#### 触发点 5：Additive mode（OpenCode/OpenClaw）新增/更新 provider 可能“总是写 live”
- `src-tauri/src/services/provider/mod.rs:167-203`（`add` 对 additive app 直接 `write_live_with_common_config`）

> 含义：用户主观上“一次保存/导入/改目录”，后端可能触发一次“全家桶写入”（provider + MCP + skills），写入面越大，iCloud 同步压力与冲突窗口越大。

---

### 2.2 写设备级设置（`~/.cc-switch/settings.json`）

#### 内容特点：它**不是 SSOT**，是“设备级偏好 + 设备级当前 provider”
- 字段：`current_provider_*`、`*_config_dir` 等（`src-tauri/src/settings.rs:215-242`）
- 路径固定：`~/.cc-switch/settings.json`（`src-tauri/src/settings.rs:319-326`）

#### 触发点：切换 provider 会写 `settings.json`
- `set_current_provider`：`src-tauri/src/settings.rs:582-598`（保存设备级 current provider）

#### 写入方式风险：**truncate + write（非 atomic rename）**
- `save_settings_file`：`src-tauri/src/settings.rs:405-439`
  - Unix 下 `OpenOptions(truncate(true))` 写入

> iCloud 风险：另一台设备可能同步到“刚 truncate 但还没写完”的短暂状态（空文件/半文件），导致解析失败 → 回落默认值 → UI/状态看起来“乱跳”。

---

### 2.3 写 SSOT（SQLite：`~/.cc-switch/cc-switch.db`）

#### 重要事实：即使用户不做任何操作，cc-switch 启动也可能写 DB
- 启动清理与空间回收：`src-tauri/src/database/mod.rs:138-151`
  - `cleanup_old_stream_check_logs(7)`
  - `rollup_and_prune(30)`
  - `PRAGMA incremental_vacuum;`
- 可能触发重建（`VACUUM;`）：`src-tauri/src/database/mod.rs:192-213`

> iCloud 风险：两台 Mac 只要都启动过 app，就会发生“维护性写入竞争”，冲突概率显著上升。

---

## 3. 为什么 iCloud 会导致“延迟/冲突/怪异状态”？（机制层）

### 3.1 iCloud 是最终一致同步，不是事务型复制
- 不提供跨设备文件锁
- 不保证“多文件更新”的一致性顺序
- 冲突时可能产生 “conflicted copy”

### 3.2 多文件配置天然容易出现“半套状态”
典型例子：
- Codex：`auth.json` + `config.toml`（且存在两阶段/回滚写入策略）
- Gemini：`.env` + `settings.json`
- 后置同步还会额外写 MCP/skills

iCloud 可能先同步其中一个文件到另一台设备，另一台读取到半套配置 → 行为异常。

### 3.3 `atomic_write` 本地原子，不等于 iCloud 跨设备事务
- `atomic_write`：`src-tauri/src/config.rs:183-238`
  - 先写 `*.tmp.<ts>` 再 `rename` 覆盖

本地很好，但 iCloud 远端呈现顺序不保证（可能先看到 tmp 文件、再看到 rename、或中间态被 watcher/读取者捕获）。

### 3.4 SQLite 放云盘同步目录是经典雷区
SQLite 正常依赖本地文件系统的锁与一致性语义（即使不显式设置 WAL）。在 iCloud 这种“同步层”之上会出现：
- 文件级冲突 → DB 冲突副本
- 同步到半文件 → 读取失败、回滚、数据看似丢失

### 3.5 `settings.json` 本意是“设备级可不同”，同步它会变成互相覆盖
- `get_effective_current_provider` 明确：设备级 current provider 优先（`src-tauri/src/settings.rs:600-633`）
- 一旦把 `settings.json` 同步到 iCloud，多设备会互相覆盖 current provider、目录覆盖等字段 → 状态回跳。

---

## 4. 最可能的根因组合（按概率从高到低）

1) **SQLite DB 在 iCloud**（或整个 `~/.cc-switch` 在 iCloud）
- 启动维护写入导致两台机器无操作也会竞争

2) **`~/.cc-switch/settings.json` 在 iCloud**
- truncate 写入导致远端可能拿到空/半文件

3) **CLI live 目录在 iCloud**
- 多文件不同步/顺序不一致导致另一台读到半套

4) **后置同步写入面过大**（provider + MCP + skills）
- iCloud 同步压力大、冲突窗口大

---

## 5. 最小折腾的改法（止损/改造方向）

### 方案 A（最稳、最省操作）：iCloud 只传“快照文件”，不要同步 live/SQLite
- 保持本地默认目录：`~/.claude` / `~/.codex` / `~/.gemini` / `~/.cc-switch`
- iCloud 仅存放导出的 `cc-switch-export-*.sql`（或将来 zip/json snapshot）
- 另一台：检测到新快照 → 导入 → 调用 `sync_current_providers_live`

### 方案 B（要“自动热同步”但仍稳）：iCloud 作为发布通道，本地仍是 SSOT
- 自动导出快照到 iCloud（单文件 + atomic rename）
- 另一台用文件监听（FSEvents）检测快照更新自动导入
- 导入后统一触发后置同步写 live

### 方案 C（坚持同步目录的止损版）：自动单写者租约 + 限制危险写入
- 写前自动抢 iCloud lock（TTL + machine id），抢不到进入只读
- 同时建议：
  - `settings.json` 改成 atomic_write（避免 truncate 半文件传播）
  - 禁止/降低启动 vacuum/cleanup（避免无操作写 DB）
  - 评估是否禁用“switch backfill live→DB”（避免用远端未收敛 live 覆盖 SSOT）

---

## 6. 相关代码索引（快速定位）

- 后置同步命令：`src-tauri/src/commands/import_export.rs:62-76`
- 导入后置同步 + reload settings：`src-tauri/src/commands/sync_support.rs:10-14`
- 后置同步实现（写 provider + MCP + skills）：`src-tauri/src/services/provider/live.rs:830-864`
- 设备级 current provider：`src-tauri/src/settings.rs:582-633`
- settings.json 写入（truncate）：`src-tauri/src/settings.rs:405-439`
- SQLite 启动维护写入：`src-tauri/src/database/mod.rs:138-151`、`192-213`
- atomic_write（tmp + rename）：`src-tauri/src/config.rs:183-238`
