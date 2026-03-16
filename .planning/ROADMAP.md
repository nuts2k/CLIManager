# Roadmap: CLIManager

## Milestones

- ✅ **v1.0 MVP** — Phases 1-5 (shipped 2026-03-12)
- ✅ **v1.1 System Tray** — Phases 6-7 (shipped 2026-03-13)
- ✅ **v2.0 Local Proxy** — Phases 8-11 (shipped 2026-03-14)
- ✅ **v2.1 Release Engineering** — Phases 12-13 (shipped 2026-03-14)
- ✅ **v2.2 协议转换** — Phases 14-16 (shipped 2026-03-15)
- ✅ **v2.3 前端调整及美化** — Phases 17-22 (shipped 2026-03-15)
- ✅ **v2.4 Anthropic 模型映射** — Phase 23 (shipped 2026-03-15)
- **v2.5 Claude 全局配置 Overlay** — Phases 24-25 (planning)

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

<details>
<summary>✅ v2.4 Anthropic 模型映射 (Phase 23) — SHIPPED 2026-03-15</summary>

- [x] Phase 23: Anthropic 模型映射 (2/2 plans) — completed 2026-03-15

</details>

<details>
<summary>v2.5 Claude 全局配置 Overlay (Phases 24-25) — PLANNING</summary>

- [ ] **Phase 24: 「Claude settings overlay end-to-end」** — 端到端交付 overlay 的 UI + 存储 + 合并 + 保护字段 + apply 触发 + watcher + 错误可见性
- [ ] **Phase 25: 「测试覆盖」** — 自动化测试覆盖深度合并/保护字段优先级/ClaudeAdapter overlay 注入

</details>

## Phase Details

### Phase 24: 「Claude settings overlay end-to-end」
**Goal**: 用户在 CLIManager 内完成 Claude settings.json overlay 的端到端可用体验：可编辑/校验/保存（iCloud 优先，本地降级且位置可见），并能安全、自动、可见地 apply 到 `~/.claude/settings.json`（深度合并 + null 删除 + 保护字段优先 + 保存/启动/watcher 触发 + 错误可见性）。
**Depends on**: Phase 23
**Requirements**: COVL-01, COVL-02, COVL-03, COVL-04, COVL-05, COVL-06, COVL-07, COVL-08, COVL-09, COVL-10, COVL-11, COVL-12, COVL-13
**Success Criteria** (what must be TRUE):
  1. 用户在 Settings → Advanced → Claude 小节能编辑多行 JSON overlay，并在保存时获得 JSON 校验：非法 JSON 或 root 非 object 会给出明确错误提示且拒绝保存。
  2. 用户保存 overlay 后会立即 apply 到 `~/.claude/settings.json`，并按深度合并规则生效：object 递归合并、array 整体替换、scalar 覆盖；overlay 中将 key 设为 null 时会删除目标文件中的对应 key。
  3. 无论 overlay 如何设置，`env.ANTHROPIC_AUTH_TOKEN` 与 `env.ANTHROPIC_BASE_URL` 的最终值始终以 Provider/Proxy 写入为准；当 overlay 包含这些保护字段时会被忽略，且用户能在 UI 中看到“该字段由 Provider/Proxy 管理，不可覆盖”的提示。
  4. overlay 会被可靠持久化：iCloud 可用时写入可同步位置，不可用时自动降级写入本地目录；用户在 UI 中可以清楚看到当前存放位置（iCloud / 本地降级）以及是否跨设备同步。
  5. 自动对齐机制可观察且安全：应用启动时若 overlay 存在会 best-effort apply（失败不阻断启动但可通过日志/通知获知）；iCloud 同步导致 overlay 变更时 watcher 会自动触发 apply；当 overlay 文件或 `~/.claude/settings.json` 非法时会返回可见错误且不会静默覆盖/写坏原文件。
**Plans**: TBD

### Phase 25: 「测试覆盖」
**Goal**: 关键 overlay 注入行为具备可重复验证的自动化测试，防止深度合并/保护字段优先级/ClaudeAdapter surgical patch 回归。
**Depends on**: Phase 24
**Requirements**: COVL-14, COVL-15, COVL-16
**Success Criteria** (what must be TRUE):
  1. 开发者运行 `cargo test` 时，单元测试覆盖深度合并规则（递归合并/数组替换/标量覆盖/null 删除）并全部通过。
  2. 开发者运行 `cargo test` 时，测试能证明保护字段永远优先：overlay 尝试覆盖 `env.ANTHROPIC_AUTH_TOKEN` / `env.ANTHROPIC_BASE_URL` 不得生效，并全部通过。
  3. 开发者运行集成测试时，能证明 ClaudeAdapter 的 surgical patch 行为仍然成立，同时 overlay 注入的额外 env 字段能被写入且不影响凭据/模型字段 patch。
**Plans**: TBD

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
| 23. Anthropic 模型映射 | v2.4 | 2/2 | Complete | 2026-03-15 |
| 24. Claude settings overlay end-to-end | v2.5 | 0/0 | Not started | - |
| 25. 测试覆盖 | v2.5 | 0/0 | Not started | - |

---
*Roadmap created: 2026-03-12 (v1.0)*
*Last updated: 2026-03-16 — v2.5 roadmap revised (Phase 24-25)*
