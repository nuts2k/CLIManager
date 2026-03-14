# 更新日志

所有重要变更都会记录在这个文件中。

格式基于 [Conventional Commits](https://www.conventionalcommits.org/)。

---

## v0.2.2 (2026-03-14)

### 修复
- 修复 CI 使用 tauri-action@v0（v1 不存在）

## v0.2.1 (2026-03-14)

### 新功能
- 创建 GitHub Actions release CI/CD 流水线
- 添加 useUpdater hook、UpdateDialog、AboutSection 组件
- 集成 UpdateDialog 到 AppShell 和 SettingsPage
- 创建 /ship 一键发版技能
- 生成 Ed25519 密钥并写入公钥到 tauri.conf.json

### 修复
- 移除 AboutSection 中不可达的 disabled 检查

### 其他
- 版本来源统一 + updater/process 插件依赖与注册
- 创建初始 CHANGELOG.md
- auto-publish release (releaseDraft false)
