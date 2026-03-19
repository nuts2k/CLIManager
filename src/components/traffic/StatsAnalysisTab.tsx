import { useTranslation } from "react-i18next";
import { BarChart2 } from "lucide-react";
import { useTrafficStats } from "@/hooks/useTrafficStats";
import { ProviderLeaderboard } from "./ProviderLeaderboard";
import { CacheLeaderboard } from "./CacheLeaderboard";
import { TrafficTrendChart } from "./TrafficTrendChart";
import type { TimeRange } from "@/types/traffic";

/**
 * 统计分析 Tab 主面板。
 * - 顶部：24h / 7d Segment 按钮组（联动更新下方所有数据）
 * - 中区：供应商排行榜、缓存命中率排行榜（各占一行）
 * - 下区：趋势图
 */
export function StatsAnalysisTab() {
  const { t } = useTranslation();
  const {
    timeRange,
    setTimeRange,
    providerStats,
    timeTrend,
    loading,
    dbError,
    lastSuccessfulRange,
  } = useTrafficStats();

  const ranges: TimeRange[] = ["24h", "7d"];
  const hasData = providerStats.length > 0 || timeTrend.length > 0;
  const showErrorBanner = !!dbError && hasData;
  const isStale = showErrorBanner && lastSuccessfulRange !== null && lastSuccessfulRange !== timeRange;
  const displayedRange = lastSuccessfulRange ?? timeRange;

  return (
    <div className="flex flex-col gap-5 pt-2">
      {/* 时间范围 Segment 按钮组 */}
      <div className="flex items-center">
        <div className="inline-flex rounded-lg bg-muted/50 p-0.5">
          {ranges.map((r) => (
            <button
              key={r}
              onClick={() => setTimeRange(r)}
              className={[
                "px-4 py-1.5 text-sm rounded-md transition-all duration-200",
                timeRange === r
                  ? "bg-foreground text-background font-medium shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              ].join(" ")}
            >
              {t(`traffic.analysis.range${r === "24h" ? "24h" : "7d"}`)}
            </button>
          ))}
        </div>
      </div>

      {showErrorBanner && (
        <div className="px-4 py-3 rounded-md bg-destructive/10 border border-destructive/20 text-sm text-destructive">
          <span className="font-medium">{t("traffic.dbErrorTitle")}</span>
          <span className="ml-1 text-destructive/80">
            {t(isStale ? "traffic.analysis.dbErrorShowingPreviousData" : "traffic.dbErrorDesc")}
          </span>
        </div>
      )}

      {/* 加载中骨架 */}
      {loading ? (
        <div className="flex items-center justify-center py-20">
          <span className="text-sm text-muted-foreground animate-pulse">
            {t("traffic.analysis.loading")}
          </span>
        </div>
      ) : dbError && !hasData ? (
        <div className="flex flex-col items-center justify-center gap-3 py-20 text-center">
          <BarChart2 className="size-10 text-destructive/60" />
          <div>
            <h3 className="text-base font-medium text-destructive">
              {t("traffic.dbErrorTitle")}
            </h3>
            <p className="mt-1 text-sm text-muted-foreground">
              {t("traffic.dbErrorDesc")}
            </p>
          </div>
        </div>
      ) : !hasData ? (
        /* 空状态 */
        <div className="flex flex-col items-center justify-center gap-3 py-20 text-center">
          <BarChart2 className="size-10 text-muted-foreground/50" />
          <div>
            <h3 className="text-base font-medium">
              {t("traffic.analysis.noData")}
            </h3>
            <p className="mt-1 text-sm text-muted-foreground">
              {t("traffic.analysis.noDataDesc")}
            </p>
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-5">
          {/* 排行榜区域：各占一行，充分展示数据 */}
          <ProviderLeaderboard data={providerStats} />
          <CacheLeaderboard data={providerStats} />

          {/* 趋势图：双轴（请求数柱状 + Token 折线） */}
          <TrafficTrendChart data={timeTrend} timeRange={displayedRange} />
        </div>
      )}
    </div>
  );
}
