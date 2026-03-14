# Roadmap: CLIManager

## Milestones

- ✅ **v1.0 MVP** — Phases 1-5 (shipped 2026-03-12)
- ✅ **v1.1 System Tray** — Phases 6-7 (shipped 2026-03-13)
- ✅ **v2.0 Local Proxy** — Phases 8-11 (shipped 2026-03-14)
- 🚧 **v2.1 Release Engineering** — Phases 12-13 (in progress)

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

### 🚧 v2.1 Release Engineering (In Progress)

**Milestone Goal:** 建立完整的构建、签名、分发和自动更新流程，让 app 能实际安装使用并持续迭代

- [ ] **Phase 12: 全栈实现** — 密钥配置 + CI/CD + Updater + 发版脚本，wave 并行
- [ ] **Phase 13: 端到端验证** — 完整发版流程验证，确保闭环可用

## Phase Details

### Phase 12: 全栈实现
**Goal**: 完成所有代码和配置变更：CI 流水线、签名、updater 集成、发版脚本、Gatekeeper 文档
**Depends on**: Phase 11 (v2.0 已交付完整 app)
**Requirements**: REL-01, SIGN-02, SIGN-03, SIGN-01, CICD-01, CICD-02, CICD-03, UPD-01, UPD-02, UPD-03, UPD-04, REL-02, REL-03
**Plans:** 4 plans
**Success Criteria** (what must be TRUE):
  1. Cargo.toml 是版本号唯一来源，tauri.conf.json 无独立 version 字段
  2. updater Ed25519 密钥对已生成，私钥存入 GitHub Secrets 并已备份，公钥写入 tauri.conf.json
  3. 推送 `v*.*.*` 格式 tag 后 GitHub Actions 自动触发，产出双架构 ad-hoc 签名 DMG
  4. 构建产物（DMG + .app.tar.gz + .sig + latest.json）自动上传到 GitHub Release Draft
  5. tauri-plugin-updater + tauri-plugin-process 已集成，app 启动时检查 latest.json
  6. 自定义 React 更新 UI 显示进度条，用户可选立即安装或稍后提醒
  7. 发版脚本一条命令完成 bump → CHANGELOG → commit → tag → push
  8. GitHub Release Notes 模板包含 Gatekeeper 安装指引

**Parallelism**: Wave 1 → Wave 2 (3 路并行)

Plans:
- [ ] 12-01-PLAN.md — 密钥与配置基础 [REL-01, SIGN-02, SIGN-03] — Wave 1
- [ ] 12-02-PLAN.md — CI/CD 流水线 [CICD-01, CICD-02, CICD-03, SIGN-01, REL-03] — Wave 2
- [ ] 12-03-PLAN.md — Updater 插件与自定义 UI [UPD-01, UPD-02, UPD-03, UPD-04] — Wave 2
- [ ] 12-04-PLAN.md — 发版脚本与用户引导 [REL-02, REL-03] — Wave 2

### Phase 13: 端到端验证
**Goal**: 完整发版流程端到端验证：release script → CI build → updater check → download → install → relaunch
**Depends on**: Phase 12 (所有代码和配置变更完成)
**Requirements**: (验证 Phase 12 所有需求的集成)
**Success Criteria** (what must be TRUE):
  1. 发版脚本创建的 tag 成功触发 CI，双架构 DMG 出现在 GitHub Release Draft
  2. Publish Release 后，已安装的旧版 app 启动时检测到新版本并弹出更新提示
  3. 用户确认更新后，下载、签名验证、安装、重启全部自动完成，新版本正常运行
  4. aarch64 和 x86_64 两个架构均端到端验证通过

Plans:
- [ ] 13-01: 端到端发版与更新验证

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
| 12. 全栈实现 | v2.1 | 0/4 | Planning complete | - |
| 13. 端到端验证 | v2.1 | 0/1 | Not started | - |

---
*Roadmap created: 2026-03-12 (v1.0)*
*Last updated: 2026-03-14 — Phase 12 plans created (4 plans, 2 waves)*
