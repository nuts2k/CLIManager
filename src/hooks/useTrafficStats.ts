import { useState, useEffect } from "react";
import type { ProviderStat, TimeStat, TimeRange } from "@/types/traffic";
import { getProviderStats, getTimeTrend } from "@/lib/tauri";

/**
 * 聚合数据 hook — 管理时间范围状态，自动拉取 ProviderStat 和 TimeStat 数据。
 * timeRange 变化时联动刷新，无需 SWR/React Query（项目无此依赖）。
 *
 * loading 仅在当前无旧数据可展示时为 true；切换时间范围时若已有旧数据则继续展示，避免闪烁。
 */
export function useTrafficStats(): {
  timeRange: TimeRange;
  setTimeRange: (range: TimeRange) => void;
  providerStats: ProviderStat[];
  timeTrend: TimeStat[];
  loading: boolean;
  dbError: string | null;
  lastSuccessfulRange: TimeRange | null;
} {
  const [timeRange, setTimeRange] = useState<TimeRange>("24h");
  const [providerStats, setProviderStats] = useState<ProviderStat[]>([]);
  const [timeTrend, setTimeTrend] = useState<TimeStat[]>([]);
  const [loading, setLoading] = useState(true);
  const [dbError, setDbError] = useState<string | null>(null);
  const [lastSuccessfulRange, setLastSuccessfulRange] = useState<TimeRange | null>(null);

  useEffect(() => {
    let cancelled = false;
    const hasCachedData = providerStats.length > 0 || timeTrend.length > 0;

    setDbError(null);
    setLoading(!hasCachedData);

    Promise.all([getProviderStats(timeRange), getTimeTrend(timeRange)])
      .then(([stats, trend]) => {
        if (!cancelled) {
          setProviderStats(stats);
          setTimeTrend(trend);
          setLastSuccessfulRange(timeRange);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("[useTrafficStats] 拉取聚合数据失败:", err);
          setDbError(String(err));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [timeRange]);

  return {
    timeRange,
    setTimeRange,
    providerStats,
    timeTrend,
    loading,
    dbError,
    lastSuccessfulRange,
  };
}
