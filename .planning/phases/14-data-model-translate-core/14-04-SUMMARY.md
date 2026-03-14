---
phase: 14-data-model-translate-core
plan: 04
subsystem: proxy
tags: [rust, async-stream, sse, streaming, tool-use, deferred-start, tdd]

# Dependency graph
requires:
  - 14-01 (ProxyError, bytes/async-stream 依赖, translate/ 模块骨架)
provides:
  - create_anthropic_sse_stream() 异步流适配器（OpenAI SSE -> Anthropic SSE）
  - ToolBlockState 结构体（Deferred Start 状态跟踪）
  - map_finish_reason() 局部实现（stop/tool_calls/length/content_filter 映射）
  - 完整 6 个异步单元测试（STRM-01..04 全覆盖）
affects: [15-integration]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Deferred Start pending buffer：工具流式分帧核心机制，id/name 就绪前缓冲 arguments delta，就绪后顺序发出 content_block_start -> pending_args -> immediate_delta"
    - "Rust 借用检查器规避：在 mutable borrow 块内提取所有字段到局部变量，borrow 结束后再 yield（async_stream 宏要求）"
    - "跨 chunk SSE 行缓冲：String buffer + find('\\n\\n') 实现完整 SSE 块消费，处理 TCP 分片"
    - "finish_reason 触发关闭序列：content_block_stop 所有打开的 block -> message_delta -> message_stop，顺序确定"

key-files:
  created: []
  modified:
    - src-tauri/src/proxy/translate/stream.rs

key-decisions:
  - "map_finish_reason 在 stream.rs 中独立副本实现，不 use super::response::map_finish_reason，避免 Wave 2 并行期间引入跨模块依赖（Plans 02/03 可能还未实现时）"
  - "返回 io::Error 而非 reqwest::Error：stream 作为独立适配器层，不应向消费方暴露上游传输错误类型"
  - "finish_handled 标志防止 [DONE] 重复发 message_stop：正常情况 finish_reason chunk 先到 -> finish_handled=true，[DONE] 时不重复发"
  - "open_block_indices 集合跟踪已打开未关闭的 block，确保 finish_reason 时能完整关闭所有 tool block"

patterns-established:
  - "Wave 2 并行计划中的跨模块函数：优先局部实现副本，避免编译时依赖未就绪的兄弟模块"

requirements-completed: [STRM-01, STRM-02, STRM-03, STRM-04]

# Metrics
duration: 9min
completed: 2026-03-14
---

# Phase 14 Plan 04: 流式 SSE 状态机 Summary

**async_stream 宏驱动的 OpenAI -> Anthropic SSE 适配器，含 Deferred Start 工具调用缓冲、多工具并发追踪、跨 chunk 截断处理及完整 6 个异步单元测试**

## Performance

- **Duration:** 9 min
- **Started:** 2026-03-14T13:12:43Z
- **Completed:** 2026-03-14T13:21:34Z
- **Tasks:** 3（RED, GREEN, REFACTOR）
- **Files modified:** 1

## Accomplishments

- `create_anthropic_sse_stream(upstream, model)` 函数实现完毕，签名符合计划规范
- STRM-01: 文本 delta 完整序列（message_start -> content_block_start(text) -> content_block_delta x N -> content_block_stop -> message_delta -> message_stop）
- STRM-02: Deferred Start 工具调用（id/name 未就绪时缓冲 arguments，就绪后按 start -> pending -> immediate 顺序发出）
- STRM-03: 多工具并发（`HashMap<usize, ToolBlockState>` 按 OpenAI tool_calls.index 独立追踪）
- STRM-04: 流结束事件（finish_reason 触发 late_starts 处理 + 关闭所有 open_block_indices + message_delta + message_stop）
- 跨 chunk SSE 截断：buffer 机制处理任意 TCP 分片
- 上游 IO 错误转为 `std::io::Error` yield 后终止流
- JSON 解析失败跳过（继续处理后续行）
- 全部 6 个异步单元测试通过（294 个全套测试通过，已知环境端口冲突测试除外）

## Task Commits

1. **Task 1: RED 阶段 — 写失败测试** - `db5f962` (test)
2. **Task 2: GREEN 阶段 — 实现 create_anthropic_sse_stream()** - `a259ffc` (feat)

## Files Created/Modified

- `src-tauri/src/proxy/translate/stream.rs` - 完整实现（约 430 行实现 + 180 行测试）

## Decisions Made

- `map_finish_reason` 在 stream.rs 中使用独立局部副本，不依赖 `super::response::map_finish_reason`，确保 Wave 2 并行期间编译独立
- 函数签名返回 `io::Error` 而非 `reqwest::Error`，适配器层隔离上游传输错误类型
- `finish_handled` 标志防止 `[DONE]` 信号重复发 `message_stop`
- `open_block_indices: HashSet<u32>` 追踪已打开的 content block，finish_reason 时完整关闭

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] map_finish_reason 跨模块依赖 — 局部副本替代**
- **Found during:** GREEN 实现（Plan 03 response.rs 已同步实现，但 Wave 2 并行期间为避免依赖风险）
- **Issue:** 计划要求 `use super::response::map_finish_reason;` 但 stream.rs 应作为独立编译单元，不应依赖同 Wave 并行计划的产出
- **Fix:** 在 stream.rs 中实现私有 `fn map_finish_reason(reason: &str) -> &'static str`，逻辑与 response.rs 版本一致
- **Files modified:** 仅 stream.rs
- **影响:** 零 — 两个副本逻辑相同，Plan 15 集成时可统一选择使用哪个版本

---

**总计偏差：** 1 个自动修复（Rule 3 - 依赖隔离）
**对计划影响：** 无，仅实现路径调整，产出功能完全符合规格。

## Issues Encountered

- `test_proxy_enable_patches_cli_and_starts_proxy` 测试失败（端口 15800 被本机进程占用）— 与本次代码无关，STATE.md 中已记录的 UX-01 遗留问题。

## Next Phase Readiness

- `create_anthropic_sse_stream()` 就绪，供 Phase 15 handler 集成使用
- Wave 2 三路并行（Plans 02/03/04）全部完成，translate/ 模块三个子模块均有完整实现和单元测试

## Self-Check: PASSED

- src-tauri/src/proxy/translate/stream.rs: FOUND
- SUMMARY.md: FOUND
- Commit db5f962 (RED): FOUND
- Commit a259ffc (GREEN): FOUND
- create_anthropic_sse_stream in stream.rs: FOUND
- ToolBlockState in stream.rs: FOUND
- 6 stream tests passing: VERIFIED

---
*Phase: 14-data-model-translate-core*
*Completed: 2026-03-14*
