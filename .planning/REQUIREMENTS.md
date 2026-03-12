# Requirements: CLIManager

**Defined:** 2026-03-12
**Core Value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容

## v1.1 Requirements

Requirements for v1.1 System Tray milestone. Each maps to roadmap phases.

### 托盘基础 (TRAY)

- [ ] **TRAY-01**: 应用启动后在 macOS 菜单栏显示托盘图标（模板图标，自适应暗色/亮色模式）
- [ ] **TRAY-02**: 关闭主窗口时应用不退出，隐藏窗口并驻留在托盘
- [ ] **TRAY-03**: 窗口隐藏时切换为 Accessory 模式（不显示在 Dock 和 Cmd+Tab），窗口显示时恢复 Regular 模式

### Provider 切换 (PROV)

- [ ] **PROV-01**: 托盘菜单按 CLI 分组显示所有 Provider，当前激活的 Provider 显示勾选标记
- [ ] **PROV-02**: 点击托盘菜单中的 Provider 即可一键切换，无需打开主窗口
- [ ] **PROV-03**: 主窗口中 Provider 增删改或 iCloud 同步变化后，托盘菜单自动刷新

### 菜单项 (MENU)

- [ ] **MENU-01**: 托盘菜单包含"打开主窗口"选项，点击后显示并聚焦主窗口
- [ ] **MENU-02**: 托盘菜单包含"退出"选项，点击后完全退出应用
- [ ] **MENU-03**: 托盘菜单文字跟随应用语言设置（中/英）

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### 托盘增强

- **TRAY-04**: 托盘图标 tooltip 显示当前激活 Provider 名称
- **TRAY-05**: 托盘图标根据是否有激活 Provider 显示不同状态（正常/暗淡）
- **PROV-04**: Provider 菜单项显示模型名称用于区分同名 Provider

### 快捷键

- **KEY-01**: 全局快捷键打开托盘菜单或触发 Provider 切换

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| 托盘内 Provider 增删改 | 原生菜单无法承载表单/验证，管理操作留在主窗口 |
| 嵌套子菜单按 CLI 分组 | 扁平列表+分区标题更快捷，Provider 数量少（2-5 个/CLI）不需要子菜单 |
| 切换成功通知 | 通知疲劳，CheckMenuItem 勾选移动已足够反馈 |
| 动态生成托盘图标（显示 Provider 首字母） | 需要 Core Graphics 运行时渲染，与其他托盘图标风格不一致 |
| 网络/位置自动切换 | 巨大范围，手动一键切换足够 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| TRAY-01 | Phase 6 | Pending |
| TRAY-02 | Phase 6 | Pending |
| TRAY-03 | Phase 6 | Pending |
| PROV-01 | Phase 7 | Pending |
| PROV-02 | Phase 7 | Pending |
| PROV-03 | Phase 7 | Pending |
| MENU-01 | Phase 6 | Pending |
| MENU-02 | Phase 6 | Pending |
| MENU-03 | Phase 7 | Pending |

**Coverage:**
- v1.1 requirements: 9 total
- Mapped to phases: 9
- Unmapped: 0

---
*Requirements defined: 2026-03-12*
*Last updated: 2026-03-12 after roadmap creation (traceability added)*
