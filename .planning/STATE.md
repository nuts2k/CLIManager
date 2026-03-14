---
gsd_state_version: 1.0
milestone: v2.2
milestone_name: 协议转换
status: planning
stopped_at: Phase 15 context gathered
last_updated: "2026-03-14T13:57:41.626Z"
last_activity: 2026-03-14 — v2.2 roadmap restructured (3 phases, 27 requirements, max parallelism)
progress:
  total_phases: 3
  completed_phases: 1
  total_plans: 4
  completed_plans: 4
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-14)

**Core value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容
**Current focus:** Phase 14 — 数据模型 + 转换核心

## Current Position

Phase: 14 of 16 (数据模型 + 转换核心)
Plan: — (未开始规划)
Status: Ready to plan
Last activity: 2026-03-14 — v2.2 roadmap restructured (3 phases, 27 requirements, max parallelism)

Progress: [██████████] 100% (Phase 14 完成)

## Performance Metrics

**Historical Velocity:**
- v1.0: 12 plans, ~1.12 hours total (avg 6min/plan)
- v1.1: 3 plans, ~25 min total (avg 8min/plan)
- v2.0: 7 plans, ~35 min total (avg 5min/plan)
- v2.1: 5 plans, ~39 min total (avg 8min/plan)
- Combined: 27 plans across 4 milestones

## Accumulated Context

### Decisions

- [v2.2]: 3 phases instead of 5 — 最大化并行度：Phase 14 内 Wave 2 三路并行（request/response/stream），Phase 16 内两路并行（Responses API + Provider UI）
- [v2.2]: 纯函数先行 — translate 子模块（request/response/stream）独立实现和单元测试，再集成到 handler
- [v2.2]: serde_json::Value 动态映射 — 比 typed struct 兼容未知字段，无需新核心 crate
- [v2.2]: 仅新增 bytes + futures 显式依赖 — 两者已作为传递依赖锁定在 Cargo.lock
- [v2.2]: Deferred Start pending buffer — 工具流式分帧核心机制，id/name 就绪后才发 content_block_start
- [v2.0]: PROXY_MANAGED 占位 key 标识代理接管配置
- [Phase 14-data-model-translate-core]: OpenAiChatCompletions 替代旧名 OpenAiCompatible，serde alias 保持向前兼容
- [Phase 14-data-model-translate-core]: TranslateError 返回 400 BAD_REQUEST，handler.rs OpenAiResponses 暂时使用 Bearer token 认证（Phase 16 细化）
- [Phase 14-data-model-translate-core]: model 字段原样透传，Phase 15 handler 层负责模型映射
- [Phase 14-data-model-translate-core]: clean_schema 移除所有 format + default 字段（扩展 cc-switch 只移除 format=uri 的逻辑）
- [Phase 14-data-model-translate-core]: arguments 反序列化失败包装为 {"raw": "原字符串"} 而非空对象——保留原始数据便于调试
- [Phase 14-data-model-translate-core]: stream.rs 中 map_finish_reason 使用局部副本，不依赖 response.rs，保持 Wave 2 并行编译独立性

### Pending Todos

None.

### Blockers/Concerns

- UX-01 端口冲突检测依赖脆弱的中文子串匹配（v2.0 遗留，低优先级）
- Phase 14 Plan C (stream.rs) 实现前需精读 cc-switch streaming.rs 第 280-347 行（Deferred Start 逻辑）— 已完成，Plan 04 实现时已应用

## Session Continuity

Last session: 2026-03-14T13:57:41.617Z
Stopped at: Phase 15 context gathered
Resume file: .planning/phases/15-handler/15-CONTEXT.md
