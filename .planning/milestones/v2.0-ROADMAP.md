# Roadmap: CLIManager

## Milestones

- ✅ **v1.0 MVP** — Phases 1-5 (shipped 2026-03-12)
- ✅ **v1.1 System Tray** — Phases 6-7 (shipped 2026-03-13)
- 📋 **v2.0 Local Proxy** — Phases 8-11 (planned)

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

### v2.0 Local Proxy

**Milestone Goal:** 在应用内运行本地代理服务，CLI 指向 localhost 实现实时 Provider 切换，无需修改配置文件或重启 CLI。

- [x] **Phase 8: 代理核心** - axum HTTP 代理服务器，支持请求转发、SSE 流式透传、动态上游切换 (completed 2026-03-13)
- [ ] **Phase 9: 模式切换与持久化** - 直连 vs 代理双模式切换，CLI 配置联动 patch，崩溃恢复，设置持久化
- [x] **Phase 10: 实时切换与 UI 集成** - Provider 变更实时更新代理内存，前端开关控件，端口冲突检测 (completed 2026-03-14)
- [x] **Phase 11: 代理感知修复与文档同步** - 托盘/编辑路径代理感知修复，审计差距文档同步 (completed 2026-03-14)

## Phase Details

### Phase 8: 代理核心
**Goal**: 每个 CLI 拥有独立端口的本地 HTTP 代理服务器，能将请求转发到上游 Provider 并支持 SSE 流式响应
**Depends on**: Phase 7 (v1.1 完成)
**Requirements**: PROXY-01, PROXY-02, PROXY-03, PROXY-04, PROXY-05, UX-03
**Success Criteria** (what must be TRUE):
  1. CLI 发出的 API 请求经过本地代理成功到达上游 Provider 并返回正确响应
  2. SSE 流式响应逐 chunk 实时透传到 CLI，无缓冲延迟
  3. 代理自动将请求中的占位 API key 替换为当前活跃 Provider 的真实凭据后转发
  4. Claude Code (15800) 和 Codex (15801) 各自监听独立固定端口
  5. 上游不可达时 CLI 收到 502 + JSON 结构化错误（而非 connection refused）
**Plans**: 2 plans

Plans:
- [x] 08-01-PLAN.md — 代理引擎核心（依赖更新 + 类型/错误/状态 + handler 转发 + server 生命周期）
- [x] 08-02-PLAN.md — ProxyService 多端口管理器 + Tauri 命令层集成

### Phase 9: 模式切换与持久化
**Goal**: 用户可在直连模式和代理模式间安全切换，切换时 CLI 配置自动联动，应用退出或崩溃后状态正确恢复
**Depends on**: Phase 8
**Requirements**: MODE-01, MODE-02, MODE-03, MODE-04, MODE-05, MODE-06, LIVE-04, UX-02
**Success Criteria** (what must be TRUE):
  1. 用户可通过全局总开关和每 CLI 独立开关控制代理模式的开启/关闭
  2. 开启代理模式时 CLI 配置自动 patch 为 localhost:port + 占位 key，关闭时自动还原为真实凭据
  3. 应用正常退出时所有已代理的 CLI 配置被还原为直连状态
  4. 应用异常崩溃后重启时自动检测 takeover 标志并还原 CLI 配置
  5. 代理开关状态持久化到本地设备层，重启后自动恢复之前的代理状态
**Plans**: 2 plans

Plans:
- [ ] 09-01-PLAN.md — 模式切换后端核心（LocalSettings 扩展 + 端口常量 + 四个切换命令 + set_active_provider 改造）
- [ ] 09-02-PLAN.md — 退出清理 + 崩溃恢复 + 启动自动恢复

### Phase 10: 实时切换与 UI 集成
**Goal**: 代理模式下切换 Provider 对 CLI 完全透明且即时生效，用户通过前端 UI 控制所有代理相关设置
**Depends on**: Phase 9
**Requirements**: LIVE-01, LIVE-02, LIVE-03, UX-01
**Success Criteria** (what must be TRUE):
  1. 代理模式下切换 Provider 后，下一个 CLI 请求立即使用新 Provider（无需重启 CLI）
  2. iCloud 同步的 Provider 内容变更和本地 CRUD 操作自动更新代理内存中的上游目标
  3. 启动代理时端口被占用能给出清晰错误提示而非静默失败
  4. 设置页全局开关和 Tab 内独立开关联动正确，UI 状态与后端一致
**Plans**: 2 plans

Plans:
- [ ] 10-01-PLAN.md — 后端代理联动（watcher iCloud 同步 + update_provider/delete_provider 代理模式感知）
- [ ] 10-02-PLAN.md — 前端 UI 集成（Switch 开关 + 状态指示 + 端口占用 toast + i18n）

### Phase 11: 代理感知修复与文档同步
**Goal**: 修复托盘菜单切换和 Provider 编辑路径的代理感知缺失，同步审计发现的文档差距
**Depends on**: Phase 10
**Requirements**: LIVE-01 (integration fix), LIVE-03 (integration fix), UX-01 (doc sync)
**Gap Closure:** Closes gaps from v2.0 milestone audit
**Success Criteria** (what must be TRUE):
  1. 托盘菜单切换 Provider 时，代理模式下跳过 adapter.patch()，仅更新 active_providers 和代理上游
  2. 编辑活跃 Provider 时，代理模式下跳过 patch_provider_for_cli，仅保存文件并更新代理上游
  3. REQUIREMENTS.md UX-01 复选框标记为完成，10-02-SUMMARY.md 包含 requirements-completed 字段
**Plans**: 1 plan

Plans:
- [ ] 11-01-PLAN.md — 托盘/编辑代理感知修复 + UX-01 文档同步

## Progress

**Execution Order:** Phase 8 -> Phase 9 -> Phase 10 -> Phase 11

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
| 9. 模式切换与持久化 | 1/2 | In Progress|  | - |
| 10. 实时切换与 UI 集成 | 2/2 | Complete    | 2026-03-14 | - |
| 11. 代理感知修复与文档同步 | 1/1 | Complete    | 2026-03-14 | - |

---
*Roadmap created: 2026-03-12 (v1.0)*
*Last updated: 2026-03-14 — Phase 11 plan created*
