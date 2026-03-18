import { useState, useEffect } from "react";
import type { ProviderStat, TimeStat, TimeRange } from "@/types/traffic";
import { getProviderStats, getTimeTrend } from "@/lib/tauri";

/**
 * 聚合数据 hook — 管理时间范围状态，自动拉取 ProviderStat 和 TimeStat 数据。
 * timeRange 变化时联动刷新，无需 SWR/React Query（项目无此依赖）。
 */
export function useTrafficStats(): {
  timeRange: TimeRange;
  setTimeRange: (range: TimeRange) => void;
  providerStats: ProviderStat[];
  timeTrend: TimeStat[];
  loading: boolean;
} {
  const [timeRange, setTimeRange] = useState<TimeRange>("24h");
  const [providerStats, setProviderStats] = useState<ProviderStat[]>([]);
  const [timeTrend, setTimeTrend] = useState<TimeStat[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    Promise.all([getProviderStats(timeRange), getTimeTrend(timeRange)])
      .then(([stats, trend]) => {
        if (!cancelled) {
          setProviderStats(stats);
          setTimeTrend(trend);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("[useTrafficStats] 拉取聚合数据失败:", err);
          setProviderStats([]);
          setTimeTrend([]);
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

  return { timeRange, setTimeRange, providerStats, timeTrend, loading };
}
