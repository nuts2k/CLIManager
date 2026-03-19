# Phase 29: 前端流量监控页面 - Research

**Researched:** 2026-03-18
**Domain:** React / Tauri 前端 — 实时日志表格、统计摘要卡片、视图导航扩展
**Confidence:** HIGH（基于代码直读，无需第三方文档猜测）

---

<user_constraints>
## User Constraints（来自 CONTEXT.md）

### 锁定决策（Locked Decisions）

**导航入口**
- Header 右侧添加 Traffic 图标按钮，与 Settings 齿轮按钮并排
- 无状态指示（无红点、无计数徽章），纯图标按钮
- AppView 扩展为 `"main" | "traffic" | "settings"` 三视图互斥切换
- 点击图标进入对应视图，再点一次回到 main（toggle 行为，与 Settings 按钮一致）
- 复用现有 AppShell 的 opacity + pointer-events 过渡模式（150ms），Traffic 视图始终渲染不卸载，切走后保持状态

**日志表格列设计**
- 精简 6 列：时间、Provider、模型、状态码、Token、耗时
- 单元格内多行堆叠展示：
  - Token 列：第一行 in/out（如 1.2k / 3.4k），第二行缓存信息（如 cache read 128）
  - 耗时列：第一行总耗时（如 8.2s），第二行 TTFB（如 TTFB 1.2s），第三行 tps（如 42 t/s）
- 单元格内容垂直居中对齐
- 点击行展开详情区域，显示完整信息：协议类型、upstream_model、CLI、is_streaming、stop_reason、error_message

**日志表格时间显示**
- 1 小时内显示相对时间（xx 秒前 / xx 分前）
- 超过 1 小时显示具体时间（如 14:32:01）

**日志表格排序与插入**
- 固定按时间降序，不支持按列排序
- 新条目置顶插入
- 用户已滚动到下方时，不自动跳回顶部

**流式请求状态表现**
- token=null / duration=null 时，Token 和耗时列显示 `"--"` 占位符
- 收到 `traffic-log type="update"` 后替换为实际数值

**统计摘要卡片**
- 位于表格上方横向排列
- 带图标的宽松卡片风格（图标 + 标签 + 大号数值 + 微小趋势线）
- 5 张卡片：请求数、Input Token、Output Token、成功率、缓存命中率
- 数据范围：滚动 24 小时
- 随新日志实时更新

**Provider 筛选**
- 筛选下拉框位于统计卡片和表格之间
- 选项来源：从前端内存中的日志条目 distinct provider_name 提取
- 默认"全部"，选择后表格即时过滤
- 筛选同时影响统计卡片数值

**空状态与边界**
- 代理未开启时正常显示历史日志，不阻断页面
- 无任何日志时显示简洁文字提示（与 Provider 空状态风格一致）
- 不使用虚拟滚动；初始加载 100 条 + 实时追加，前端内存保持最多 500 条，超出时丢弃最旧

### Claude's Discretion（Claude 自行决定）
- Traffic 图标选择（lucide 图标库中的具体图标）
- 表格具体样式实现（原生 table / div grid / 第三方库）
- 卡片趋势线的具体实现方式（sparkline 库或简单 SVG）
- 统计卡片滚动 24 小时的技术实现（前端计算 vs 后端查询）
- 数值格式化规则（k/M 缩写阈值）
- 详情展开区域的具体布局
- 筛选下拉框是否也筛选统计卡片（推荐联动，已在锁定决策中确认联动）

### Deferred Ideas（超出本阶段范围）
- 按 Provider 聚合表格（各 Provider 请求数、token、平均耗时）— Phase 30
- 按时间聚合表格（每小时/每天）— Phase 30
- 趋势图表（recharts 折线图/柱状图）— Phase 30
- rollup_and_prune 定时清理 — Phase 30
- 费用估算 (cost_usd) — v2.7+ (ADV-01)
- 实时告警 — v2.7+ (ADV-02)
- 导出报表 — v2.7+ (ADV-03)

</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LOG-02 | 独立流量监控页面展示实时日志表格（时间、Provider、模型、状态码、token、耗时等列） | TrafficLogPayload 已定义所有需要的字段；`get_recent_logs` Tauri command 已就绪；`traffic-log` 事件已 emit |
| LOG-03 | 日志表格支持按 Provider 筛选，缺省显示全部 | 纯前端过滤：从内存日志 distinct provider_name；shadcn/ui `select.tsx` 已可用 |
| STAT-01 | 统计摘要卡片展示总请求数、总 input/output token、成功率 | 所有字段在 TrafficLogPayload 中均已存在；纯前端内存计算 |

</phase_requirements>

---

## Summary

Phase 29 是纯前端实现阶段。后端数据管道（Phase 27/28）已完全就绪：`get_recent_logs` Tauri command 可拉取历史日志，`traffic-log` Tauri 事件（type="new"/"update"）提供实时推送。本阶段工作是在前端消费这套数据，构建 TrafficPage 视图。

前端改动集中在四个维度：(1) 导航层扩展——AppShell 的 AppView 联合类型和 Header 的按钮；(2) 数据层——新 `useTrafficLogs` hook 封装历史拉取 + 实时监听；(3) UI 层——TrafficPage 组件树（统计卡片、筛选框、日志表格）；(4) 国际化——新增 traffic 相关 i18n key。

所有依赖库均已安装（lucide-react、radix-ui/select、shadcn/ui card/scroll-area）。本阶段**无需**安装新 npm 包，也**无需**任何 Rust 后端变更。

**Primary recommendation:** 使用原生 `<table>` + Tailwind 实现日志表格，行展开通过 React state 控制，统计计算在 useMemo 中对过滤后的日志执行，趋势线用轻量 inline SVG sparkline 实现。

---

## Standard Stack

### Core（已在项目中，无需安装）
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| React | 19.1.0 | UI 框架 | 项目基础 |
| lucide-react | 0.577.0 | 图标库 | 项目已用（Settings 齿轮等） |
| radix-ui / select | 1.4.3 | Provider 筛选下拉框 | 项目已有 shadcn/ui select.tsx |
| shadcn/ui card.tsx | — | 统计摘要卡片基础 | 项目已有 |
| shadcn/ui scroll-area.tsx | — | 表格滚动区域 | 项目已有 |
| shadcn/ui collapsible.tsx | — | 行展开详情 | 项目已有，基于 Radix Collapsible |
| @tauri-apps/api/event | ^2 | listen() 订阅 traffic-log 事件 | 项目已用（同 useSyncListener 模式） |
| @tauri-apps/api/core | ^2 | invoke() 调用 get_recent_logs | 项目已用（lib/tauri.ts 模式） |
| react-i18next | 16.5.6 | 国际化 | 项目已有，所有 UI 文字须经 useTranslation |

### 安装命令
```bash
# 无需安装新依赖，所有依赖均已在 package.json 中
```

---

## Architecture Patterns

### 推荐项目结构扩展
```
src/
├── components/
│   └── traffic/            # 新建目录
│       ├── TrafficPage.tsx      # 页面根组件（统筹布局）
│       ├── TrafficStatsBar.tsx  # 统计摘要卡片横排区域
│       ├── TrafficFilter.tsx    # Provider 筛选下拉框
│       ├── TrafficTable.tsx     # 日志表格（含行展开）
│       └── TrafficEmptyState.tsx # 无日志时的空状态
├── hooks/
│   └── useTrafficLogs.ts   # 新建：历史拉取 + 事件监听 + 内存管理
├── types/
│   └── traffic.ts          # 新建：TrafficLog TypeScript 类型
├── lib/
│   └── tauri.ts            # 扩展：新增 getRecentLogs() 函数
└── i18n/locales/
    ├── zh.json             # 扩展：新增 traffic.* 翻译 key
    └── en.json             # 扩展：新增 traffic.* 翻译 key
```

### Pattern 1: AppView 三视图扩展（AppShell.tsx）

**What:** 将 `AppView = "main" | "settings"` 扩展为 `"main" | "traffic" | "settings"`，并为 traffic 视图添加 always-render + opacity 过渡分支

**When to use:** 需要新增顶级视图，与现有 settings 视图完全对称

**Example:**
```typescript
// 修改前
type AppView = "main" | "settings";

// 修改后
type AppView = "main" | "traffic" | "settings";

// AppShell 渲染分支补充（完全复制 settings 模式）
const showTrafficView = view === "traffic" || exitingView === "traffic";

{showTrafficView ? (
  <div
    inert={view !== "traffic"}
    aria-hidden={view !== "traffic"}
    className={`absolute inset-0 transition-opacity duration-150 ease-out ${
      view === "traffic" ? "opacity-100" : "opacity-0 pointer-events-none"
    }`}
  >
    <TrafficPage />
  </div>
) : null}
```

### Pattern 2: Header Toggle 导航模式

**What:** Settings 按钮已有"再点一次回到 main"的 toggle 行为，Traffic 按钮需相同逻辑。

**关键点：** Header 收到 `onNavigate` 回调，AppShell 中 `handleNavigate` 已有 `if (nextView === view) return;` 防重复切换。Toggle 需在 Header 内判断：当前是 traffic 视图时点击 Traffic 按钮 → 调用 `onNavigate("main")`，否则调用 `onNavigate("traffic")`。

**Example:**
```typescript
// Header.tsx 扩展
interface HeaderProps {
  currentView: "main" | "traffic" | "settings";
  onNavigate: (view: "main" | "traffic" | "settings") => void;
}

// Traffic 按钮 onClick
onClick={() => onNavigate(currentView === "traffic" ? "main" : "traffic")}
```

> **注意：** Header 目前不接收 `currentView` prop，需新增此 prop 才能实现 toggle。Settings 按钮也需同样更新。

### Pattern 3: useTrafficLogs Hook（历史拉取 + 增量追加）

**What:** 封装"启动时拉历史 + 监听 traffic-log 事件增量追加"的双轨模式，内存最多 500 条

**When to use:** 任何消费 traffic 日志数据的组件

**Example:**
```typescript
// src/hooks/useTrafficLogs.ts
import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getRecentLogs } from "@/lib/tauri";
import type { TrafficLog } from "@/types/traffic";

const MAX_LOGS = 500;

export function useTrafficLogs() {
  const [logs, setLogs] = useState<TrafficLog[]>([]);
  const [loading, setLoading] = useState(true);

  // 初始拉取
  useEffect(() => {
    getRecentLogs(100)
      .then((items) => setLogs(items as TrafficLog[]))
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  // 实时监听
  useEffect(() => {
    const unlisten = listen<TrafficLog>("traffic-log", (event) => {
      const payload = event.payload;
      if (payload.type === "new") {
        setLogs((prev) => {
          const next = [payload, ...prev];
          return next.length > MAX_LOGS ? next.slice(0, MAX_LOGS) : next;
        });
      } else if (payload.type === "update") {
        setLogs((prev) =>
          prev.map((log) => (log.id === payload.id ? payload : log))
        );
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  return { logs, loading };
}
```

### Pattern 4: 统计计算（useMemo + 纯前端）

**What:** 对过滤后的日志列表，在 useMemo 中计算 5 个统计指标

**When to use:** Provider 筛选变更或 logs 变更时自动重算

**Example:**
```typescript
// 在 TrafficPage.tsx 或 TrafficStatsBar.tsx 中
const stats = useMemo(() => {
  const now = Date.now();
  const cutoff = now - 24 * 60 * 60 * 1000; // 24小时前
  const window = filteredLogs.filter((log) => log.created_at >= cutoff);

  const total = window.length;
  const successCount = window.filter(
    (log) => log.status_code !== null && log.status_code >= 200 && log.status_code < 300
  ).length;
  const inputTokens = window.reduce((sum, log) => sum + (log.input_tokens ?? 0), 0);
  const outputTokens = window.reduce((sum, log) => sum + (log.output_tokens ?? 0), 0);
  const cacheHitCount = window.filter(
    (log) => (log.cache_read_tokens ?? 0) > 0
  ).length;

  return {
    total,
    successRate: total > 0 ? successCount / total : 0,
    inputTokens,
    outputTokens,
    cacheHitRate: total > 0 ? cacheHitCount / total : 0,
  };
}, [filteredLogs]);
```

### Pattern 5: 不自动滚回顶部（scroll 位置保护）

**What:** 新条目置顶插入时，若用户已滚到下方，不跳回顶部

**When to use:** 每次 logs 更新时检查 ScrollArea 的滚动位置

**Example:**
```typescript
const scrollRef = useRef<HTMLDivElement>(null);
const isAtTopRef = useRef(true);

// 监听滚动位置
const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
  isAtTopRef.current = (e.currentTarget.scrollTop < 50);
};

// 新 log 插入后仅在顶部时才滚回顶
useEffect(() => {
  if (isAtTopRef.current && scrollRef.current) {
    scrollRef.current.scrollTo({ top: 0, behavior: "smooth" });
  }
}, [logs.length]);
```

> **注意：** Radix ScrollArea 的 viewport 元素需通过 `ref` 访问，可用 `data-slot="scroll-area-viewport"` 选择器或将 ref 传给 ScrollArea 内部的 div。

### Pattern 6: 行展开详情（React state）

**What:** 点击行 toggle expandedId state，不使用 Radix Collapsible（避免在 table row 中嵌套结构问题）

**When to use:** div-based 表格布局，点击行显示/隐藏详情区

**Example:**
```typescript
const [expandedId, setExpandedId] = useState<number | null>(null);

// 行点击
onClick={() => setExpandedId(expandedId === log.id ? null : log.id)}

// 详情区
{expandedId === log.id && (
  <div className="col-span-6 px-4 py-3 bg-muted/50 rounded-md text-xs">
    <dl className="grid grid-cols-2 gap-x-4 gap-y-1">
      <dt className="text-muted-foreground">协议</dt>
      <dd>{log.protocol_type}</dd>
      {/* ... */}
    </dl>
  </div>
)}
```

### Pattern 7: 数值格式化工具

**What:** k/M 缩写，时间相对/绝对，tps 计算

**Example:**
```typescript
// 数值格式化（阈值：>= 1000 使用 k，>= 1000000 使用 M）
function formatTokenCount(n: number | null): string {
  if (n === null) return "--";
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

// 相对时间（1h 内）
function formatTime(epochMs: number): string {
  const diff = Date.now() - epochMs;
  if (diff < 60_000) return `${Math.floor(diff / 1000)}秒前`;
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}分前`;
  const d = new Date(epochMs);
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`;
}

// tps = output_tokens / (duration_ms / 1000)
function calcTps(outputTokens: number | null, durationMs: number | null): string {
  if (!outputTokens || !durationMs || durationMs === 0) return "--";
  return `${(outputTokens / (durationMs / 1000)).toFixed(1)} t/s`;
}
```

### Pattern 8: TrafficLog TypeScript 类型定义

**What:** 与后端 `TrafficLogPayload` 字段一一对应的 TypeScript 接口

**Example:**
```typescript
// src/types/traffic.ts
export interface TrafficLog {
  type: "new" | "update" | "history";
  id: number;
  created_at: number;          // epoch ms
  provider_name: string;
  cli_id: string;
  method: string;
  path: string;
  status_code: number | null;
  is_streaming: number;        // 0 或 1
  request_model: string | null;
  upstream_model: string | null;
  protocol_type: string;       // "anthropic" | "open_ai_chat_completions" | "open_ai_responses"
  input_tokens: number | null;
  output_tokens: number | null;
  cache_creation_tokens: number | null;
  cache_read_tokens: number | null;
  ttfb_ms: number | null;
  duration_ms: number | null;
  stop_reason: string | null;
  error_message: string | null;
}
```

### Anti-Patterns to Avoid

- **将 TrafficPage 直接挂载为独立路由：** 本项目无路由，所有视图通过 AppView state 管理，使用 opacity 过渡。
- **在 useEffect 内部直接 mutate logs array：** 始终使用函数式 setState，避免 stale closure 问题（特别是 update 事件的 map 操作）。
- **每次 logs 变更都 scrollTo(0)：** 会打断用户阅读历史。需先检查 isAtTopRef。
- **用原生 HTML `<table>` 嵌套展开行：** `<tr>` 内不能直接放任意 div，展开行需额外 `<tr>` + `<td colSpan={6}>`，这在原生 table 中可以，但样式控制复杂。推荐用 div grid 布局替代。
- **后端筛选 Provider：** 不需要，也不应该新增后端 command 做筛选，前端内存过滤足够（最多 500 条）。
- **在 AppShell 中引入新的 state 管理库：** 保持与现有模式一致，用 React useState 即可。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Provider 筛选下拉框 | 自定义 dropdown | shadcn/ui `select.tsx`（已有） | Radix Select 处理了 a11y、键盘导航、portal 等 |
| 图标 | SVG 手写 | lucide-react（已安装） | 项目统一图标源，Traffic 可用 `Activity` 或 `BarChart2` |
| 卡片 UI | 纯 div | shadcn/ui `card.tsx`（已有） | 保持设计系统一致性 |
| 表格滚动区域 | overflow-y: auto + custom scrollbar | shadcn/ui `scroll-area.tsx`（已有） | Radix ScrollArea 跨平台滚动条样式一致 |
| 相对时间格式化 | 引入 date-fns/dayjs | 纯手写（逻辑简单，见上文） | 仅需 < 1h 相对 + 具体时间两种，无需引入库 |

**Key insight:** 所有 UI 组件在项目中均已存在。本阶段零新依赖。

---

## Common Pitfalls

### Pitfall 1: Header 不接收 currentView 导致 toggle 失效
**What goes wrong:** Settings 按钮现在只调用 `onNavigate("settings")`，没有 toggle 逻辑。Traffic 按钮要实现"再点一次回 main"必须知道当前视图。
**Why it happens:** Header 当前接口是 `onNavigate: (view: "main" | "settings") => void`，没有传入 currentView。
**How to avoid:** 同时更新 Header props 增加 `currentView`，或在 AppShell 中提供 toggle 包装函数。
**Warning signs:** 点击 Traffic 按钮无法从 traffic 视图切回 main。

### Pitfall 2: traffic-log "update" 事件对应的日志不在内存中
**What goes wrong:** 流式请求 type="new" 先插入，但 type="update" 来到时内存 logs 已满 500 条且最旧记录已被剔除，或者 update 先于 new 到达（竞态）。
**Why it happens:** 后端 insert + emit "new" 与 stream EOF 后 emit "update" 是异步的。
**How to avoid:** update handler 中用 `prev.map((log) => log.id === payload.id ? payload : log)`；若 find 不到，静默忽略（不新增条目）。
**Warning signs:** 流式请求完成后 token 列仍显示 "--"。

### Pitfall 3: 滚动 24h 窗口统计卡片数值与表格不同步
**What goes wrong:** 统计卡片计算 24h 内数据，但表格只显示最新 500 条。若历史请求量大，表格 500 条可能全在 24h 内，但统计应包含已被 500 条上限剔除的旧数据。
**Why it happens:** 内存只保 500 条，但统计窗口是时间维度（24h）。
**How to avoid:** 统计卡片明确标注"近 24 小时（最多 500 条）"，或在初始加载时拉取更多历史（如 500 条）用于统计，但表格仍只展示 100 条。分开两个数字：`getRecentLogs(500)` 用于内存上限（统计用），表格显示时不做额外截断（500 条全展示）。本阶段推荐：初始拉 100 条，内存上限 500 条，统计在当前内存上计算并注明"（基于当前缓存）"，与 CONTEXT.md 决策一致。

### Pitfall 4: Radix ScrollArea ref 访问 viewport
**What goes wrong:** 需要监听滚动事件或 scrollTo，但 ScrollArea 包装了内部 viewport，外部 ref 拿到的是外层容器而非滚动容器。
**Why it happens:** Radix ScrollAreaPrimitive.Viewport 是实际滚动的元素。
**How to avoid:** 给 ScrollArea 内部加 `onScroll` handler 或使用 `data-slot="scroll-area-viewport"` 做 querySelector（不推荐），最简洁的方式是直接用原生 `div` + `overflow-y-auto` 替代 ScrollArea，完全掌控 ref 和事件。

### Pitfall 5: 相对时间不自动刷新
**What goes wrong:** "xx 秒前"显示后不更新，60 秒过去后不变成"1分前"。
**Why it happens:** React 状态没有变化，不触发重渲染，时间字符串是在 render 期间计算的。
**How to avoid:** 在 TrafficPage 或 TrafficTable 中设置一个 `setInterval(1分钟)` 触发 forceUpdate（或用 `useState` 的计时器刷新），确保时间列按时重算。

### Pitfall 6: div-grid 表格中展开行宽度计算
**What goes wrong:** 使用 CSS Grid 实现表格时，展开的详情区域需要跨越所有 6 列，但 `col-span-6` 在不同 grid 配置下可能失效。
**Why it happens:** Grid 列定义需与 colSpan 数字匹配。
**How to avoid:** 明确定义 `grid-cols-[col1-width_col2-width_col3-width_col4-width_col5-width_col6-width]`，并在展开行上使用 `col-span-6`。

---

## Code Examples

### Tauri Command 封装（lib/tauri.ts 扩展）
```typescript
// 新增到 src/lib/tauri.ts
import type { TrafficLog } from "@/types/traffic";

export async function getRecentLogs(limit?: number): Promise<TrafficLog[]> {
  return invoke("get_recent_logs", { limit });
}
```

### traffic-log 事件监听模式（参考 useSyncListener.ts）
```typescript
// 参考现有模式：listen() 返回 Promise<UnlistenFn>
const unlisten = listen<TrafficLog>("traffic-log", (event) => {
  const payload = event.payload;
  // payload.type === "new" | "update" | "history"
  // payload.id: number（SQLite rowid）
  // payload.created_at: number（epoch ms）
});

// 清理
return () => { unlisten.then((fn) => fn()); };
```

### Provider 筛选 Select 组件模式（参考 settings/SettingsPage.tsx）
```typescript
import {
  Select, SelectContent, SelectItem,
  SelectTrigger, SelectValue
} from "@/components/ui/select";

// 从 logs 提取 distinct providers
const providers = useMemo(() =>
  [...new Set(logs.map((log) => log.provider_name))].sort(),
  [logs]
);

<Select value={selectedProvider} onValueChange={setSelectedProvider}>
  <SelectTrigger className="w-48">
    <SelectValue placeholder={t("traffic.allProviders")} />
  </SelectTrigger>
  <SelectContent>
    <SelectItem value="__all__">{t("traffic.allProviders")}</SelectItem>
    {providers.map((p) => (
      <SelectItem key={p} value={p}>{p}</SelectItem>
    ))}
  </SelectContent>
</Select>
```

### 状态码颜色语义（基于现有 CSS 变量）
```typescript
function statusCodeClass(code: number | null): string {
  if (code === null) return "text-muted-foreground";
  if (code >= 200 && code < 300) return "text-status-success";
  if (code >= 400 && code < 500) return "text-status-warning";
  if (code >= 500) return "text-destructive";
  return "text-foreground";
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Provider 筛选通过后端 SQL WHERE 实现 | 纯前端内存过滤（500 条上限内） | Phase 29 设计决策 | 无需新 Rust command，响应更快 |
| 没有 traffic 页面 | TrafficPage 作为第三个顶级视图 | Phase 29 | AppView 联合类型扩展 |

**Deprecated/outdated:**
- 无（本阶段为新增功能）

---

## Open Questions

1. **趋势线实现方式**
   - What we know: CONTEXT.md 将趋势线列为 Claude's Discretion；项目无 recharts（recharts 被 defer 到 Phase 30）；有 tailwindcss 且 React 19 支持 SVG
   - What's unclear: 简单 inline SVG sparkline 所需数据粒度（每条记录？每分钟聚合？）
   - Recommendation: 使用轻量 inline SVG polyline，基于过去 30 个时间桶（每桶 48 分钟）的请求数，纯计算无需库。若觉得实现复杂，可先用纯数字卡片（无趋势线），趋势线设为 "nice to have"。

2. **统计卡片 24h 窗口与 500 条内存上限的潜在偏差**
   - What we know: CONTEXT.md 明确"数据范围：滚动 24 小时（具体技术实现方式 Claude 设计，难以实现时再讨论）"
   - What's unclear: 高流量用户 24h 内可能有 >500 条请求，统计会漏算
   - Recommendation: 初始拉取时使用 `limit=500`（而非 100），用 500 条作为统计基准。表格展示同一批 500 条（不需要单独 100 条限制）。CONTEXT.md 说"初始加载 100 条 + 实时追加，前端内存保持最多 500 条"，所以初始拉 100 在界面上，但内存允许到 500。**合理方案：初始拉 100 展示，内存追加到 500，统计按内存内所有数据（<=500 条）+24h 筛选计算，不追求绝对精确。**

3. **时间列相对时间刷新频率**
   - What we know: "xx 秒前"需要定期刷新才能更新
   - What's unclear: 刷新频率 1s vs 30s？
   - Recommendation: 30s 刷新一次（`setInterval(30_000)`），精度足够，避免过多 rerender。刷新通过 `useReducer` 或 `useState` 的 dummy counter 触发。

---

## Validation Architecture

nyquist_validation 已启用（config.json `workflow.nyquist_validation: true`）。

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust 内置测试（`cargo test`）—— 已有 traffic/log.rs 中的 #[cfg(test)] 模块 |
| 前端测试框架 | 无（项目无 vitest/jest 配置，前端代码无测试文件） |
| Config file | `src-tauri/Cargo.toml`（Rust tests） |
| Quick run command | `cd src-tauri && cargo test --lib traffic` |
| Full suite command | `cd src-tauri && cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LOG-02 | TrafficPage 正确渲染日志表格（有数据、空状态） | manual-only | — | ❌ 无前端测试框架 |
| LOG-02 | get_recent_logs Tauri command 返回正确格式 | unit (Rust) | `cd src-tauri && cargo test --lib traffic::log` | ✅ traffic/log.rs tests 已覆盖 query_recent_logs |
| LOG-03 | Provider 筛选仅显示匹配项 | manual-only | — | ❌ 无前端测试框架 |
| STAT-01 | 统计卡片数值与内存数据一致 | manual-only | — | ❌ 无前端测试框架 |

### Sampling Rate
- **Per task commit:** `cd src-tauri && cargo test --lib traffic::log` （如有 Rust 改动）
- **Per wave merge:** `cd src-tauri && cargo test`
- **Phase gate:** Rust 测试全绿 + 手动验证 5 个 success criteria 后才执行 `/gsd:verify-work`

### Wave 0 Gaps
- 前端测试框架：项目无 vitest/jest，无前端测试。本阶段纯前端实现，验收依赖手动测试 success criteria。无需在 Wave 0 建立前端测试基础设施（超出本阶段范围）。

---

## Sources

### Primary（HIGH confidence）
- 直读项目源码：`src/components/layout/AppShell.tsx`、`Header.tsx`、`hooks/useSyncListener.ts`
- 直读后端：`src-tauri/src/traffic/log.rs`、`commands/traffic.rs`、`traffic/schema.rs`
- 直读现有类型：`src/types/provider.ts`、`src/types/settings.ts`
- 直读 UI 组件：`src/components/ui/card.tsx`、`select.tsx`、`scroll-area.tsx`、`collapsible.tsx`
- 直读 i18n：`src/i18n/locales/zh.json`（了解现有翻译 key 结构）
- 直读 CSS：`src/index.css`（了解 oklch CSS 变量体系）

### Secondary（MEDIUM confidence）
- CONTEXT.md 中的 Implementation Decisions（用户已确认的决策）
- REQUIREMENTS.md 中 LOG-02、LOG-03、STAT-01 描述

### Tertiary（LOW confidence）
- 无

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 全部通过直读代码确认，零第三方文档依赖
- Architecture patterns: HIGH — 基于现有 AppShell/Header/useSyncListener 代码推导，一致性强
- Pitfalls: HIGH — 基于代码现状分析的实际风险点，非猜测

**Research date:** 2026-03-18
**Valid until:** 2026-06-18（项目前端架构稳定，90 天内有效）
