---
phase: 14-data-model-translate-core
plan: 02
subsystem: api
tags: [rust, serde_json, translate, proxy, anthropic, openai, tdd]

# Dependency graph
requires:
  - phase: 14-data-model-translate-core/14-01
    provides: "ProxyError::TranslateError variant + translate/ 模块骨架"
provides:
  - anthropic_to_openai() 纯函数：Anthropic Messages API → OpenAI Chat Completions API 请求转换
  - build_proxy_endpoint_url() 纯函数：端点 URL 重写（/v1 路径处理）
  - clean_schema() 纯函数：递归清理 JSON Schema 不兼容字段（format/default）
  - 29 个单元测试覆盖所有 REQT-01..08 场景
affects: [15-integration, 14-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "纯函数转换：不依赖外部状态，serde_json::Value 动态映射"
    - "TDD GREEN：测试与实现同文件，29 个测试直接通过"
    - "cache_control 透传模式：system/text block/tool 三处一致的透传逻辑"
    - "静默丢弃模式：thinking blocks 和未知 content block type 不报错"

key-files:
  created: []
  modified:
    - src-tauri/src/proxy/translate/request.rs
    - src-tauri/src/proxy/translate/stream.rs

key-decisions:
  - "model 字段原样透传（Phase 15 handler 层负责模型映射，不在转换层处理）"
  - "多段 text block 合并策略：无 cache_control 时换行合并为字符串；有 cache_control 时保持数组格式"
  - "BatchTool 过滤：type==BatchTool 的工具定义不出现在转换结果中，无报错"
  - "clean_schema 移除 format + default（cc-switch 参考只移除 format=uri，本实现扩展为移除所有 format）"

patterns-established:
  - "Rule 3 占位 stub：依赖函数未实现时先加空占位（stream.rs create_anthropic_sse_stream）解除编译阻塞，不干扰其他计划的 RED 测试"

requirements-completed: [REQT-01, REQT-02, REQT-03, REQT-04, REQT-05, REQT-06, REQT-07, REQT-08]

# Metrics
duration: 4min
completed: 2026-03-14
---

# Phase 14 Plan 02: 请求转换 anthropic_to_openai() Summary

**anthropic_to_openai() + build_proxy_endpoint_url() + clean_schema() 三个纯函数实现 Anthropic→OpenAI 请求转换，29 个单元测试覆盖全部 REQT-01..08 场景**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-14T13:12:21Z
- **Completed:** 2026-03-14T13:16:21Z
- **Tasks:** 1 (TDD: RED + GREEN 合并执行)
- **Files modified:** 2

## Accomplishments

- anthropic_to_openai()：完整实现 Anthropic Messages API → OpenAI Chat Completions API 请求转换，处理 system（字符串/数组）、text/tool_use/tool_result/image 四类 content block、BatchTool 过滤、thinking blocks 静默丢弃、cache_control 透传
- build_proxy_endpoint_url()：端点重写逻辑，正确处理 base_url 含/不含 /v1 两种情况（含尾部斜杠处理）
- clean_schema()：递归移除 JSON Schema 中 format 和 default 字段（OpenAI 不兼容）
- Rule 3 fix：stream.rs 添加 create_anthropic_sse_stream 占位实现解除编译阻塞（Plan 14-04 将实现完整逻辑）

## Task Commits

1. **TDD GREEN: anthropic_to_openai() + 辅助函数 + 29 个单元测试** - `62ba1da` (feat)

**Plan metadata:** (本 SUMMARY 提交)

## Files Created/Modified

- `src-tauri/src/proxy/translate/request.rs` - anthropic_to_openai() + build_proxy_endpoint_url() + clean_schema() 三个纯函数 + 29 个单元测试
- `src-tauri/src/proxy/translate/stream.rs` - 添加 create_anthropic_sse_stream 占位实现（Rule 3 fix）

## Decisions Made

- model 字段原样透传：Phase 14 转换层不做模型映射，由 Phase 15 handler 层处理
- 多段 text block 合并策略：无 cache_control 时用换行符合并为单字符串；有 cache_control 时保持数组格式以保留 cache_control 字段
- clean_schema 扩展：cc-switch 参考实现只移除 format="uri"，本实现改为移除所有 format 字段（以及 default 字段），更彻底地覆盖 OpenAI 不兼容字段
- BatchTool 过滤条件：检查 type == "BatchTool"（而非名称），与 cc-switch 参考一致

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] stream.rs 编译阻塞：create_anthropic_sse_stream 未定义**
- **Found during:** 执行 cargo test 时
- **Issue:** stream.rs（Plan 14-04 RED 阶段测试文件）引用 `create_anthropic_sse_stream` 函数，而该函数尚未实现，导致整个 crate 编译失败，无法运行 request.rs 测试
- **Fix:** 在 stream.rs 顶部添加 `create_anthropic_sse_stream` 占位实现（返回空流），满足类型检查，不影响 Plan 14-04 的 RED 测试语义
- **Files modified:** src-tauri/src/proxy/translate/stream.rs
- **Verification:** cargo test --package cli-manager proxy::translate::request::tests 全部通过（29/29）
- **Committed in:** 62ba1da（Task commit）

---

**Total deviations:** 1 auto-fixed（Rule 3 - 编译阻塞）
**Impact on plan:** 必要修复，stream.rs 占位不影响 Plan 14-04 的实现计划，仅解除编译依赖。

## Issues Encountered

- stream.rs 已有 Plan 14-04 的 RED 阶段测试（type annotation 问题），添加占位 stub 后解决编译问题，stream 测试仍在预期的 RED 状态（输出空流，测试失败）

## Next Phase Readiness

- request.rs 三个纯函数就绪，可供 Phase 15 integration handler 直接调用
- 29 个单元测试作为回归保护，确保后续修改不破坏转换逻辑
- response.rs（Plan 14-03）和 stream.rs（Plan 14-04）仍需实现

## Self-Check: PASSED

- request.rs: FOUND
- stream.rs stub: FOUND
- Commit 62ba1da: FOUND (git log verified)
- 29 tests passing: VERIFIED (cargo test output)

---
*Phase: 14-data-model-translate-core*
*Completed: 2026-03-14*
