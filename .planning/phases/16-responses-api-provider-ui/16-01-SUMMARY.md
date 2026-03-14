---
phase: 16-responses-api-provider-ui
plan: "01"
subsystem: proxy/translate
tags: [rust, tdd, responses-api, request-translation]
dependency_graph:
  requires: []
  provides: [anthropic_to_responses]
  affects: [proxy/handler]
tech_stack:
  added: []
  patterns: [tdd, pure-function, serde_json-dynamic-mapping]
key_files:
  created:
    - src-tauri/src/proxy/translate/responses_request.rs
  modified:
    - src-tauri/src/proxy/translate/mod.rs
decisions:
  - "image block 转换为 {type:'input_image', image_url:'data:...'} 格式（符合 Responses API 规范）"
  - "多轮对话中 tool_use 先推送已有文本再插入 function_call 独立项，确保顺序正确"
  - "复用 super::request::clean_schema 而非重复实现"
metrics:
  duration_seconds: 250
  completed_date: "2026-03-14"
  tasks_completed: 1
  files_created: 1
  files_modified: 1
---

# Phase 16 Plan 01: Anthropic → Responses API 请求转换纯函数 Summary

**一句话总结：** 实现 anthropic_to_responses() 纯函数（531 行），覆盖 12 个 TDD 单元测试，包括 max_tokens 映射、system→instructions、工具定义无 function 包装层、tool_use/tool_result 转为独立 input 项、图片转换、thinking blocks 丢弃、Schema 清理。

## What Was Built

实现了 `src-tauri/src/proxy/translate/responses_request.rs` 模块，提供 Anthropic Messages API → OpenAI Responses API 的完整请求转换。

**核心差异（vs Chat Completions）：**
- `max_tokens` → `max_output_tokens`（字段名不同）
- `system`（字符串/数组）→ `instructions`（单字符串，数组多段换行拼接）
- `messages` → `input`（OpenAI Responses API 使用 input 字段）
- 工具定义：无 `function:{}` 包装层，name/description/parameters 直接放顶层
- `tool_use` block → `{type:"function_call"}` 独立 input 项
- `tool_result` block → `{type:"function_call_output"}` 独立 input 项
- image → `{type:"input_image", image_url:"data:..."}` 格式

**内部辅助函数：**
- `extract_system_text()` — 提取 system 为字符串
- `convert_messages_to_input()` — messages 数组转换
- `convert_tool_definitions()` — 工具定义转换

## Tests

12 个单元测试全部通过：
1. `test_basic_text_request` — 基础文本请求完整转换
2. `test_system_array_format` — system 数组格式拼接
3. `test_max_tokens_mapping` — max_tokens → max_output_tokens，原字段不存在
4. `test_tools_no_function_wrapper` — 工具定义无 function 包装层
5. `test_tool_result_to_function_call_output` — tool_result 转换
6. `test_assistant_tool_use_to_function_call` — tool_use 转换
7. `test_multi_turn_conversation` — 多轮对话完整顺序验证
8. `test_stream_passthrough` — stream 标志透传
9. `test_standard_params` — temperature/top_p/stop_sequences 映射
10. `test_thinking_blocks_dropped` — thinking blocks 静默丢弃
11. `test_image_content_to_input_image` — 图片内容转换
12. `test_clean_schema_applied` — JSON Schema format/default 字段清理

## Deviations from Plan

### 执行环境偏差（非代码问题）

**并行 Agent 共享文件：** mod.rs 在本 agent 执行时已被 plan 16-02 的 agent 更新（添加了 responses_response 和 responses_stream 模块），本 agent 基于最新版本合并添加了 responses_request 模块。最终 mod.rs 包含六个子模块（正确）。

**提交归属：** responses_request.rs 和 mod.rs 的最终版本被包含在提交 `e7610a0`（feat(16-03)）中，因为并行 agent 先暂存了这些文件。文件内容完全正确，测试全部通过。

除此之外，无其他偏差，计划按规格执行。

## Self-Check: PASSED

- responses_request.rs: FOUND (531 行，12 个测试)
- 16-01-SUMMARY.md: FOUND
- 提交 e7610a0: FOUND
- cargo test 12/12: PASSED
