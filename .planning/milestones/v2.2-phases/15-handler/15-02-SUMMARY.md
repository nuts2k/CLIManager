---
phase: 15-handler
plan: "02"
subsystem: proxy
tags: [rust, handler, protocol-routing, model-mapping, openai-compat, integration-test]

# Dependency graph
requires:
  - phase: 15-handler
    plan: "01"
    provides: "UpstreamTarget 扩展（upstream_model / upstream_model_map 字段）"
  - phase: 14-data-model-translate-core
    provides: "anthropic_to_openai / openai_to_anthropic / create_anthropic_sse_stream 转换函数"
provides:
  - "handler.rs 协议路由分支：OpenAiChatCompletions 走转换路径，Anthropic/OpenAiResponses 直接透传"
  - "apply_upstream_model_mapping 纯函数（三级优先级：精确匹配 > upstream_model > 保留原名）"
  - "6 个新测试（3 单元 + 3 集成）全部通过"
affects:
  - "proxy handler 端到端协议转换链路（Phase 15 核心交付）"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "协议路由分支：match upstream.protocol_type 决定请求/响应转换路径"
    - "模型映射纯函数：apply_upstream_model_mapping 在请求转换前执行，不依赖外部状态"
    - "SSE 流式路径：create_anthropic_sse_stream 包装 bytes_stream()，Body::from_stream 返回"
    - "4xx/5xx 错误响应直接透传，不经转换处理（RESP-05）"

key-files:
  created: []
  modified:
    - src-tauri/src/proxy/handler.rs
    - src-tauri/src/proxy/mod.rs

key-decisions:
  - "apply_upstream_model_mapping 在 anthropic_to_openai 之前执行——model 字段由 request.rs 原样透传，所以映射必须在 handler 层调用转换前完成"
  - "request_model 从 body_value（映射前）提取，用于 SSE 事件 message_start 中的 model 字段——保留客户端所见的原始模型名"
  - "Anthropic/OpenAiResponses 透传路径 is_streaming=false、request_model=String::new()——变量存在但不影响后续分支（_ 分支直接透传）"
  - "非流式转换后 content-length 无需额外处理——步骤 I 的过滤已移除上游响应中的 content-length 头"

patterns-established:
  - "handler 协议分支：C 步骤后 match protocol_type 决定 (upstream_url, final_body_bytes, is_streaming, request_model)"
  - "J 步骤响应分支：match protocol_type { OpenAiChatCompletions => { 非流/流/错误 }, _ => 透传 }"
  - "集成测试模式：mock 上游绑定 POST /v1/chat/completions，start_mock_upstream_router 启动，Arc<TokioMutex<Option<Value>>> capture 请求"

requirements-completed: [ROUT-01, ROUT-02, MODL-03]

# Metrics
duration: 3min
completed: 2026-03-14
---

# Phase 15 Plan 02: handler.rs 协议路由分支 + 集成测试 Summary

**在 handler.rs 中插入 OpenAiChatCompletions 完整转换链路（请求转换 + 模型映射 + 响应转换），Anthropic 直接透传，6 个新测试全部通过**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-14T14:24:32Z
- **Completed:** 2026-03-14T14:27:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `handler.rs` 步骤 C 后新增协议路由分支：`OpenAiChatCompletions` 走完整转换链路，`Anthropic`/`OpenAiResponses` 直接透传
- `apply_upstream_model_mapping` 纯函数实现三级优先级：精确匹配 `upstream_model_map` > `upstream_model` 默认 > 保留原名
- 步骤 J 按 `protocol_type` 分支处理响应体：非流式转换（`openai_to_anthropic`）、流式 SSE 转换（`create_anthropic_sse_stream`）、4xx/5xx 直接透传
- `mod.rs` 新增 `make_upstream_openai` 辅助函数 + 3 个集成测试，非流式/流式/模型映射 roundtrip 全部通过
- 300 个测试通过（294 原有 + 6 新增），仅已知 UX-01 端口冲突测试（pre-existing）除外

## Task Commits

每个任务原子提交：

1. **Task 1: handler.rs 协议路由分支 + 模型映射函数** - `6a6a1a0` (feat)
2. **Task 2: 集成测试 — OpenAiChatCompletions 非流式 + 流式 roundtrip** - `0e8b646` (feat)

**计划元数据提交：** 待 final commit

## Files Created/Modified

- `/Users/kelin/Workspace/CLIManager/src-tauri/src/proxy/handler.rs` - 新增 use 声明（bytes/HashMap/translate/UpstreamTarget）、apply_upstream_model_mapping 函数、步骤 C 后协议路由分支、步骤 H 使用 final_body_bytes、步骤 J 分支响应处理、3 个模型映射单元测试
- `/Users/kelin/Workspace/CLIManager/src-tauri/src/proxy/mod.rs` - 新增 make_upstream_openai 辅助函数、3 个 OpenAiChatCompletions 集成测试（非流式/流式/模型映射验证）

## Decisions Made

- `apply_upstream_model_mapping` 在 `anthropic_to_openai` 之前执行，因为 request.rs 中 model 字段原样透传，必须在 handler 层转换前完成映射
- `request_model` 从映射前的 `body_value` 提取，保留客户端所见的原始模型名，用于 SSE message_start 事件
- 透传路径（Anthropic/OpenAiResponses）的 `is_streaming`/`request_model` 变量赋默认值，不影响 `_` 分支行为

## Deviations from Plan

None - 计划执行完全符合预期。

## Issues Encountered

无。

## Next Phase Readiness

- Phase 15 核心交付完成：端到端协议转换链路（Anthropic 请求 → OpenAI 上游 → Anthropic 响应）已在 handler.rs 实现并通过集成测试
- Anthropic 透传路径零回归（ROUT-02）
- Phase 16（Responses API + Provider UI）可在此基础上扩展

## Self-Check: PASSED

- FOUND: `.planning/phases/15-handler/15-02-SUMMARY.md`
- FOUND: commit `6a6a1a0`
- FOUND: commit `0e8b646`

---
*Phase: 15-handler*
*Completed: 2026-03-14*
