# Requirements: CLIManager

**Defined:** 2026-03-15
**Core Value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容

## v2.3 Requirements

Requirements for v2.3 前端调整及美化。Each maps to roadmap phases.

### 首页布局

- [x] **HOME-01**: Provider 卡片的编辑、测试、删除等操作从三点菜单外露为可见图标按钮
- [x] **HOME-02**: Provider 卡片 hover 时有微升起效果（shadow + border 变化过渡）
- [x] **HOME-03**: 无 Provider 时的空状态页面更精致（视觉和文案优化）
- [x] **HOME-04**: 代理模式开关和状态指示（绿色圆点）视觉更突出明确

### Provider 编辑

- [x] **EDIT-01**: Provider 编辑 Dialog 加宽并支持内容区域滚动
- [x] **EDIT-02**: 编辑表单字段分组优化（基础信息 / 协议设置 / 模型配置 分区）
- [x] **EDIT-03**: 表单验证错误提示更友好、字段说明更清晰

### 设置页

- [x] **SETT-01**: 设置页改为 Tab 布局（通用 / 高级 / 关于）

### 视觉风格

- [x] **VISU-01**: 使用 CSS 变量统一全局配色方案（橙色强调色融入，无硬编码色值）
- [x] **VISU-02**: 可交互元素加入 hover/切换/加载微动效过渡（150-300ms）
- [x] **VISU-03**: 全局间距和圆角规范统一
- [x] **VISU-04**: Header 导航栏视觉提升，品牌感更强

### 图标

- [ ] **ICON-01**: 全新设计应用图标（生成全套 icns/ico/png 尺寸）
- [ ] **ICON-02**: 托盘图标从应用图标派生（轮廓/简化版 template 图标），视觉统一

## Future Requirements

（无 — v2.3 聚焦前端）

## Out of Scope

| Feature | Reason |
|---------|--------|
| 亮色/暗色主题切换 | v2.3 聚焦暗色精调，双主题后续考虑 |
| 首页双栏/表格视图 | 保持卡片列表形式，只优化交互 |
| 多语言新增（日文等） | v2.3 不新增语言，只优化现有中英文 UI |
| Provider 编辑改为独立页面 | 保持 Dialog 形式，加宽+可滚动即可 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| HOME-01 | Phase 18 | Complete |
| HOME-02 | Phase 18 | Complete |
| HOME-03 | Phase 18 | Complete |
| HOME-04 | Phase 18 | Complete |
| EDIT-01 | Phase 19 | Complete |
| EDIT-02 | Phase 19 | Complete |
| EDIT-03 | Phase 19 | Complete |
| SETT-01 | Phase 20 | Complete |
| VISU-01 | Phase 17 | Complete |
| VISU-02 | Phase 21 | Complete |
| VISU-03 | Phase 17 | Complete |
| VISU-04 | Phase 21 | Complete |
| ICON-01 | Phase 22 | Pending |
| ICON-02 | Phase 22 | Pending |

**Coverage:**
- v2.3 requirements: 14 total
- Mapped to phases: 14
- Unmapped: 0

---
*Requirements defined: 2026-03-15*
*Last updated: 2026-03-15 after v2.3 roadmap creation (Phases 17-22)*
