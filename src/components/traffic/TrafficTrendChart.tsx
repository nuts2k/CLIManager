/**
 * 流量趋势图 — 同时实现 STAT-03（按时间聚合展示）和 STAT-04（趋势图表）。
 * 设计决策：以 recharts 图表形式替代独立的时间聚合表格，提供更直观的可视化。
 * 此方案在 Phase 30 Plan 03 用户验收中已 approved。
 */
import { useTranslation } from "react-i18next";
import {
  ResponsiveContainer,
  ComposedChart,
  CartesianGrid,
  XAxis,
  YAxis,
  Bar,
  Line,
  Tooltip,
  Legend,
} from "recharts";
import type { TimeStat, TimeRange } from "@/types/traffic";

// ──────────────────────────────────────────────
// 数据填充纯函数
// ──────────────────────────────────────────────

/**
 * 将后端返回的小时级 TimeStat 填充为完整滚动 24 小时点（缺失填 0）。
 * 后端 label 格式：如 "08:00"（本地时间）；本函数以当前本地整点为终点，
 * 向前生成 24 个标签，与后端按 `unixepoch, 'localtime'` 聚合得到的小时标签保持一致，缺失项补 0。
 */
export function buildHourlyData(raw: TimeStat[], now = new Date()): TimeStat[] {
  const map = new Map<string, TimeStat>();
  for (const item of raw) {
    map.set(item.label, item);
  }

  const currentHour = new Date(now);
  currentHour.setMinutes(0, 0, 0);

  return Array.from({ length: 24 }, (_, index) => {
    const point = new Date(currentHour);
    point.setHours(currentHour.getHours() - (23 - index));
    const label = `${String(point.getHours()).padStart(2, "0")}:00`;
    return map.get(label) ?? { label, request_count: 0, total_tokens: 0 };
  });
}

function formatLocalDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/**
 * 将后端返回的天级 TimeStat 填充为完整 7 天（缺失填 0）。
 * 后端 label 格式：如 "2024-03-18"（YYYY-MM-DD）；
 * 本函数按本地日期生成从今天往前 6 天的 7 个标签，X 轴显示 "MM-DD" 缩写。
 */
export function buildDailyData(raw: TimeStat[]): TimeStat[] {
  // 后端 label 是 "YYYY-MM-DD"；建立 full-date → item 的映射
  const map = new Map<string, TimeStat>();
  for (const item of raw) {
    map.set(item.label, item);
  }

  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date();
    d.setHours(0, 0, 0, 0);
    d.setDate(d.getDate() - (6 - i)); // 从 6 天前到今天
    const fullDate = formatLocalDate(d); // "YYYY-MM-DD"
    const displayLabel = fullDate.slice(5); // "MM-DD"（X 轴展示）
    const raw_item = map.get(fullDate);
    return {
      label: displayLabel,
      request_count: raw_item?.request_count ?? 0,
      total_tokens: raw_item?.total_tokens ?? 0,
    };
  });
}

// ──────────────────────────────────────────────
// 组件
// ──────────────────────────────────────────────

interface TrafficTrendChartProps {
  data: TimeStat[];
  timeRange: TimeRange;
}

/**
 * 双轴趋势图：
 * - 左 Y 轴（柱状图）：请求数
 * - 右 Y 轴（折线图）：Token 总量
 * 使用 recharts ComposedChart，颜色全部通过 CSS 变量适配暗色模式。
 */
export function TrafficTrendChart({ data, timeRange }: TrafficTrendChartProps) {
  const { t } = useTranslation();

  const chartData =
    timeRange === "24h" ? buildHourlyData(data) : buildDailyData(data);

  return (
    <div className="rounded-lg border border-border/60 overflow-hidden bg-card/30">
      <div className="px-4 py-2.5 border-b border-border/60 bg-muted/20">
        <span className="text-sm font-medium tracking-wide">
          {t("traffic.analysis.trendChart")}
        </span>
      </div>
      <div className="p-4">
      <ResponsiveContainer width="100%" height={240}>
        <ComposedChart
          data={chartData}
          margin={{ top: 4, right: 16, left: 0, bottom: 0 }}
        >
          <CartesianGrid
            strokeDasharray="3 3"
            stroke="var(--color-border)"
            vertical={false}
          />
          <XAxis
            dataKey="label"
            tick={{ fill: "var(--color-muted-foreground)", fontSize: 11 }}
            tickLine={false}
            axisLine={false}
            interval={timeRange === "24h" ? 3 : 0}
          />
          {/* 左 Y 轴：请求数 */}
          <YAxis
            yAxisId="requests"
            orientation="left"
            tick={{ fill: "var(--color-muted-foreground)", fontSize: 11 }}
            tickLine={false}
            axisLine={false}
            allowDecimals={false}
            width={36}
          />
          {/* 右 Y 轴：Token 总量 */}
          <YAxis
            yAxisId="tokens"
            orientation="right"
            tick={{ fill: "var(--color-muted-foreground)", fontSize: 11 }}
            tickLine={false}
            axisLine={false}
            allowDecimals={false}
            width={48}
          />
          <Tooltip
            contentStyle={{
              background: "var(--color-popover)",
              border: "1px solid var(--color-border)",
              borderRadius: "6px",
              color: "var(--color-foreground)",
              fontSize: 12,
            }}
            labelStyle={{ color: "var(--color-muted-foreground)" }}
            cursor={{ fill: "var(--color-muted)/0.1" }}
          />
          <Legend
            wrapperStyle={{
              fontSize: 12,
              color: "var(--color-muted-foreground)",
            }}
          />
          {/* 柱状图：请求数 */}
          <Bar
            yAxisId="requests"
            dataKey="request_count"
            name={t("traffic.analysis.chartRequests")}
            fill="var(--color-chart-1)"
            radius={[2, 2, 0, 0]}
            maxBarSize={32}
          />
          {/* 折线图：Token 总量 */}
          <Line
            yAxisId="tokens"
            dataKey="total_tokens"
            name={t("traffic.analysis.chartTokens")}
            stroke="var(--color-chart-2)"
            dot={false}
            strokeWidth={2}
            type="monotone"
          />
        </ComposedChart>
      </ResponsiveContainer>
      </div>
    </div>
  );
}
