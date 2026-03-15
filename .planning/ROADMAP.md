# Roadmap: CLIManager

## Milestones

- ✅ **v1.0 MVP** — Phases 1-5 (shipped 2026-03-12)
- ✅ **v1.1 System Tray** — Phases 6-7 (shipped 2026-03-13)
- ✅ **v2.0 Local Proxy** — Phases 8-11 (shipped 2026-03-14)
- ✅ **v2.1 Release Engineering** — Phases 12-13 (shipped 2026-03-14)
- ✅ **v2.2 协议转换** — Phases 14-16 (shipped 2026-03-15)
- 🚧 **v2.3 前端调整及美化** — Phases 17-22 (in progress)

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

### 🚧 v2.3 前端调整及美化 (In Progress)

**Milestone Goal:** 全面提升前端交互体验、视觉质感和设计一致性

- [x] **Phase 17: 设计基础** - 建立 CSS 变量体系、统一间距与圆角规范 (completed 2026-03-15)
- [x] **Phase 18: 首页布局优化** - Provider 卡片操作外露、hover 效果、空状态和代理状态视觉优化 (completed 2026-03-15)
- [ ] **Phase 19: Provider 编辑改进** - 对话框加宽可滚动、表单字段分组、验证提示优化
- [ ] **Phase 20: 设置页 Tab 化** - 通用/高级/关于三 Tab 分组布局
- [ ] **Phase 21: 微动效与 Header 提升** - 全局微动效过渡和 Header 品牌视觉优化
- [ ] **Phase 22: 应用图标** - 全新应用图标设计及托盘图标派生

## Phase Details

### Phase 17: 设计基础
**Goal**: 全局配色、间距和圆角体系建立完毕，所有后续视觉工作有统一的设计 token 可用
**Depends on**: Phase 16
**Requirements**: VISU-01, VISU-03
**Success Criteria** (what must be TRUE):
  1. 橙色强调色（#F97316）通过 CSS 变量引用，全局无硬编码颜色值
  2. 暗色背景、卡片、边框、文字各层级颜色均通过 CSS 变量定义且在整个 UI 中一致
  3. 组件间距遵循统一的间距阶梯（如 4/8/12/16/24px），无随意 px 值
  4. 圆角规范统一（如 rounded-md/rounded-lg），所有卡片和对话框视觉一致
**Plans**: 2 plans
Plans:
- [x] 17-01-PLAN.md — CSS 变量配色体系：品牌色 + 语义色定义，替换硬编码颜色
- [x] 17-02-PLAN.md — 间距阶梯与圆角规范统一，审计全部组件 + 视觉验证

### Phase 18: 首页布局优化
**Goal**: 首页 Provider 卡片操作直观可发现，空状态精致，代理状态指示清晰
**Depends on**: Phase 17
**Requirements**: HOME-01, HOME-02, HOME-03, HOME-04
**Success Criteria** (what must be TRUE):
  1. 编辑、测试、删除按钮在卡片上直接可见，无需展开三点菜单即可点击
  2. 鼠标悬停 Provider 卡片时，卡片有可见的阴影加深和边框颜色变化过渡动效
  3. 无 Provider 时页面展示精致的空状态图示和引导文案，非空白或文字列表
  4. 代理模式开关旁的状态圆点在启用时为绿色且明显，停用时清晰区分
**Plans**: 2 plans
Plans:
- [ ] 18-01-PLAN.md — 卡片操作按钮外露 + hover 升起效果
- [ ] 18-02-PLAN.md — 空状态精致化 + 代理状态指示优化

### Phase 19: Provider 编辑改进
**Goal**: Provider 编辑对话框宽敞易用，字段分组清晰，验证反馈明确
**Depends on**: Phase 17
**Requirements**: EDIT-01, EDIT-02, EDIT-03
**Success Criteria** (what must be TRUE):
  1. 编辑对话框宽度明显大于当前版本（至少 600px），长表单内容区域可纵向滚动
  2. 表单字段分为"基础信息"、"协议设置"、"模型配置"三个视觉分区，分区之间有明确分隔
  3. 必填字段验证失败时，错误提示文字清晰显示在对应字段下方，不依赖通用 toast
  4. 字段标签或说明文字对非技术用户友好，关键字段有 placeholder 或说明提示
**Plans**: TBD

### Phase 20: 设置页 Tab 化
**Goal**: 设置页内容按功能分组到三个 Tab，不再是一个滚动长页
**Depends on**: Phase 17
**Requirements**: SETT-01
**Success Criteria** (what must be TRUE):
  1. 设置页顶部显示"通用"、"高级"、"关于"三个 Tab 选项，点击可切换
  2. 各 Tab 内容独立：通用含语言/主题等全局设置，高级含代理端口等技术配置，关于含版本和更新
  3. 刷新页面或重新打开设置后，上次选中的 Tab 保持（或默认停留"通用"）
**Plans**: TBD

### Phase 21: 微动效与 Header 提升
**Goal**: 可交互元素有流畅的过渡动效，Header 视觉品牌感更强
**Depends on**: Phase 18
**Requirements**: VISU-02, VISU-04
**Success Criteria** (what must be TRUE):
  1. 按钮、开关、卡片的 hover/active/loading 状态切换有 150-300ms 的平滑过渡，无跳变
  2. 模式切换（直连/代理）和 Provider 激活切换有可见的状态过渡动效
  3. Header 导航栏带有应用 Logo/名称标识，视觉层次比内容区域更突出
  4. 整体 UI 在视觉上感觉现代、精致，操作反馈即时且流畅
**Plans**: TBD

### Phase 22: 应用图标
**Goal**: 应用有专属的全新图标，托盘图标与应用图标视觉统一
**Depends on**: Phase 17
**Requirements**: ICON-01, ICON-02
**Success Criteria** (what must be TRUE):
  1. 应用图标在 Finder/Dock 中显示新设计，替换旧图标，全套尺寸（16/32/128/256/512px）完整
  2. 托盘图标为应用图标的简化/轮廓版本，在 macOS 菜单栏中清晰可辨
  3. 托盘图标为 template 图标（黑白），在浅色和深色菜单栏下均正常显示
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
| 18. 首页布局优化 | 2/2 | Complete   | 2026-03-15 | - |
| 19. Provider 编辑改进 | v2.3 | 0/TBD | Not started | - |
| 20. 设置页 Tab 化 | v2.3 | 0/TBD | Not started | - |
| 21. 微动效与 Header 提升 | v2.3 | 0/TBD | Not started | - |
| 22. 应用图标 | v2.3 | 0/TBD | Not started | - |

---
*Roadmap created: 2026-03-12 (v1.0)*
*Last updated: 2026-03-15 — Phase 18 planned (2 plans)*
