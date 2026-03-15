---
phase: 23-anthropic-model-mapping
plan: 01
subsystem: proxy
tags: [rust, anthropic, model-mapping, sse, handler]

# Dependency graph
requires: []
provides:
  - Anthropic /v1/messages 分支请求模型映射（三级优先级复用 apply_upstream_model_mapping）
  - 非流式 Anthropic 响应 model 字段反向映射（reverse_model_in_response）
  - 流式 SSE Anthropic 响应 model 字段反向映射（reverse_model_in_sse_line + create_anthropic_reverse_model_stream）
  - AnthropicPassthrough 响应模式变体
affects: [23-02]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Anthropic 透传分支独立路由：ProtocolType::Anthropic if path == "/v1/messages" 作为独立 match 分支
    - SSE 反向映射：逐行缓冲分割，处理顶层 model 及 message.model 嵌套字段（message_start 事件格式）
    - 响应反向映射模式：请求方向记录 request_model，响应分支时替换回原始名

key-files:
  created: []
  modified:
    - src-tauri/src/proxy/handler.rs

key-decisions:
  - "reverse_model_in_sse_line 需同时处理顶层 model 和 message.model 嵌套字段，因为 Anthropic message_start 事件将 model 放在 message.model 而非顶层"
  - "无映射配置时 Anthropic /v1/messages 走 Passthrough 而非 AnthropicPassthrough，保持零开销原有行为"
  - "SSE 流处理采用 async_stream::stream! + 行缓冲方案，与 OpenAI 分支保持一致模式"

patterns-established:
  - "响应反向映射模式：在路由分支记录 request_model → AnthropicPassthrough 变体携带 → 响应分支替换回原始名"

requirements-completed: [MMAP-01, MMAP-02, MMAP-03]

# Metrics
duration: 7min
completed: 2026-03-15
---

# Phase 23 Plan 01: Anthropic 模型映射 Summary

**Anthropic /v1/messages 透传分支新增三级模型映射 + 非流式/流式 SSE 响应 model 字段反向映射，复用现有 apply_upstream_model_mapping 函数**

## Performance

- **Duration:** 7 分钟
- **Started:** 2026-03-15T09:18:14Z
- **Completed:** 2026-03-15T09:25:27Z
- **Tasks:** 1（TDD：RED + GREEN 两次提交）
- **Files modified:** 1

## Accomplishments

- Anthropic 协议 + /v1/messages 路由分支从 `_ =>` fallback 升级为显式分支，具备完整模型映射能力
- 请求方向：复用 `apply_upstream_model_mapping`（三级优先级：精确匹配 > upstream_model > 保留原名）
- 响应方向：非流式响应读完后替换 model 字段回原始请求名
- 流式 SSE：逐行扫描，处理顶层 model 字段和 `message.model` 嵌套字段（message_start 事件）
- 无映射配置时完全保持原有纯透传行为（走 Passthrough，不解析请求体）
- 新增 11 个 Anthropic 分支专属测试 + 单元测试，全量 367 个测试 0 失败

## Task Commits

TDD 流程各阶段独立提交：

1. **Task 1 RED: 添加失败测试** - `cc63a6c` (test)
2. **Task 1 GREEN: 实现功能** - `d623c45` (feat)

## Files Created/Modified

- `src-tauri/src/proxy/handler.rs` — 新增 AnthropicPassthrough 变体、Anthropic /v1/messages 路由分支、reverse_model_in_response、reverse_model_in_sse_line、create_anthropic_reverse_model_stream、响应分支处理、11 个新测试

## Decisions Made

- `reverse_model_in_sse_line` 需同时处理顶层 `model` 和 `message.model` 嵌套字段：Anthropic message_start 事件格式是 `{"type":"message_start","message":{"model":"..."}}`，model 在嵌套层；其他事件类型（content_block_delta 等）无 model 字段，不受影响
- 无映射配置时走 `Passthrough` 而非 `AnthropicPassthrough`，避免对请求体进行不必要的 JSON 解析/序列化
- SSE 流处理采用 `async_stream::stream!` + 行缓冲方案（与 OpenAI 分支一致），处理跨 chunk 不完整行

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] reverse_model_in_sse_line 扩展为处理嵌套 message.model**
- **Found during:** Task 1 GREEN 阶段（test_anthropic_messages_sse_model_reverse_mapped 失败）
- **Issue:** 计划中描述函数"替换 data: 行中的 model 字段"，但 Anthropic message_start SSE 事件的 model 字段在 `message.model` 嵌套层，不在顶层，导致流式测试失败
- **Fix:** 在 `reverse_model_in_sse_line` 中增加对 `message.model` 嵌套字段的处理
- **Files modified:** src-tauri/src/proxy/handler.rs
- **Verification:** test_anthropic_messages_sse_model_reverse_mapped 通过，367 个全量测试 0 失败
- **Committed in:** d623c45（与功能实现同一提交）

---

**Total deviations:** 1 auto-fixed（Rule 1 Bug）
**Impact on plan:** 必要修正，确保 SSE 流式反向映射正确处理 Anthropic 事件格式。无范围扩张。

## Issues Encountered

无额外问题——Anthropic SSE 格式的嵌套 model 字段在 GREEN 阶段第一次测试运行时即发现并修复。

## Next Phase Readiness

- 后端 Anthropic 模型映射完成，所有 MMAP-01/02/03 需求已满足
- Plan 23-02（前端 UI）可并行执行，无代码依赖

---
*Phase: 23-anthropic-model-mapping*
*Completed: 2026-03-15*
