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
 * 将后端返回的小时级 TimeStat 填充为完整 24 个小时点（缺失填 0）。
 * 后端 label 格式：如 "08:00"；本函数生成 "00:00" ~ "23:00" 共 24 个标签，
 * 与后端数据做精确匹配，缺失项补 0。
 */
export function buildHourlyData(raw: TimeStat[]): TimeStat[] {
  const map = new Map<string, TimeStat>();
  for (const item of raw) {
    map.set(item.label, item);
  }

  return Array.from({ length: 24 }, (_, h) => {
    const label = `${String(h).padStart(2, "0")}:00`;
    return map.get(label) ?? { label, request_count: 0, total_tokens: 0 };
  });
}

/**
 * 将后端返回的天级 TimeStat 填充为完整 7 天（缺失填 0）。
 * 后端 label 格式：如 "2024-03-18"（YYYY-MM-DD）；
 * 本函数生成从今天往前 6 天的 7 个标签，X 轴显示 "MM-DD" 缩写。
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
    const fullDate = d.toISOString().slice(0, 10); // "YYYY-MM-DD"
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
    <div className="rounded-md border border-border/50 p-4">
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
  );
}
