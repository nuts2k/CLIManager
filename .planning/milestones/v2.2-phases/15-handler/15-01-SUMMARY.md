---
phase: 15-handler
plan: "01"
subsystem: proxy
tags: [rust, upstream-target, model-mapping, proxy, provider]

# Dependency graph
requires:
  - phase: 14-data-model-translate-core
    provides: "TranslateRequest/Response/Stream 类型定义，模型映射所需的数据模型基础"
provides:
  - "UpstreamTarget 结构体扩展：upstream_model + upstream_model_map 两个 Option 字段"
  - "build_upstream_target_from_provider 直接保留完整 provider.base_url（不 strip path）"
  - "所有 16 个 UpstreamTarget 构造点编译通过并携带映射字段"
affects:
  - "15-handler/15-02: handler 协议路由分支可从 UpstreamTarget 读取映射数据"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Provider → UpstreamTarget 透传模型映射字段（upstream_model / upstream_model_map）"
    - "build_upstream_target_from_provider 直接使用 provider.base_url，保留完整路径供后续 URL 拼接"
    - "build_upstream_target（用户手动输入路径）继续调用 normalize_origin_base_url 进行严格校验"

key-files:
  created: []
  modified:
    - src-tauri/src/proxy/state.rs
    - src-tauri/src/commands/proxy.rs
    - src-tauri/src/commands/provider.rs
    - src-tauri/src/watcher/mod.rs
    - src-tauri/src/proxy/mod.rs
    - src-tauri/src/proxy/server.rs

key-decisions:
  - "build_upstream_target_from_provider 直接保留 provider.base_url，不再调用 extract_origin_base_url——Provider 的 base_url 已在创建时校验，handler 层需要完整路径（如 /v1/chat/completions）供 build_proxy_endpoint_url 正确拼接"
  - "commands/provider.rs 和 watcher/mod.rs 中的 UpstreamTarget 构造同样改为直接保留 base_url，确保联动更新时不意外丢失路径"
  - "移除 commands/proxy.rs 和 commands/provider.rs 中已无使用的 extract_origin_base_url import，避免编译警告"

patterns-established:
  - "Provider 场景的 UpstreamTarget 构造：直接使用 provider.base_url.clone()，透传 upstream_model / upstream_model_map"
  - "用户手动输入场景的 UpstreamTarget 构造：继续调用 normalize_origin_base_url 进行严格 origin-only 校验"

requirements-completed: [ROUT-02, MODL-03]

# Metrics
duration: 8min
completed: 2026-03-14
---

# Phase 15 Plan 01: UpstreamTarget 扩展 + 16 个构造点更新 Summary

**UpstreamTarget 扩展为 5 字段结构体，携带 upstream_model / upstream_model_map 映射数据，build_upstream_target_from_provider 改为保留完整 base_url，全部 16 个构造点编译通过**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-14T14:13:00Z
- **Completed:** 2026-03-14T14:21:34Z
- **Tasks:** 1
- **Files modified:** 6

## Accomplishments

- UpstreamTarget 结构体新增 `upstream_model: Option<String>` 和 `upstream_model_map: Option<HashMap<String, String>>` 两个字段
- `build_upstream_target_from_provider` 关键变更：不再调用 `extract_origin_base_url` strip path，直接保留 `provider.base_url`，并透传映射字段
- `commands/provider.rs`（2 处）和 `watcher/mod.rs`（1 处）联动构造同样改为保留完整 base_url 并携带映射字段
- 测试 `test_build_upstream_target_from_provider_strips_legacy_path` 更名为 `test_build_upstream_target_from_provider_preserves_base_url`，断言改为验证 base_url 被保留而非 strip
- 294 个测试通过，仅已知 UX-01 端口冲突测试除外（与本次变更无关）

## Task Commits

每个任务原子提交：

1. **Task 1: 扩展 UpstreamTarget + 更新全部构造点** - `df37128` (feat)

**计划元数据提交：** 待 final commit

## Files Created/Modified

- `/Users/kelin/Workspace/CLIManager/src-tauri/src/proxy/state.rs` - UpstreamTarget 新增 upstream_model + upstream_model_map 字段，测试中 make_target 和内联构造更新
- `/Users/kelin/Workspace/CLIManager/src-tauri/src/commands/proxy.rs` - build_upstream_target_from_provider 保留 base_url + 透传映射字段；测试重命名和断言更新；移除无用 import
- `/Users/kelin/Workspace/CLIManager/src-tauri/src/commands/provider.rs` - update_provider 和 _set_active_provider_in_proxy_mode 两处构造更新；移除无用 import
- `/Users/kelin/Workspace/CLIManager/src-tauri/src/watcher/mod.rs` - iCloud 联动处构造更新，移除不必要的 extract_origin_base_url 调用
- `/Users/kelin/Workspace/CLIManager/src-tauri/src/proxy/mod.rs` - 测试辅助 make_upstream 新字段填 None
- `/Users/kelin/Workspace/CLIManager/src-tauri/src/proxy/server.rs` - 四处测试内联构造新字段填 None

## Decisions Made

- `build_upstream_target_from_provider` 直接保留 `provider.base_url`，不再调用 `extract_origin_base_url`——Provider 的 base_url 已在创建时校验过，handler 层需要完整路径（如 `/v1/chat/completions`）供后续 URL 拼接
- `commands/provider.rs` 和 `watcher/mod.rs` 中的联动构造同样改为直接保留 base_url，确保 provider 更新事件不意外丢失路径信息
- `build_upstream_target`（用户手动输入场景）继续调用 `normalize_origin_base_url` 进行严格 origin-only 校验——用户输入路径不可信

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] 移除两个文件中已无使用的 extract_origin_base_url import**
- **Found during:** Task 1 (更新构造点后检查 import)
- **Issue:** `commands/proxy.rs` 和 `commands/provider.rs` 的 import 中保留了 `extract_origin_base_url`，但代码中已无使用点，会产生 unused import 编译警告
- **Fix:** 从两个文件的 import 语句中移除 `extract_origin_base_url`
- **Files modified:** src-tauri/src/commands/proxy.rs, src-tauri/src/commands/provider.rs
- **Verification:** cargo test 通过，无 unused import 警告
- **Committed in:** df37128 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 - 清理无用 import)
**Impact on plan:** 纯清理，无功能影响，避免了编译器警告。

## Issues Encountered

无。

## Next Phase Readiness

- UpstreamTarget 已携带模型映射数据，Plan 02（handler 协议路由分支）可直接从 UpstreamTarget 读取 upstream_model / upstream_model_map
- base_url 保留完整路径，build_proxy_endpoint_url 可正确拼接 endpoint URL
- 全套测试 294 passed，代码库健康，无阻碍因素

## Self-Check: PASSED

- FOUND: `.planning/phases/15-handler/15-01-SUMMARY.md`
- FOUND: commit `df37128`

---
*Phase: 15-handler*
*Completed: 2026-03-14*
