# 更新日志

所有重要变更都会记录在这个文件中。

格式基于 [Conventional Commits](https://www.conventionalcommits.org/)。

---

## v0.2.3 (2026-03-15)

### 新功能
- CSS 变量全局配色体系（品牌橙色 oklch + 语义色命名）
- 间距阶梯变量 + Card 组件圆角统一为 rounded-lg
- 替换全部业务组件硬编码颜色为 CSS 变量引用
- ProviderCard 操作按钮外露为图标按钮 + hover 升起效果
- 空状态页面精致化（品牌橙色装饰 + 优化文案）
- 代理状态指示优化（绿点加大 + 脉冲 + 开关旁指示圆点）
- ProviderDialog 加宽至 640px + 三分区平铺表单 + 固定 Header/Footer 滚动
- 字段 placeholder 国际化 + 验证错误红色边框
- SettingsPage 重构为三 Tab 布局（通用/高级/关于）+ 关于页 Logo
- Header 品牌视觉提升 + --header-bg CSS 变量
- 页面切换淡入淡出过渡（150ms ease-out）
- 全新应用图标全套尺寸文件（icns/ico/png）
- 黑白轮廓托盘 template 图标
- Anthropic → OpenAI Chat Completions 双向协议转换（请求/响应/流式 SSE）
- Responses API 完整转换层（请求/非流式响应/流式 SSE）
- handler.rs 三分支协议路由 + 模型映射函数
- Provider UI 三协议选择 + 模型映射配置
- Provider 数据模型扩展（ProtocolType 三变体 + upstream 映射字段）

### 修复
- 修复页面切换时隐藏页仍可交互
- 图标加透明边距，四角透明，Dock 大小与其他图标一致
- 图标背景改为圆角矩形（rx=228），适配 dev 模式 Dock 显示
- 对话框宽度从 576px 改为 640px 满足成功标准
- 支持 OpenAI base_url path prefixes
- 完善 OpenAI Responses 兼容与测试模型配置
- 限制协议转换仅对 messages 请求生效
- test_provider 使用 upstream_model 并清理诊断日志
- migrate legacy test_model defaults
- 修复 updater 设置页更新交互与重启失败处理

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
