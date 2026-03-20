import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  Activity,
  ArrowDownRight,
  ArrowUpRight,
  Shield,
  Database,
} from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import type { TrafficLog } from "@/types/traffic";
import { formatTokenCount } from "./formatters";

interface TrafficStatsBarProps {
  /** 已经过筛选的日志列表 */
  logs: TrafficLog[];
  /** 从 DB 查询的 24h 精确请求总数（不受内存上限影响），仅在无 Provider 筛选时使用 */
  totalCount: number;
  /** 是否处于 Provider 筛选状态 */
  isFiltered?: boolean;
}

/** 将 24h 窗口内日志按时间分为 12 个桶（每桶 2h），统计每桶请求数 */
function buildSparklineData(logs: TrafficLog[]): number[] {
  const now = Date.now();
  const windowMs = 24 * 60 * 60 * 1000;
  const bucketMs = windowMs / 12;
  const windowStart = now - windowMs;

  const buckets = new Array<number>(12).fill(0);
  for (const log of logs) {
    if (log.created_at < windowStart) continue;
    const idx = Math.floor((log.created_at - windowStart) / bucketMs);
    const safeIdx = Math.min(Math.max(idx, 0), 11);
    buckets[safeIdx]++;
  }
  return buckets;
}

/** 轻量 inline SVG sparkline 趋势线 */
function Sparkline({ data }: { data: number[] }) {
  const nonZeroCount = data.filter((v) => v > 0).length;
  if (nonZeroCount < 2) return null;

  const width = 60;
  const height = 20;
  const max = Math.max(...data, 1);

  const points = data
    .map((v, i) => {
      const x = (i / (data.length - 1)) * width;
      const y = height - (v / max) * (height - 2) - 1;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");

  return (
    <svg
      width={width}
      height={height}
      className="text-muted-foreground/30 overflow-visible"
      aria-hidden="true"
    >
      <polyline
        points={points}
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function TrafficStatsBar({ logs, totalCount, isFiltered }: TrafficStatsBarProps) {
  const { t } = useTranslation();

  const stats = useMemo(() => {
    const now = Date.now();
    const windowMs = 24 * 60 * 60 * 1000;
    const windowLogs = logs.filter((l) => l.created_at >= now - windowMs);

    const total = windowLogs.length;
    const successCount = windowLogs.filter(
      (l) => l.status_code !== null && l.status_code >= 200 && l.status_code < 300
    ).length;
    const successRate = total > 0 ? (successCount / total) * 100 : 0;

    const inputTokens = windowLogs.reduce(
      (sum, l) => sum + (l.input_tokens ?? 0),
      0
    );
    const outputTokens = windowLogs.reduce(
      (sum, l) => sum + (l.output_tokens ?? 0),
      0
    );

    const cacheReadTokens = windowLogs.reduce(
      (sum, l) => sum + (l.cache_read_tokens ?? 0),
      0
    );
    const cacheCreationTokens = windowLogs.reduce(
      (sum, l) => sum + (l.cache_creation_tokens ?? 0),
      0
    );
    const inputSideTokens = inputTokens + cacheCreationTokens + cacheReadTokens;
    const cacheHitRate = inputSideTokens > 0 ? (cacheReadTokens / inputSideTokens) * 100 : 0;

    return { total, successRate, inputTokens, outputTokens, cacheHitRate, windowLogs };
  }, [logs]);

  const sparklineData = useMemo(
    () => buildSparklineData(stats.windowLogs),
    [stats.windowLogs]
  );

  const cards = [
    {
      icon: Activity,
      label: t("traffic.stats.totalRequests"),
      value: String(isFiltered ? stats.total : totalCount),
    },
    {
      icon: ArrowDownRight,
      label: t("traffic.stats.inputTokens"),
      value: formatTokenCount(stats.inputTokens),
    },
    {
      icon: ArrowUpRight,
      label: t("traffic.stats.outputTokens"),
      value: formatTokenCount(stats.outputTokens),
    },
    {
      icon: Shield,
      label: t("traffic.stats.successRate"),
      value: stats.total > 0 ? stats.successRate.toFixed(1) + "%" : "--",
    },
    {
      icon: Database,
      label: t("traffic.stats.cacheHitRate"),
      value: stats.total > 0 ? stats.cacheHitRate.toFixed(1) + "%" : "--",
    },
  ];

  return (
    <div className="grid grid-cols-5 gap-3 px-6 py-3">
      {cards.map((card, idx) => {
        const Icon = card.icon;
        return (
          <Card key={idx} className="gap-2 py-3">
            <CardContent className="px-4">
              <div className="flex items-center gap-1.5 mb-1">
                <Icon className="size-4 text-muted-foreground" />
                <span className="text-xs text-muted-foreground">
                  {card.label}
                </span>
              </div>
              <div className="text-2xl font-bold leading-none mb-1">
                {card.value}
              </div>
              <div className="flex items-center justify-between">
                <Sparkline data={sparklineData} />
                <span className="text-[10px] text-muted-foreground/60">
                  {t("traffic.statsBasis")}
                </span>
              </div>
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}
