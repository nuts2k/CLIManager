# Phase 30: 统计聚合与数据保留 - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

历史统计数据按 Provider 和时间维度聚合可查，超期明细自动清理不占用磁盘，趋势图表可视化流量变化。包含：rollup_and_prune 定时任务、按 Provider 聚合表格（供应商排行榜 + 缓存命中率排行榜）、按时间聚合趋势图表、数据保留策略执行。

</domain>

<decisions>
## Implementation Decisions

### 统计视图布局
- TrafficPage 内部新增 Tab 切换：「实时日志」和「统计分析」两个 Tab
- Tab 样式使用 line 下划线风格（与 Settings 页 Tab 视觉统一）
- 默认进入「实时日志」Tab（现有功能），用户可切换到「统计分析」Tab
- 现有 5 张统计摘要卡片只在「实时日志」Tab 显示，「统计分析」Tab 不显示卡片

### 统计分析 Tab 内部布局
- 顶部：24h / 7d Segment 按钮组（控制所有统计数据的时间范围）
- 上半区：左右并排两个排行榜表格（供应商排行榜 + 缓存命中率排行榜）
- 下半区：全宽趋势图表

### 趋势图表
- 使用 recharts 库实现（React 生态成熟方案，REQUIREMENTS 已提及）
- 双轴图表：左 Y 轴请求数（柱状图），右 Y 轴 Token 总量（折线图）
- 时间粒度跟随 24h/7d 切换：24h 模式显示 24 个小时点，7d 模式显示 7 个天点
- 24h 数据从 request_logs 按小时聚合查询，7d 数据从 daily_rollups 按天查询

### 供应商排行榜
- 列：Provider、请求数、Token（in/out 合并一列）、成功率、平均 TTFB、平均 TPS
- 点击表头列名切换升/降序排序，默认按请求数降序
- Token 列格式：in: 1.2k / out: 3.4k（与实时日志表格 Token 列风格一致）
- 支持 24h 和 7d 两个时间维度（跟随全局 Segment 切换）

### 缓存命中率排行榜
- 列（Phase 26 定义）：缓存触发请求数、缓存命中率、缓存读取 token 数、总 token 数
- 点击表头排序，默认按命中率降序
- 支持 24h 和 7d 两个时间维度

### 24h/7d 时间维度切换
- Segment 按钮组，位于统计分析 Tab 标题旁
- 切换后联动更新：两个排行榜表格 + 趋势图表
- 24h 数据源：request_logs 实时查询
- 7d 数据源：daily_rollups 聚合查询

### rollup_and_prune 定时任务
- 触发时机：应用启动时立即执行一次 + tokio::interval(1h) 定时重复
- 完全静默执行，用户无感知，成功/失败只记 log::info/warn
- 单次事务内原子操作：INSERT/UPDATE daily_rollups → DELETE 超 24h 明细
- 同时删除超 7d 的 daily_rollups 记录
- 失败不重试，等下一轮自动触发

### Claude's Discretion
- recharts 具体版本和图表样式细节（颜色、tooltip、动画等）
- Segment 按钮组的具体样式实现（shadcn Tabs 或自定义）
- 排行榜表格为空时的空状态设计
- 后端聚合查询的 SQL 优化和接口设计
- rollup_and_prune 的 SQL 事务实现细节
- 趋势图表的暗色主题适配细节

</decisions>

<specifics>
## Specific Ideas

- Phase 26 已详细定义 daily_rollups schema（13 列，10 个聚合字段），rollup SQL 直接对齐这些字段
- Phase 26 确定的排行榜指标可从 rollup 组合计算（已验证可组合性）：平均 TPS = SUM(output_tokens) / (SUM(duration_ms) - SUM(ttfb_ms)) * 1000
- 趋势图表的双轴设计（请求量柱状 + Token 折线）可以直观看出请求频率和 Token 消耗的关系
- 统计分析 Tab 的排行榜左右分栏设计充分利用横向空间，避免纵向过长需要滚动

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TrafficPage.tsx`: 现有页面结构，需扩展为 Tab 切换
- `TrafficStatsBar.tsx`: 5 张统计卡片，保留在实时日志 Tab
- `TrafficTable.tsx`: 实时日志表格，保留在实时日志 Tab
- `TrafficFilter.tsx`: Provider 筛选下拉框，保留在实时日志 Tab
- `traffic/log.rs::TrafficDb`: 已有 insert/update/query_recent_logs 方法，需新增聚合查询方法
- `traffic/schema.rs`: daily_rollups 表已创建（13 列：id + provider_name + rollup_date + 10 聚合字段）
- `formatters.ts`: formatTokenCount 等格式化函数可复用
- SVG Sparkline 组件（TrafficStatsBar 内）: 卡片趋势线，与新的 recharts 图表独立

### Established Patterns
- Tauri `.manage()` 注入 + `try_state::<TrafficDb>()` 安全访问
- Tauri command: `#[tauri::command]` + `invoke()` 前后端通信
- Settings 页 line 下划线 Tab 风格: 可参考复用到 TrafficPage Tab
- div-based grid 布局: Phase 29 TrafficTable 已使用，排行榜可沿用
- i18n: 所有 UI 文字通过 useTranslation() 国际化
- CSS 变量暗色主题体系

### Integration Points
- `TrafficPage.tsx`: 重构为 Tab 容器，内含两个 Tab 面板
- `traffic/mod.rs` 或新文件: 新增 rollup_and_prune() 方法 + 聚合查询方法
- `lib.rs` setup 闭包: 启动 rollup_and_prune 后台 task（tokio::spawn + interval）
- `Cargo.toml`: 无需新增依赖（rusqlite/tokio 已有）
- `package.json`: 需新增 recharts 依赖
- `i18n/`: 新增统计分析 Tab 相关翻译 key

</code_context>

<deferred>
## Deferred Ideas

- 费用估算 (cost_usd) -- v2.7+ (ADV-01)
- 实时告警与阈值配置 -- v2.7+ (ADV-02)
- 导出报表 (JSON/CSV) -- v2.7+ (ADV-03)
- 保留时长用户可配置（当前硬编码 24h/7d）-- v2.7+

</deferred>

---

*Phase: 30-stats-rollup*
*Context gathered: 2026-03-18*
