# 更新日志

所有重要变更都会记录在这个文件中。

格式基于 [Conventional Commits](https://www.conventionalcommits.org/)。

---

## v0.2.10 (2026-03-19)

### 修复
- 修复 Cargo.toml 版本号遗漏，规范发版流程

## v0.2.9 (2026-03-19)

### 新功能
- 美化实时日志表格，自定义暗色主题精细滚动条替代原生滚动条
- 请求进行中显示旋转图标+脉冲左边框，区分等待首字节与流式接收状态
- 已收到首字节时展示 TTFB，让用户直观确认连接已建立

## v0.2.8 (2026-03-19)

### 修复
- 修复 v0.2.7 版本号未正确写入导致客户端无法检测更新

## v0.2.7 (2026-03-19)

### 修复
- hide token count requests from logs and stats
- read final anthropic usage from message_delta
- avoid Claude settings write races

### 文档
- create phase plan
- add validation strategy
- research phase SQLite 基础设施
- capture phase context

## v0.2.6 (2026-03-19)

### 新功能
- 代理模式完整流量监控：SQLite WAL 持久化日志 + 三协议 token 提取（含流式 SSE）
- mpsc channel 非阻塞日志写入管道，后台 task 异步写入不阻塞代理请求
- 三协议流式 SSE Token 提取（Anthropic/OpenAI Chat/Responses），oneshot 回传 + 后台 UPDATE
- 独立 TrafficPage 实时日志表格 + Provider 筛选 + 5 张统计摘要卡片
- rollup_and_prune 定时聚合（启动 + 每小时），24h 明细 + 7d 统计自动清理
- Provider/Cache 排行榜 + recharts ComposedChart 双轴趋势图（24h/7d 切换）
- try_state 安全 DB 访问 + dbError 内联警告 banner
- total_cache_creation_tokens 暴露到 ProviderStat API 和 CacheLeaderboard 前端

### 修复
- 修正上游模型记录与展示
- 修正 7d 趋势图日期标签时区偏移
- 修正 24h 趋势图滚动时间轴
- 补齐 7d 流量统计最近 24h 数据
- TPS 计算扣除 TTFB，反映真实 token 生成速率
- 修复流式请求 token 数据全为 null 的问题
- DB 初始化失败时 try_state 安全访问 + 前端 dbError 内联警告
- 修正 30-03-SUMMARY.md 中 i18n 文件路径记录

### 其他
- WON'T FIX 技术债务添加设计意图注释（3 项）
## v0.2.5 (2026-03-17)

### 新功能
- Settings → Advanced 新增 Claude overlay 编辑小节（JSON 多行编辑/校验/保存）
- overlay 存储层：iCloud 优先写入，不可用时本地降级，UI 显示存放位置
- json_merge 深度合并引擎：object 递归合并、array 替换、scalar 覆盖、null 删除
- ClaudeAdapter patch 集成 overlay 深度合并 + 保护字段（Provider/Proxy 凭据）永远优先
- 保存即 apply + 启动 best-effort apply + iCloud watcher 文件变更自动 apply
- startup 通知缓存回放（解决 Tauri setup 时序问题）
- Textarea 通用 UI 组件

### 修复
- 修复前端 overlay_json 与后端 content 字段名不匹配
- 修复更新重启后 Dialog overlay 遮挡设置按钮
- 保留 Claude overlay apply 的代理接管配置
- 将空 Claude overlay 视为已清空
- 支持自定义 Claude config 目录的 overlay 路径

### 测试
- 补充 json_merge 深度合并边界测试（空 overlay/嵌套 null 删除）
- 补充保护字段优先级边界测试（保护+自定义共存/序贯 patch）
- 补充 ClaudeAdapter overlay 集成边界测试（overlay+clear 交互/顶层 key 合并）

### 其他
- rustfmt 格式化及代码结构位置修复
## v0.2.4 (2026-03-15)

### 新功能
- Anthropic 分支请求模型映射 + 响应/流式反向映射
- Anthropic 协议显示模型映射 UI 且字段为可选

### 修复
- 修复 OpenAI SSE UTF-8 分块解码
- 修复 Anthropic 映射与 SSE 回归

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
