# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-13)

**Core value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容
**Current focus:** Phase 8 - 代理核心

## Current Position

Phase: 8 of 10 (代理核心)
Plan: 0 of ? in current phase
Status: Ready to plan
Last activity: 2026-03-13 — v2.0 roadmap created

Progress: [░░░░░░░░░░] 0% (v2.0 milestone: 0/3 phases)

## Performance Metrics

**Historical Velocity:**
- v1.0: 12 plans, ~1.12 hours total (avg 6min/plan)
- v1.1: 3 plans, ~25 min total (avg 8min/plan)
- Combined: 15 plans completed

**By Phase (v2.0):**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 8. 代理核心 | - | - | - |
| 9. 模式切换与持久化 | - | - | - |
| 10. 实时切换与 UI 集成 | - | - | - |

## Accumulated Context

### Decisions

Full decision log in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [v2.0 research]: axum 0.8 作为代理框架，复用 Tauri 内置 tokio runtime
- [v2.0 research]: 每 CLI 独立固定端口（Claude Code 15800, Codex 15801）
- [v2.0 research]: 绑定 127.0.0.1 避免 macOS 防火墙弹窗
- [v2.0 research]: takeover 标志持久化实现崩溃恢复

### Pending Todos

None.

### Blockers/Concerns

- [research]: 占位 API key 的具体格式需确认（各 CLI 对 key 格式有校验要求）
- [research]: Tauri app.exit() 调用 std::process::exit() 不触发 drop，退出时需显式执行还原逻辑
- [research]: cc-switch 用 axum 0.7，我们用 0.8，路径语法从 `/:param` 变为 `/{param}`
- [v1.1]: Release build tray behavior may differ from dev build (needs verification)

## Session Continuity

Last session: 2026-03-13
Stopped at: v2.0 roadmap created, ready to plan Phase 8
Resume file: None
