---
phase: 14-data-model-translate-core
plan: 01
subsystem: api
tags: [rust, serde, protocoltype, provider, translate, proxy, cargo]

# Dependency graph
requires: []
provides:
  - ProtocolType 三变体（Anthropic, OpenAiChatCompletions, OpenAiResponses）及 serde alias 向前兼容
  - Provider struct 新增 upstream_model / upstream_model_map 两个 Option 字段
  - proxy/translate/ 模块骨架（mod.rs + request/response/stream 占位子模块）
  - ProxyError::TranslateError variant（400 BAD_REQUEST 响应）
  - bytes / futures / async-stream 显式 Cargo 依赖
affects: [14-02, 14-03, 14-04, 15-integration]

# Tech tracking
tech-stack:
  added: [bytes = "1", async-stream = "0.3"]
  patterns:
    - "serde alias 向前兼容：旧 enum 变体名通过 #[serde(alias)] 继续支持旧 JSON"
    - "Option 字段 serde default + skip_serializing_if：旧 JSON 反序列化不崩溃，新字段序列化时自动省略"

key-files:
  created:
    - src-tauri/src/proxy/translate/mod.rs
    - src-tauri/src/proxy/translate/request.rs
    - src-tauri/src/proxy/translate/response.rs
    - src-tauri/src/proxy/translate/stream.rs
  modified:
    - src-tauri/src/provider.rs
    - src-tauri/src/proxy/handler.rs
    - src-tauri/src/proxy/state.rs
    - src-tauri/src/proxy/error.rs
    - src-tauri/src/proxy/mod.rs
    - src-tauri/Cargo.toml
    - src-tauri/src/commands/proxy.rs
    - src-tauri/src/commands/provider.rs
    - src-tauri/src/commands/onboarding.rs
    - src-tauri/src/adapter/claude.rs
    - src-tauri/src/adapter/codex.rs
    - src-tauri/src/adapter/mod.rs
    - src-tauri/src/storage/icloud.rs

key-decisions:
  - "OpenAiChatCompletions 替代旧名 OpenAiCompatible，通过 #[serde(alias)] 保持旧 JSON 向前兼容，不破坏已有配置文件"
  - "handler.rs 对 OpenAiChatCompletions|OpenAiResponses 统一使用 Bearer token 认证，Phase 16 再细化 Responses API 差异"
  - "TranslateError 返回 400 BAD_REQUEST：转换失败意味着请求内容无法处理，由调用方负责"

patterns-established:
  - "Rule 3 全量扫描：枚举变体重命名时，需扫描整个 codebase 所有引用点（命令/适配器/存储/测试辅助函数），统一一次性修复"
  - "Provider struct 字面量添加 `upstream_model: None, upstream_model_map: None` 作为新字段零值"

requirements-completed: [MODL-01, MODL-02]

# Metrics
duration: 10min
completed: 2026-03-14
---

# Phase 14 Plan 01: Provider 数据模型扩展 + 转换模块骨架 Summary

**ProtocolType 扩展为三变体并通过 serde alias 保持向前兼容，Provider 新增 upstream_model/upstream_model_map 字段，translate/ 模块骨架就绪，bytes/async-stream 依赖显式声明**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-14T12:59:04Z
- **Completed:** 2026-03-14T13:09:00Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments

- ProtocolType 扩展：Anthropic / OpenAiChatCompletions（含 "open_ai_compatible" alias）/ OpenAiResponses，TDD 验证 8 个测试
- Provider struct 新增 upstream_model / upstream_model_map 两个 Option 字段，旧 JSON 向前兼容
- proxy/translate/ 模块骨架创建完毕（mod.rs 声明 + 三个空子模块），编译就绪
- Cargo.toml 新增 bytes / futures / async-stream 依赖，ProxyError::TranslateError 可用

## Task Commits

1. **Task 1: Provider 数据模型扩展（ProtocolType 三变体 + upstream 映射字段）** - `8559ab3` (feat)
2. **Task 2: 转换模块骨架 + 依赖声明 + ProxyError 扩展** - `fd5326f` (feat)

## Files Created/Modified

- `src-tauri/src/provider.rs` - ProtocolType 三变体 + Provider upstream 字段 + 新增测试
- `src-tauri/src/proxy/translate/mod.rs` - 协议转换模块声明骨架
- `src-tauri/src/proxy/translate/request.rs` - 占位文件（Wave 2 实现）
- `src-tauri/src/proxy/translate/response.rs` - 占位文件（Wave 2 实现）
- `src-tauri/src/proxy/translate/stream.rs` - 占位文件（Wave 2 实现）
- `src-tauri/src/proxy/error.rs` - 新增 TranslateError variant（400 响应）
- `src-tauri/src/proxy/mod.rs` - 新增 pub mod translate 声明
- `src-tauri/src/proxy/handler.rs` - OpenAiCompatible → OpenAiChatCompletions|OpenAiResponses
- `src-tauri/src/proxy/state.rs` - 测试中 OpenAiCompatible → OpenAiChatCompletions
- `src-tauri/Cargo.toml` - 新增 bytes/futures/async-stream 依赖
- `src-tauri/src/commands/proxy.rs` - 全量更新变体名 + Provider 字面量补充新字段
- `src-tauri/src/commands/provider.rs` - 变体名更新 + Provider 字面量补充新字段
- `src-tauri/src/commands/onboarding.rs` - 全量替换 OpenAiCompatible 引用
- `src-tauri/src/adapter/claude.rs` - Provider 字面量补充新字段
- `src-tauri/src/adapter/codex.rs` - 变体名更新 + Provider 字面量补充新字段
- `src-tauri/src/adapter/mod.rs` - 变体名更新 + Provider 字面量补充新字段
- `src-tauri/src/storage/icloud.rs` - Provider 字面量补充新字段

## Decisions Made

- OpenAiChatCompletions 替代旧名 OpenAiCompatible，`#[serde(alias = "open_ai_compatible")]` 保证已有配置文件无需迁移
- handler.rs 暂时让 OpenAiResponses 与 OpenAiChatCompletions 使用相同的 Bearer token 认证；Phase 16 细化 Responses API 认证差异
- TranslateError 返回 400 BAD_REQUEST：转换失败属于请求内容问题，非服务器内部错误

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 全量修复 codebase 中 OpenAiCompatible 引用**
- **Found during:** Task 1（执行 cargo check 时发现编译错误）
- **Issue:** 除 plan 指定的 handler.rs / state.rs 外，还有 commands/proxy.rs、commands/provider.rs、commands/onboarding.rs、adapter/claude.rs、adapter/codex.rs、adapter/mod.rs、storage/icloud.rs 共 7 个文件引用了旧变体名，并有多处 Provider struct 字面量缺少新字段 upstream_model/upstream_model_map
- **Fix:** 一次性将所有 OpenAiCompatible 引用改为 OpenAiChatCompletions，所有 Provider struct 字面量补充两个新字段的 None 初始化
- **Files modified:** 全部 9 个受影响文件
- **Verification:** cargo check 无 error，61 个 provider 测试全部通过
- **Committed in:** 8559ab3（Task 1 commit）

---

**Total deviations:** 1 auto-fixed（Rule 3 - 阻塞编译）
**Impact on plan:** 必要修复，属于枚举变体重命名的正常全量更新，无范围扩张。

## Issues Encountered

- 测试 `test_proxy_enable_patches_cli_and_starts_proxy` 在本次执行中失败：端口 15800 被已运行的 cli-manager 进程（PID 49012）占用，属于本机环境问题，与本次代码变更无关（该测试使用固定端口 PROXY_PORT_CLAUDE=15800）。该问题在 STATE.md 中已有记录（UX-01 遗留）。

## Next Phase Readiness

- provider.rs 三变体模型和 upstream 字段就绪，可供 Wave 2 三路并行（Plans 02/03/04）使用
- translate/ 模块骨架已就绪，Wave 2 直接在各子模块实现具体转换逻辑
- bytes/async-stream 依赖已声明，stream.rs 实现时直接使用

## Self-Check: PASSED

- translate/mod.rs: FOUND
- translate/request.rs: FOUND
- translate/response.rs: FOUND
- translate/stream.rs: FOUND
- SUMMARY.md: FOUND
- Commit 8559ab3: FOUND
- Commit fd5326f: FOUND
- TranslateError in error.rs: FOUND
- async-stream in Cargo.toml: FOUND
- upstream_model in provider.rs: FOUND (22 occurrences)

---
*Phase: 14-data-model-translate-core*
*Completed: 2026-03-14*
