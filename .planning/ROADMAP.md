# Roadmap: CLIManager

## Milestones

- ✅ **v1.0 MVP** — Phases 1-5 (shipped 2026-03-12)
- ✅ **v1.1 System Tray** — Phases 6-7 (shipped 2026-03-13)
- ✅ **v2.0 Local Proxy** — Phases 8-11 (shipped 2026-03-14)
- ✅ **v2.1 Release Engineering** — Phases 12-13 (shipped 2026-03-14)
- ✅ **v2.2 协议转换** — Phases 14-16 (shipped 2026-03-15)
- ✅ **v2.3 前端调整及美化** — Phases 17-22 (shipped 2026-03-15)
- ✅ **v2.4 Anthropic 模型映射** — Phase 23 (shipped 2026-03-15)
- ✅ **v2.5 Claude 全局配置 Overlay** — Phases 24-25 (shipped 2026-03-17)
- 🚧 **v2.6 流量监控** — Phases 26-30 (in progress)

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
<summary>✅ v2.5 Claude 全局配置 Overlay (Phases 24-25) — SHIPPED 2026-03-17</summary>

- [x] Phase 24: Claude settings overlay end-to-end (4/4 plans) — completed 2026-03-17
- [x] Phase 25: 测试覆盖 (1/1 plan) — completed 2026-03-17

</details>

### 🚧 v2.6 流量监控 (Phases 26-30)

**Milestone Goal:** 为代理模式增加请求日志记录、实时流量展示和统计分析能力

- [x] **Phase 26: SQLite 基础设施** — 初始化 traffic.db，建立两张表 schema 和连接管理 (completed 2026-03-18)
- [x] **Phase 27: 日志写入管道** — handler 采集元数据经 mpsc channel 写入 SQLite 并 emit 到前端 (completed 2026-03-18)
- [x] **Phase 28: 流式 SSE Token 提取** — 三协议 SSE 流完全结束后提取完整 token 用量 (completed 2026-03-18)
- [x] **Phase 29: 前端流量监控页面** — 独立 TrafficPage，实时日志表格 + Provider 筛选 + 统计摘要卡片 (completed 2026-03-18)
- [ ] **Phase 30: 统计聚合与数据保留** — rollup_and_prune 定时任务，按 Provider/时间聚合表格，趋势图表

## Phase Details

### Phase 26: SQLite 基础设施
**Goal**: traffic.db 可在正确路径（非 iCloud）安全初始化，schema 和连接管理就绪，后续所有读写操作有稳固基础
**Depends on**: Phase 25 (v2.5 完成)
**Requirements**: STORE-01, STORE-02
**Success Criteria** (what must be TRUE):
  1. 应用启动后 `~/Library/Application Support/` 下出现 `traffic.db` 文件，路径不含 iCloud/Mobile Documents
  2. traffic.db 以 WAL 模式运行，`request_logs` 和 `daily_rollups` 两张表及索引均已创建
  3. schema 版本通过 rusqlite_migration user_version pragma 追踪，重复启动不会报错或重建表
**Plans:** 1/1 plans complete
Plans:
- [ ] 26-01-PLAN.md — SQLite 基础设施（依赖安装 + traffic 模块 + schema + 连接管理 + lib.rs 集成）

### Phase 27: 日志写入管道
**Goal**: 每个代理请求完成后，非阻塞地将元数据（含非流式 token 用量、错误信息）写入 SQLite 并实时推送到前端
**Depends on**: Phase 26
**Requirements**: STORE-03, COLLECT-01, COLLECT-02, COLLECT-04, LOG-01
**Success Criteria** (what must be TRUE):
  1. 发出一个非流式代理请求后，SQLite `request_logs` 表中出现对应记录（含时间戳、CLI、Provider、状态码、耗时、模型名）
  2. 非流式响应的 input/output token 字段在日志记录中有正确数值（非 0、非 null）
  3. 请求失败时日志记录含 error_message 字段；请求成功时含 stop_reason 字段
  4. SQLite 写入通过 mpsc channel 在后台 task 执行，不增加代理请求响应延迟
  5. 写入完成后前端可通过 `traffic-log` Tauri 事件接收到该日志条目
**Plans:** 2/2 plans complete
Plans:
- [ ] 27-01-PLAN.md — 数据结构 + 写入管道 + 状态扩展（LogEntry、log_worker、mpsc channel、UpstreamTarget/ProxyState 扩展）
- [ ] 27-02-PLAN.md — handler 埋点 + 非流式 token 提取（三协议 token 提取、计时、日志发送）

### Phase 28: 流式 SSE Token 提取
**Goal**: 三种协议的流式 SSE 请求在 stream 完全结束后，token 用量被正确提取并写入日志
**Depends on**: Phase 27
**Requirements**: COLLECT-03
**Success Criteria** (what must be TRUE):
  1. 发出 Anthropic 协议流式请求后，日志记录中 input_tokens 和 output_tokens 均有正确数值
  2. 发出 OpenAI Chat Completions 协议流式请求后，日志记录中 token 字段有正确数值
  3. 发出 OpenAI Responses API 协议流式请求后，日志记录中 token 字段有正确数值
  4. 流式请求的 token 数值仅在 stream EOF 后写入，中途日志记录不出现部分/错误 token 值
**Plans:** 2/2 plans complete
Plans:
- [ ] 28-01-PLAN.md — 基础设施（StreamTokenData + update_streaming_log + ProxyState.app_handle 传递链路）
- [ ] 28-02-PLAN.md — 三协议流函数 oneshot 回传 + handler 后台 task UPDATE/emit

### Phase 29: 前端流量监控页面
**Goal**: 用户可在独立的流量监控页面实时查看代理请求日志，支持 Provider 筛选，并看到全局统计摘要
**Depends on**: Phase 27
**Requirements**: LOG-02, LOG-03, STAT-01
**Success Criteria** (what must be TRUE):
  1. 顶部导航栏出现"流量"入口，点击进入独立 TrafficPage，与 Providers 和 Settings 页面并列
  2. TrafficPage 展示实时日志表格，每行包含时间、Provider、模型、状态码、token 用量（in/out）、耗时列
  3. 新请求完成时，日志条目无需刷新自动追加到表格顶部
  4. Provider 筛选下拉框可选择单个 Provider，表格即时过滤只显示该 Provider 的日志
  5. 页面顶部统计摘要卡片展示：总请求数、总 input token、总 output token、成功率
**Plans:** 2/2 plans complete
Plans:
- [ ] 29-01-PLAN.md — 基础设施（类型定义 + Tauri 封装 + useTrafficLogs hook + 导航层扩展 + i18n key）
- [ ] 29-02-PLAN.md — UI 组件（格式化工具 + 统计卡片 + Provider 筛选 + 日志表格 + 空状态 + 整合）

### Phase 30: 统计聚合与数据保留
**Goal**: 历史统计数据按 Provider 和时间维度聚合可查，超期明细自动清理不占用磁盘，趋势图表可视化流量变化
**Depends on**: Phase 29
**Requirements**: STORE-04, STAT-02, STAT-03, STAT-04
**Success Criteria** (what must be TRUE):
  1. 统计页（或 Tab）展示按 Provider 聚合表格，列出各 Provider 的请求数、input/output token 合计、平均耗时
  2. 统计页展示按时间聚合表格，支持按小时或按天切换，展示对应粒度的请求数和 token 量
  3. 趋势图表以折线图或柱状图可视化时间维度的请求量和 token 变化
  4. 超过 24 小时的明细记录被聚合入 daily_rollups 后从 request_logs 删除，磁盘不无限增长
  5. 应用启动时及每小时自动触发一次 rollup_and_prune，无需用户手动操作
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
| 24. Claude settings overlay end-to-end | v2.5 | 4/4 | Complete | 2026-03-17 |
| 25. 测试覆盖 | v2.5 | 1/1 | Complete | 2026-03-17 |
| 26. SQLite 基础设施 | v2.6 | 1/1 | Complete | 2026-03-18 |
| 27. 日志写入管道 | v2.6 | 2/2 | Complete | 2026-03-18 |
| 28. 流式 SSE Token 提取 | v2.6 | 2/2 | Complete | 2026-03-18 |
| 29. 前端流量监控页面 | 2/2 | Complete    | 2026-03-18 | - |
| 30. 统计聚合与数据保留 | v2.6 | 0/TBD | Not started | - |

---
*Roadmap created: 2026-03-12 (v1.0)*
*Last updated: 2026-03-18 — Phase 29 planned (2 plans)*
