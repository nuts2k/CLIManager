# Phase 29: 前端流量监控页面 - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

独立的流量监控页面（TrafficPage），与 Providers 和 Settings 页面并列为顶级视图。展示实时日志表格（支持 Provider 筛选）和统计摘要卡片。数据来源为 Phase 27 建立的双轨机制（command 初始拉取 + traffic-log 事件增量追加）。

</domain>

<decisions>
## Implementation Decisions

### 导航入口
- Header 右侧添加 Traffic 图标按钮，与 Settings 齿轮按钮并排
- 无状态指示（无红点、无计数徽章），纯图标按钮
- AppView 扩展为 "main" | "traffic" | "settings" 三视图互斥切换
- 点击图标进入对应视图，再点一次回到 main（toggle 行为，与 Settings 按钮一致）
- 复用现有 AppShell 的 opacity + pointer-events 过渡模式（150ms），Traffic 视图始终渲染不卸载，切走后保持状态

### 日志表格列设计
- 精简 6 列：时间、Provider、模型、状态码、Token、耗时
- 单元格内多行堆叠展示详细信息：
  - Token 列：第一行 in/out（如 1.2k / 3.4k），第二行缓存信息（如 cache read 128）
  - 耗时列：第一行总耗时（如 8.2s），第二行 TTFB（如 TTFB 1.2s），第三行 tps（如 42 t/s）
- 单元格内容垂直居中对齐（无论该单元格是 1 行、2 行还是 3 行）
- 点击行展开详情区域，显示完整信息：协议类型、upstream_model、CLI、is_streaming、stop_reason、error_message 等

### 日志表格时间显示
- 1 小时内显示相对时间（xx 秒前 / xx 分前）
- 超过 1 小时显示具体时间（如 14:32:01）

### 日志表格排序与插入
- 固定按时间降序排列，不支持按列排序
- 新条目置顶插入
- 用户已滚动到下方查看历史时，不自动跳回顶部（避免打断阅读）

### 流式请求状态表现
- 流式请求 token=null / duration=null 时，Token 和耗时列显示文字占位符（"--" 或类似）
- 收到 traffic-log type="update" 事件后替换为实际数值

### 统计摘要卡片
- 位于表格上方横向排列
- 带图标的宽松卡片风格（图标 + 标签 + 大号数值 + 微小趋势线）
- 5 张卡片：请求数、Input Token、Output Token、成功率、缓存命中率
- 数据范围：滚动 24 小时（具体技术实现方式 Claude 设计，难以实现时再讨论）
- 随新日志实时更新

### Provider 筛选
- 筛选下拉框位于统计卡片和表格之间
- 选项来源：从前端内存中的日志条目 distinct provider_name 提取
- 默认"全部"，选择后表格即时过滤
- 筛选同时影响统计卡片数值（仅统计选中 Provider 的数据）

### 空状态与边界
- 代理未开启时正常显示历史日志（如果有），不阻断页面
- 无任何日志记录时显示简洁文字提示（与 Provider 空状态风格一致）
- 不使用虚拟滚动；初始加载 100 条 + 实时追加，前端内存保持最多 500 条，超出时丢弃最旧的

### Claude's Discretion
- Traffic 图标选择（lucide 图标库中的具体图标）
- 表格具体样式实现（原生 table / div grid / 第三方库）
- 卡片趋势线的具体实现方式（sparkline 库或简单 SVG）
- 统计卡片滚动 24 小时的技术实现（前端计算 vs 后端查询）
- 数值格式化规则（k/M 缩写阈值）
- 详情展开区域的具体布局
- 筛选下拉框是否也筛选统计卡片（推荐联动）

</decisions>

<specifics>
## Specific Ideas

- 多行单元格设计的核心理念：在不增加列数的情况下通过堆叠展示更多信息，保持表格宽度可控
- 统计卡片风格参考：带图标 + 微小趋势线的宽松设计，不是紧凑的纯数字卡片
- 时间列的相对时间显示（1 小时内）让实时日志更有"实时感"
- Phase 26 已预留 cache_creation_tokens 和 cache_read_tokens 列，Phase 29 统计卡片直接利用

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `AppShell.tsx`: 现有视图切换框架（opacity 过渡 + always-render），直接扩展 AppView 联合类型即可
- `Header.tsx`: 现有图标按钮模式（Settings 齿轮），Traffic 按钮可完全复用样式
- shadcn/ui `card.tsx`: 统计摘要卡片基础组件
- shadcn/ui `select.tsx`: Provider 筛选下拉框
- shadcn/ui `scroll-area.tsx`: 表格区域滚动容器
- `useProxyStatus.ts`: 代理状态 hook，可用于检测代理是否开启
- `lib/tauri.ts`: Tauri command 封装，需新增 traffic 相关查询函数

### Established Patterns
- Tauri event 监听：`useSyncListener.ts` 中 `listen("event-name", callback)` 模式，traffic-log 监听可参考
- 数据刷新：`syncKey` state 触发子组件 refetch 模式
- 暗色主题 + oklch CSS 变量体系（brand-accent, status-success 等）
- i18n：所有 UI 文字通过 `useTranslation()` 国际化

### Integration Points
- `AppShell.tsx`: 新增 "traffic" 视图分支和 TrafficPage 组件渲染
- `Header.tsx`: 新增 Traffic 图标按钮，onNavigate 类型扩展
- `lib/tauri.ts`: 新增 `getRecentLogs(limit)` 等 Tauri command 封装
- `types/`: 新增 TrafficLog 类型定义（对应后端 LogEntry 19 列 + type 字段）
- `i18n/`: 新增流量监控页面的中英文翻译 key

</code_context>

<deferred>
## Deferred Ideas

- 按 Provider 聚合表格（各 Provider 请求数、token、平均耗时）-- Phase 30
- 按时间聚合表格（每小时/每天）-- Phase 30
- 趋势图表（recharts 折线图/柱状图）-- Phase 30
- rollup_and_prune 定时清理 -- Phase 30
- 费用估算 (cost_usd) -- v2.7+ (ADV-01)
- 实时告警 -- v2.7+ (ADV-02)
- 导出报表 -- v2.7+ (ADV-03)

</deferred>

---

*Phase: 29-traffic-page*
*Context gathered: 2026-03-18*
