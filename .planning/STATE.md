---
gsd_state_version: 1.0
milestone: v2.5
milestone_name: Claude 全局配置 Overlay
status: planning
stopped_at: —
last_updated: "2026-03-16T00:00:00.000Z"
last_activity: 2026-03-16 — Milestone v2.5 started
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-15)

**Core value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容
**Current focus:** v2.5 Claude 全局配置 Overlay

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-03-16 — Milestone v2.5 started

Progress: [░░░░░░░░░░] 0%

> 注：v2.5 将重新生成 REQUIREMENTS.md/ROADMAP.md，并从 Phase 24 开始继续编号。

## Performance Metrics

**Historical Velocity:**
- v1.0: 12 plans, ~1.12 hours total (avg 6min/plan)
- v1.1: 3 plans, ~25 min total (avg 8min/plan)
- v2.0: 7 plans, ~35 min total (avg 5min/plan)
- v2.1: 5 plans, ~39 min total (avg 8min/plan)
- v2.2: 10 plans, ~57 min total (avg 6min/plan)
- v2.3: 9 plans, ~1 day total
- Combined: 46 plans across 6 milestones

## Accumulated Context

### Decisions

（v2.3 决策已归档至 .planning/milestones/v2.3-ROADMAP.md）

关键架构背景（v2.4 相关）：
- handler.rs 三分支路由：Anthropic 分支目前直接透传，不执行模型映射
- 模型映射三级优先级逻辑已在 OpenAI 分支实现，v2.4 复用该逻辑到 Anthropic 分支
- 响应反向映射：客户端发送的是 Claude 模型名，代理替换后上游返回的是目标模型名，需映射回来
- Phase 23 合并后端+前端为单 Phase，2 个并行 Plan 最大化并行度
- [Phase 23-anthropic-model-mapping]: showModelMapping 改为 true，所有协议统一显示映射区域，Anthropic 字段为可选
- [Phase 23-anthropic-model-mapping]: isOpenAiProtocol 校验：upstreamModel 必填仅限 OpenAI 系列协议，Anthropic 可留空保存
- [Phase 23-anthropic-model-mapping]: reverse_model_in_sse_line 处理 message.model 嵌套：Anthropic message_start 事件的 model 在 message.model 而非顶层
- [Phase 23-anthropic-model-mapping]: 无映射配置时 Anthropic /v1/messages 走 Passthrough 而非 AnthropicPassthrough，保持零开销

### Pending Todos

None.

### Blockers/Concerns

- UX-01 端口冲突检测依赖脆弱的中文子串匹配（v2.0 遗留，低优先级）

## Session Continuity

Last session: 2026-03-15T15:06:41.699Z
Stopped at: Completed 23-anthropic-model-mapping 23-01-PLAN.md
Resume file: None
