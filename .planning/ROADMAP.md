# Roadmap: CLIManager

## Milestones

- ✅ **v1.0 MVP** — Phases 1-5 (shipped 2026-03-12)
- ✅ **v1.1 System Tray** — Phases 6-7 (shipped 2026-03-13)
- ✅ **v2.0 Local Proxy** — Phases 8-11 (shipped 2026-03-14)
- ✅ **v2.1 Release Engineering** — Phases 12-13 (shipped 2026-03-14)
- 🚧 **v2.2 协议转换** — Phases 14-16 (in progress)

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

### 🚧 v2.2 协议转换 (In Progress)

**Milestone Goal:** 让 Claude Code 通过代理模式使用 OpenAI 兼容的 Provider，代理层自动完成 Anthropic Messages API ↔ OpenAI Chat Completions API 的协议转换

- [ ] **Phase 14: 数据模型 + 转换核心** — Provider 数据模型扩展，以及请求转换、响应转换、流式 SSE 状态机三条并行开发路线，可独立单元测试
- [ ] **Phase 15: Handler 集成与协议路由** — 转换层接入 proxy_handler，实现协议路由，端到端验证
- [ ] **Phase 16: Responses API + Provider UI** — Responses API 转换层与 Provider 编辑 UI 模型映射配置，两条并行开发路线

## Phase Details

### Phase 14: 数据模型 + 转换核心
**Goal**: Provider 数据模型扩展完成，请求转换、响应转换、流式 SSE 三个转换模块全部实现并通过单元测试，可独立于 handler 验证
**Depends on**: Phase 13 (v2.1 代理基础设施就绪)
**Requirements**: MODL-01, MODL-02, REQT-01, REQT-02, REQT-03, REQT-04, REQT-05, REQT-06, REQT-07, REQT-08, RESP-01, RESP-02, RESP-03, RESP-04, RESP-05, STRM-01, STRM-02, STRM-03, STRM-04

**Parallel Execution Note:**
- Wave 1（串行，必须先行）: Provider 数据模型扩展 — MODL-01、MODL-02 是后续转换模块的 schema 基础
- Wave 2（三路并行）:
  - Plan A: 请求转换纯函数 `anthropic_to_openai()` — REQT-01..08
  - Plan B: 非流式响应转换纯函数 `openai_to_anthropic()` — RESP-01..05
  - Plan C: 流式 SSE 状态机 `create_anthropic_sse_stream()` — STRM-01..04

**Success Criteria** (what must be TRUE):
  1. Provider JSON 文件新增 `default_model`、`model_mappings` 字段，旧 Provider 文件向前兼容（字段缺失时取默认值，不崩溃）
  2. `anthropic_to_openai()` 纯函数将包含 system prompt（字符串和数组两种格式）、text/tool_use/tool_result/image 各类 content block 的 Anthropic 请求正确转换为 OpenAI 格式，端点 `/v1/messages` 重写为 `/v1/chat/completions`，单元测试全绿
  3. `openai_to_anthropic()` 纯函数将含文本和工具调用的 OpenAI 非流式响应正确转换为 Anthropic 格式，finish_reason 穷举映射，usage 字段重命名，4xx/5xx 错误响应直接透传，单元测试全绿
  4. `create_anthropic_sse_stream()` 流适配器将 OpenAI content delta 序列转换为完整 Anthropic SSE 事件序列（message_start → content_block_start → content_block_delta × N → content_block_stop → message_delta → message_stop），Deferred Start 机制正确缓冲工具调用分帧，多工具并发各自独立状态，跨 chunk 截断正确处理，单元测试全绿
  5. thinking block、cache_control、不兼容 JSON Schema 字段均被静默丢弃，不触发上游 400 错误（可通过请求转换单元测试验证）
**Plans**: TBD

### Phase 15: Handler 集成与协议路由
**Goal**: 转换层完整接入 proxy_handler，OpenAiCompatible Provider 请求自动走转换路径并按模型映射替换模型名，Anthropic Provider 零回归，端到端请求-响应链路验证通过
**Depends on**: Phase 14 (三个转换模块就绪，数据模型扩展就绪)
**Requirements**: ROUT-01, ROUT-02, MODL-03

**Parallel Execution Note:** 单一交付路线，先集成 handler，再做 E2E 验证。

**Success Criteria** (what must be TRUE):
  1. 将 Claude Code 配置为使用 OpenAI 兼容 Provider（如 OpenRouter），发送真实请求后代理自动转换协议并返回正确 Anthropic 格式响应，Claude Code 正常显示输出
  2. 代理转换时请求中的模型名按映射表替换：精确匹配优先，无匹配时退回 default_model，default_model 也未配置时保留原模型名
  3. 现有 Anthropic Provider 请求经代理后行为与 v2.1 完全相同（透传路径不受任何影响），全部现有测试继续通过
  4. 流式请求（SSE）经代理转换后 Claude Code 逐 token 流式显示，工具调用流式返回后 Claude Code 正常解析，无截断或乱序
**Plans**: TBD

### Phase 16: Responses API + Provider UI
**Goal**: OpenAI Responses API 格式转换层可用，Provider 编辑界面支持配置默认模型和模型映射对，两条路线完成后整体 UI 工作流验证通过
**Depends on**: Phase 15 (handler 集成完成，基础转换路径验证通过)
**Requirements**: RAPI-01, RAPI-02, RAPI-03, RAPI-04, MODL-04

**Parallel Execution Note:**
- Wave 1（两路并行）:
  - Plan A: Responses API 转换层 — RAPI-01..04
  - Plan B: Provider 编辑 UI 模型映射配置 — MODL-04

**Success Criteria** (what must be TRUE):
  1. Provider 编辑页新增默认模型输入框和模型映射对列表（可增删），保存后立即生效，代理转换使用新映射
  2. Provider 可配置目标 API 格式（Chat Completions / Responses），选择 Responses 后请求和响应均走 Responses API 转换路径
  3. Responses API 非流式和流式两条路径均能将上游响应正确转换回 Anthropic 格式，Claude Code 正常解析
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
| 14. 数据模型 + 转换核心 | v2.2 | 0/? | Not started | - |
| 15. Handler 集成与协议路由 | v2.2 | 0/? | Not started | - |
| 16. Responses API + Provider UI | v2.2 | 0/? | Not started | - |

---
*Roadmap created: 2026-03-12 (v1.0)*
*Last updated: 2026-03-14 — v2.2 restructured to 3 phases (14-16) for maximum parallelism, 27 requirements fully covered*
