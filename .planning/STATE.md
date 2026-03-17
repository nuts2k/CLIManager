---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: planning
stopped_at: Completed 24-04-PLAN.md
last_updated: "2026-03-17T00:36:56.147Z"
last_activity: 2026-03-17 — 24-03 json_merge 深度合并引擎 + ClaudeAdapter overlay 集成完成
progress:
  total_phases: 2
  completed_phases: 1
  total_plans: 4
  completed_plans: 4
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-15)

**Core value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容
**Current focus:** v2.5 Claude 全局配置 Overlay

## Current Position

Phase: **24 — 「Claude settings overlay end-to-end」** (ready)
Plan: —
Status: Ready to plan
Last activity: 2026-03-17 — 24-03 json_merge 深度合并引擎 + ClaudeAdapter overlay 集成完成

Progress: [░░░░░░░░░░] 0%

> 注：v2.5 从 Phase 24 开始继续编号。

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

v2.5 路线（Phase 24-25，最大化同 Phase 并行）：
- Phase 24: 「Claude settings overlay end-to-end」（UI + 存储 + 深度合并 + 保护字段 + apply 触发 + watcher + 错误可见性）
- Phase 25: 「测试覆盖」（合并规则/保护字段优先级/adapter 注入）
- [Phase 24-01]: overlay 存储与 providers 存储分离：config 目录独立于 providers 目录
- [Phase 24-01]: set_claude_settings_overlay 仅写入不 apply：apply 逻辑留后续 plan 实现
- [Phase 24-02]: 空字符串 overlay 跳过前端 JSON 校验，允许用户清空 overlay 内容
- [Phase 24-02]: 保存成功后重新调用 getClaudeSettingsOverlay() 刷新，确保回填后端 pretty 化后的最终内容
- [Phase 24-03]: overlay_path_override 注入模式：adapter 新增可选路径字段，测试时注入，生产时走全局存储
- [Phase 24-03]: patch_claude_json 末尾始终强制回写保护字段，保证 provider 优先级无法被 overlay 绕过
- [Phase 24-claude-settings-overlay-end-to-end]: startup apply 结果写入缓存队列（ClaudeOverlayStartupNotificationQueue），前端 useSyncListener 挂载后 take/replay，彻底解决 setup 时序问题
- [Phase 24-claude-settings-overlay-end-to-end]: set_claude_settings_overlay 保存后立即 apply（强一致 COVL-09）：apply 失败则 set 整体返回 Err；overlay 通知统一模型 ClaudeOverlayApplyNotification（kind/source/settings_path/error/paths）

### Pending Todos

None.

### Blockers/Concerns

- UX-01 端口冲突检测依赖脆弱的中文子串匹配（v2.0 遗留，低优先级）

## Session Continuity

Last session: 2026-03-17T00:36:56.145Z
Stopped at: Completed 24-04-PLAN.md
Resume file: None
