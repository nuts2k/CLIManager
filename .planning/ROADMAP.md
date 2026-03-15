# Roadmap: CLIManager

## Milestones

- ✅ **v1.0 MVP** — Phases 1-5 (shipped 2026-03-12)
- ✅ **v1.1 System Tray** — Phases 6-7 (shipped 2026-03-13)
- ✅ **v2.0 Local Proxy** — Phases 8-11 (shipped 2026-03-14)
- ✅ **v2.1 Release Engineering** — Phases 12-13 (shipped 2026-03-14)
- ✅ **v2.2 协议转换** — Phases 14-16 (shipped 2026-03-15)
- ✅ **v2.3 前端调整及美化** — Phases 17-22 (shipped 2026-03-15)
- 🚧 **v2.4 Anthropic 模型映射** — Phase 23 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-5) — SHIPPED 2026-03-12</summary>

- [x] Phase 1: Storage and Data Model (2/2 plans) — completed 2026-03-10
- [x] Phase 2: Surgical Patch Engine (2/2 plans) — completed 2026-03-11
- [x] Phase 3: Provider Management UI (4/4 plans) — completed 2026-03-11
- [x] Phase 4: iCloud Sync and File Watching (2/2 plans) — completed 2026-03-11
- [x] Phase 5: Onboarding (2/2 plans) — completed 2026-03-12

</details>

<details>
<summary>✅ v1.1 System Tray (Phases 6-7) — SHIPPED 2026-03-13</summary>

- [x] Phase 6: Tray Foundation (1/1 plan) — completed 2026-03-13
- [x] Phase 7: Provider Menu and Switching (2/2 plans) — completed 2026-03-13

</details>

<details>
<summary>✅ v2.0 Local Proxy (Phases 8-11) — SHIPPED 2026-03-14</summary>

- [x] Phase 8: 代理核心 (2/2 plans) — completed 2026-03-13
- [x] Phase 9: 模式切换与持久化 (2/2 plans) — completed 2026-03-13
- [x] Phase 10: 实时切换与 UI 集成 (2/2 plans) — completed 2026-03-14
- [x] Phase 11: 代理感知修复与文档同步 (1/1 plan) — completed 2026-03-14

</details>

<details>
<summary>✅ v2.1 Release Engineering (Phases 12-13) — SHIPPED 2026-03-14</summary>

- [x] Phase 12: 全栈实现 (4/4 plans) — completed 2026-03-14
- [x] Phase 13: 端到端验证 (1/1 plan) — completed 2026-03-14

</details>

<details>
<summary>✅ v2.2 协议转换 (Phases 14-16) — SHIPPED 2026-03-15</summary>

- [x] Phase 14: 数据模型 + 转换核心 (4/4 plans) — completed 2026-03-14
- [x] Phase 15: Handler 集成与协议路由 (2/2 plans) — completed 2026-03-14
- [x] Phase 16: Responses API + Provider UI (4/4 plans) — completed 2026-03-14

</details>

<details>
<summary>✅ v2.3 前端调整及美化 (Phases 17-22) — SHIPPED 2026-03-15</summary>

- [x] Phase 17: 设计基础 (2/2 plans) — completed 2026-03-15
- [x] Phase 18: 首页布局优化 (2/2 plans) — completed 2026-03-15
- [x] Phase 19: Provider 编辑改进 (1/1 plan) — completed 2026-03-15
- [x] Phase 20: 设置页 Tab 化 (1/1 plan) — completed 2026-03-15
- [x] Phase 21: 微动效与 Header 提升 (1/1 plan) — completed 2026-03-15
- [x] Phase 22: 应用图标 (2/2 plans) — completed 2026-03-15

</details>

### v2.4 Anthropic 模型映射 (In Progress)

**Milestone Goal:** Anthropic 协议透传路径支持模型映射，使用户可通过 Anthropic 协议连接不同模型名的 Provider

- [ ] **Phase 23: Anthropic 模型映射** — 后端透传映射 + 前端配置 UI（2 个并行 Plan）

## Phase Details

### Phase 23: Anthropic 模型映射
**Goal**: Anthropic 协议透传路径完整支持模型映射 — 代理层请求/响应/流式映射 + Provider 编辑 UI 配置入口
**Depends on**: Phase 22 (v2.3 完成)
**Requirements**: MMAP-01, MMAP-02, MMAP-03, MMAP-04
**Parallelism**: 2 plans（后端 + 前端）无依赖，可并行执行
**Plans:** 2 plans
Plans:
- [ ] 23-01-PLAN.md — 后端 handler.rs Anthropic 分支模型映射（请求/响应/流式）
- [ ] 23-02-PLAN.md — 前端 Provider 编辑 UI 显示 Anthropic 模型映射配置
**Success Criteria** (what must be TRUE):
  1. 代理模式下，Anthropic 协议 Provider 配置了模型映射时，转发出去的请求中 model 字段已被替换为目标模型名
  2. 代理模式下，上游返回的非流式响应中 model 字段被映射回原始 Claude 模型名，客户端看到的是原始名
  3. 代理模式下，上游返回的 SSE 流式响应中 model 字段被映射回原始 Claude 模型名
  4. 无模型映射配置时，Anthropic 透传路径行为不变（原始 model 名透传）
  5. 编辑 Anthropic 协议 Provider 时，表单中出现默认模型字段和模型映射对列表区域
  6. 模型映射对和默认模型均为可选，字段留空时不影响保存
  7. 配置的映射规则持久化保存，重新打开 Provider 编辑时映射数据正确回填

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Storage and Data Model | v1.0 | 2/2 | Complete | 2026-03-10 |
| 2. Surgical Patch Engine | v1.0 | 2/2 | Complete | 2026-03-11 |
| 3. Provider Management UI | v1.0 | 4/4 | Complete | 2026-03-11 |
| 4. iCloud Sync and File Watching | v1.0 | 2/2 | Complete | 2026-03-11 |
| 5. Onboarding | v1.0 | 2/2 | Complete | 2026-03-12 |
| 6. Tray Foundation | v1.1 | 1/1 | Complete | 2026-03-13 |
| 7. Provider Menu and Switching | v1.1 | 2/2 | Complete | 2026-03-13 |
| 8. 代理核心 | v2.0 | 2/2 | Complete | 2026-03-13 |
| 9. 模式切换与持久化 | v2.0 | 2/2 | Complete | 2026-03-13 |
| 10. 实时切换与 UI 集成 | v2.0 | 2/2 | Complete | 2026-03-14 |
| 11. 代理感知修复与文档同步 | v2.0 | 1/1 | Complete | 2026-03-14 |
| 12. 全栈实现 | v2.1 | 4/4 | Complete | 2026-03-14 |
| 13. 端到端验证 | v2.1 | 1/1 | Complete | 2026-03-14 |
| 14. 数据模型 + 转换核心 | v2.2 | 4/4 | Complete | 2026-03-14 |
| 15. Handler 集成与协议路由 | v2.2 | 2/2 | Complete | 2026-03-14 |
| 16. Responses API + Provider UI | v2.2 | 4/4 | Complete | 2026-03-14 |
| 17. 设计基础 | v2.3 | 2/2 | Complete | 2026-03-15 |
| 18. 首页布局优化 | v2.3 | 2/2 | Complete | 2026-03-15 |
| 19. Provider 编辑改进 | v2.3 | 1/1 | Complete | 2026-03-15 |
| 20. 设置页 Tab 化 | v2.3 | 1/1 | Complete | 2026-03-15 |
| 21. 微动效与 Header 提升 | v2.3 | 1/1 | Complete | 2026-03-15 |
| 22. 应用图标 | v2.3 | 2/2 | Complete | 2026-03-15 |
| 23. Anthropic 模型映射 | v2.4 | 0/2 | Not started | - |

---
*Roadmap created: 2026-03-12 (v1.0)*
*Last updated: 2026-03-15 — Phase 23 planned: 2 parallel plans (backend + frontend)*
