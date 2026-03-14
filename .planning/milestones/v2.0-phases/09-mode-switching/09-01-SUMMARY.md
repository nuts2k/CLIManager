---
phase: 09-mode-switching
plan: 01
subsystem: proxy
tags: [tauri-commands, mode-switching, proxy-settings, local-settings, adapter-patch]

# Dependency graph
requires:
  - phase: 08-01
    provides: "ProxyServer 单端口代理引擎 + ProxyState + UpstreamTarget + ProxyError"
  - phase: 08-02
    provides: "ProxyService 多端口管理器 + Tauri State 注入模式"
provides:
  - "ProxySettings + ProxyTakeover: 代理开关状态和接管标志持久化结构体"
  - "PROXY_PORT_CLAUDE=15800, PROXY_PORT_CODEX=15801 端口常量"
  - "proxy_enable: patch CLI 配置为 localhost + PROXY_MANAGED 并启动代理"
  - "proxy_disable: 还原 CLI 配置为真实凭据并停止代理"
  - "proxy_set_global: 全局开关联动所有已启用 CLI 的代理启停"
  - "proxy_get_mode_status: 返回全局开关和每个 CLI 的代理状态"
  - "set_active_provider 代理模式分支: 代理中跳过 adapter.patch() 改为 update_upstream()"
  - "ProxyModeStatus / CliProxyStatus: 模式状态返回类型"
affects: [09-02, 10-realtime-ui]

# Tech tracking
tech-stack:
  added: []
  patterns: [同步 adapter 操作在 block 内完成后再执行 async 代理操作（Send 安全）, _proxy_enable_in/_proxy_disable_in 内部函数变体支持测试注入]

key-files:
  created: []
  modified:
    - src-tauri/src/storage/local.rs
    - src-tauri/src/proxy/mod.rs
    - src-tauri/src/commands/proxy.rs
    - src-tauri/src/commands/provider.rs
    - src-tauri/src/lib.rs

key-decisions:
  - "adapter 参数使用 Box<dyn CliAdapter + Send> 确保 async 函数 future 满足 Send bound"
  - "proxy_enable 失败时回滚 CLI 配置为真实凭据（不留半成品状态）"
  - "proxy_disable 停止代理为 best-effort（不影响 CLI 配置还原）"
  - "set_active_provider 代理模式判断在 Tauri 命令层，非 _in 函数层"
  - "proxy_set_global 关闭时从 proxy_takeover.cli_ids 获取需关闭的 CLI 列表"

patterns-established:
  - "模式切换命令模式: 同步 adapter 操作在 block 内完成 -> drop adapter -> async 代理操作"
  - "代理专用 Provider 构造: make_proxy_provider() 临时构造 api_key=PROXY_MANAGED + localhost base_url"
  - "get_adapter_for_cli_pub: 跨模块公开 adapter 获取的桥接函数"

requirements-completed: [MODE-01, MODE-02, MODE-03, MODE-04, LIVE-04]

# Metrics
duration: 8min
completed: 2026-03-13
---

# Phase 9 Plan 1: 模式切换后端核心 Summary

**代理开关持久化 + 四个模式切换 Tauri 命令 (enable/disable/set_global/get_mode_status) + set_active_provider 代理模式感知改造**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-13T14:43:29Z
- **Completed:** 2026-03-13T14:51:00Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- ProxySettings + ProxyTakeover 结构体实现代理开关和接管标志持久化到 local.json（向后兼容）
- 端口常量 PROXY_PORT_CLAUDE=15800, PROXY_PORT_CODEX=15801 和 proxy_port_for_cli() 辅助函数
- proxy_enable/disable/set_global/get_mode_status 四个模式切换 Tauri 命令完整实现
- set_active_provider 改为 async 命令，代理模式下跳过 adapter.patch() 改用 update_upstream()
- proxy_enable 失败时自动回滚 CLI 配置为真实凭据
- 186 个测试全部通过，cargo build 无错误

## Task Commits

Each task was committed atomically:

1. **Task 1: 扩展 LocalSettings + 端口常量 + 类型定义** - `b01b3cc` (feat)
2. **Task 2: 模式切换命令 + set_active_provider 代理模式改造** - `dc70941` (feat)

## Files Created/Modified
- `src-tauri/src/storage/local.rs` - 新增 ProxySettings/ProxyTakeover 结构体，LocalSettings 扩展 proxy + proxy_takeover 字段
- `src-tauri/src/proxy/mod.rs` - 新增 PROXY_PORT_CLAUDE/PROXY_PORT_CODEX 端口常量 + proxy_port_for_cli() 函数
- `src-tauri/src/commands/proxy.rs` - 新增 proxy_enable/disable/set_global/get_mode_status 四个 Tauri 命令 + ProxyModeStatus/CliProxyStatus 类型 + _proxy_enable_in/_proxy_disable_in 内部函数
- `src-tauri/src/commands/provider.rs` - set_active_provider 改为 async + 代理模式判断分支 + get_adapter_for_cli_pub 桥接函数
- `src-tauri/src/lib.rs` - 注册四个新命令

## Decisions Made
- adapter 参数使用 `Box<dyn CliAdapter + Send>` 确保 Tauri async 命令的 future 满足 Send bound（CliAdapter 的具体实现 ClaudeAdapter/CodexAdapter 均为 Send）
- proxy_enable 失败时回滚 CLI 配置为真实凭据，确保不留半成品状态
- proxy_disable 停止代理为 best-effort，不影响 CLI 配置还原结果
- set_active_provider 代理模式判断放在 Tauri 命令层（非 _set_active_provider_in 函数层），保持 tray.rs 直接调用 _in 函数不受影响
- proxy_set_global 关闭时从 proxy_takeover.cli_ids 获取需关闭的 CLI 列表（而非 cli_enabled），确保只关闭实际被接管的

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 添加 tauri::Emitter import**
- **Found during:** Task 2（proxy_enable 编译）
- **Issue:** Tauri 2.x 的 `AppHandle.emit()` 方法需要 `Emitter` trait 在 scope 中
- **Fix:** 在 commands/proxy.rs 添加 `use tauri::Emitter;`
- **Files modified:** src-tauri/src/commands/proxy.rs
- **Committed in:** dc70941

**2. [Rule 1 - Bug] 修复 dyn CliAdapter 不满足 Send 导致 async future 不 Send**
- **Found during:** Task 2（proxy_enable 编译）
- **Issue:** `Box<dyn CliAdapter>` 不是 Send，在 async 函数中跨 `.await` 持有导致 Tauri 命令注册失败
- **Fix:** 将 _proxy_enable_in/_proxy_disable_in 的 adapter 参数类型改为 `Option<Box<dyn CliAdapter + Send>>`，并将所有同步 adapter 操作放在独立 block 内确保在 `.await` 前 drop
- **Files modified:** src-tauri/src/commands/proxy.rs
- **Committed in:** dc70941

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** 两个修复都是编译兼容性问题，不影响功能设计。

## Issues Encountered
None

## User Setup Required
None - 无需外部服务配置。

## Next Phase Readiness
- 模式切换后端核心完成，Phase 9 Plan 02 可实现崩溃恢复和退出清理
- Phase 10 前端可通过 Tauri invoke 调用 proxy_enable/proxy_disable/proxy_set_global/proxy_get_mode_status
- proxy_get_mode_status 提供完整的 CLI 代理状态信息供 UI 渲染

---
*Phase: 09-mode-switching*
*Completed: 2026-03-13*

## Self-Check: PASSED

All 5 modified files verified. Both task commits (b01b3cc, dc70941) confirmed in git log. SUMMARY.md exists.
