---
phase: 24-claude-settings-overlay-end-to-end
plan: "03"
subsystem: adapter
tags: [rust, serde_json, deep-merge, overlay, claude-settings]

requires:
  - phase: 24-01
    provides: read_claude_settings_overlay() 存储接口

provides:
  - json_merge 纯函数模块（merge_with_null_delete + strip_protected_fields）
  - ClaudeAdapter patch 已强制接入 overlay 存储读取、深度合并与保护字段优先机制

affects:
  - 24-04（apply 命令复用 patch 流程）
  - 25（测试覆盖 Phase 扩展 json_merge 测试）

tech-stack:
  added: []
  patterns:
    - "overlay_path_override 注入模式：adapter 新增可选路径字段，测试时注入临时路径，生产时调全局存储"
    - "执行顺序固定：ensure env -> strip 保护字段 -> deep merge -> 强制回写保护字段"
    - "null 删除语义：overlay 中的 null 值从 base 中删除对应 key"

key-files:
  created:
    - src-tauri/src/adapter/json_merge.rs
  modified:
    - src-tauri/src/adapter/mod.rs
    - src-tauri/src/adapter/claude.rs

key-decisions:
  - "overlay_path_override 注入模式：不引入 trait 抽象，直接在结构体加可选字段，保持测试简洁"
  - "patch_claude_json 最终始终回写保护字段：无论 overlay 是否存在，保护字段优先级恒由 provider 决定"
  - "strip_protected_fields 返回 stripped_paths 供未来 UI 提示使用（本 plan 暂不消费）"

patterns-established:
  - "adapter 内 overlay 合并：parse -> strip -> merge -> 强制回写，顺序不可调换"
  - "json_merge 模块保持纯函数，无 I/O，便于 Phase 25 进一步测试扩展"

requirements-completed: [COVL-05, COVL-06, COVL-07, COVL-13]

duration: 4min
completed: 2026-03-17
---

# Phase 24 Plan 03: json_merge 深度合并引擎 + ClaudeAdapter overlay 集成 Summary

**serde_json 深度合并引擎（object 递归合并、array 整体替换、null 删除）与保护字段剥离，强制接入 ClaudeAdapter patch 流程，让 overlay 在 surgical patch 中生效同时永不能覆盖 ANTHROPIC_AUTH_TOKEN / ANTHROPIC_BASE_URL**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-17T00:22:50Z
- **Completed:** 2026-03-17T00:26:30Z
- **Tasks:** 2
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments

- 新增 `adapter/json_merge.rs`：实现 `merge_with_null_delete`（深度合并含 null 删除）与 `strip_protected_fields`（保护字段剥离），17 个单元测试全部通过
- 升级 `ClaudeAdapter::patch()`：强制读取 overlay 存储、strip + merge + 最终回写保护字段，6 个 overlay 集成测试全部通过
- 全套 398 个单元测试全部通过，端到端链路接通

## Task Commits

每个 task 均原子提交：

1. **Task 1: 新增 adapter/json_merge.rs** - `884db66` (feat)
2. **Task 2: 升级 ClaudeAdapter patch** - `5629237` (feat)

## Files Created/Modified

- `src-tauri/src/adapter/json_merge.rs` - 深度合并引擎纯函数模块（merge_with_null_delete + strip_protected_fields + PROTECTED_ENV_KEYS）
- `src-tauri/src/adapter/mod.rs` - 新增 `pub mod json_merge;` 导出
- `src-tauri/src/adapter/claude.rs` - 升级 patch 接入 overlay 合并，新增 overlay_path_override 注入字段

## Decisions Made

- **overlay_path_override 注入模式**：不引入额外 trait 抽象，直接在 `ClaudeAdapter` 结构体加 `Option<PathBuf>` 字段。测试用 `new_with_paths_and_overlay()` 注入，生产用 `new()` / `new_with_paths()` 走全局存储。方案最小化改动且不破坏现有 API。
- **保护字段最终强制回写**：无论 overlay 是否存在，`patch_claude_json` 末尾始终强制写入 `ANTHROPIC_AUTH_TOKEN` / `ANTHROPIC_BASE_URL`，保证 provider 优先级无法被绕过。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] 为 StripResult 补充 #[derive(Debug)]**
- **Found during:** Task 1（运行测试时编译报错）
- **Issue:** `unwrap_err()` 要求 `T: Debug`，StripResult 未实现 Debug
- **Fix:** 在 `StripResult` struct 上添加 `#[derive(Debug)]`
- **Files modified:** src-tauri/src/adapter/json_merge.rs
- **Verification:** 编译通过，测试通过
- **Committed in:** `884db66` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 - Missing Critical)
**Impact on plan:** 必要修复，无超出范围。

## Issues Encountered

无。测试驱动开发流程顺畅，仅有上述一处编译错误即时修复。

## Next Phase Readiness

- Plan 24-04（apply 命令）可直接调用 `ClaudeAdapter::patch()`，overlay 合并已在流程中
- Plan 25（测试覆盖）可扩展 `json_merge` 测试，模块为纯函数设计便于扩展
- `strip_protected_fields` 返回的 `stripped_paths` 已预留，供 24-04 或 25 阶段 UI 提示使用

---
*Phase: 24-claude-settings-overlay-end-to-end*
*Completed: 2026-03-17*

## Self-Check: PASSED

- FOUND: src-tauri/src/adapter/json_merge.rs
- FOUND: src-tauri/src/adapter/claude.rs
- FOUND: .planning/phases/24-claude-settings-overlay-end-to-end/24-03-SUMMARY.md
- FOUND commit: 884db66 (Task 1)
- FOUND commit: 5629237 (Task 2)
