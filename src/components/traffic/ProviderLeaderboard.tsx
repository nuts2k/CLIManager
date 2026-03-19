import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronUp, ChevronDown, ChevronsUpDown } from "lucide-react";
import type { ProviderStat } from "@/types/traffic";
import { formatTokenCount, formatDuration } from "./formatters";

interface ProviderLeaderboardProps {
  data: ProviderStat[];
}

/**
 * 供应商排行榜表格。
 *
 * 列：Provider | 请求数 | Token | 成功率 | 平均 TTFB | 平均 TPS
 * 默认排序：请求数降序。点击表头切换排序列和方向。
 */

type SortKey =
  | "provider_name"
  | "request_count"
  | "total_tokens"
  | "success_rate"
  | "avg_ttfb"
  | "avg_tps";

type SortOrder = "asc" | "desc";

/** 计算供应商 TPS：SUM(output_tokens) / (SUM(duration_ms) - SUM(ttfb_ms)) * 1000 */
function calcAvgTps(stat: ProviderStat): number {
  const net = stat.sum_duration_ms - stat.sum_ttfb_ms;
  if (net <= 0 || stat.total_output_tokens === 0) return 0;
  return (stat.total_output_tokens / net) * 1000;
}

/** 排序指示图标 */
function SortIcon({ active, order }: { active: boolean; order: SortOrder }) {
  if (!active) return <ChevronsUpDown className="inline ml-1 size-3 opacity-40" />;
  if (order === "desc") return <ChevronDown className="inline ml-1 size-3" />;
  return <ChevronUp className="inline ml-1 size-3" />;
}

export function ProviderLeaderboard({ data }: ProviderLeaderboardProps) {
  const { t } = useTranslation();
  const [sortKey, setSortKey] = useState<SortKey>("request_count");
  const [sortOrder, setSortOrder] = useState<SortOrder>("desc");

  /** 点击表头：同列切换升降序，不同列默认降序 */
  function handleSort(key: SortKey) {
    if (key === sortKey) {
      setSortOrder((prev) => (prev === "desc" ? "asc" : "desc"));
    } else {
      setSortKey(key);
      setSortOrder("desc");
    }
  }

  /** 排序后的数据 */
  const sorted = [...data].sort((a, b) => {
    let va: number | string = 0;
    let vb: number | string = 0;

    switch (sortKey) {
      case "provider_name":
        va = a.provider_name;
        vb = b.provider_name;
        break;
      case "request_count":
        va = a.request_count;
        vb = b.request_count;
        break;
      case "total_tokens":
        va = a.total_input_tokens + a.total_output_tokens;
        vb = b.total_input_tokens + b.total_output_tokens;
        break;
      case "success_rate":
        va = a.request_count > 0 ? a.success_count / a.request_count : 0;
        vb = b.request_count > 0 ? b.success_count / b.request_count : 0;
        break;
      case "avg_ttfb":
        va = a.request_count > 0 ? a.sum_ttfb_ms / a.request_count : 0;
        vb = b.request_count > 0 ? b.sum_ttfb_ms / b.request_count : 0;
        break;
      case "avg_tps":
        va = calcAvgTps(a);
        vb = calcAvgTps(b);
        break;
    }

    if (typeof va === "string" && typeof vb === "string") {
      return sortOrder === "asc"
        ? va.localeCompare(vb)
        : vb.localeCompare(va);
    }
    return sortOrder === "asc"
      ? (va as number) - (vb as number)
      : (vb as number) - (va as number);
  });

  /** 表头列定义 */
  const columns: { key: SortKey; label: string; align?: "right" }[] = [
    { key: "provider_name", label: t("traffic.analysis.colProvider") },
    { key: "request_count", label: t("traffic.analysis.colRequests"), align: "right" },
    { key: "total_tokens", label: t("traffic.analysis.colTokens"), align: "right" },
    { key: "success_rate", label: t("traffic.analysis.colSuccessRate"), align: "right" },
    { key: "avg_ttfb", label: t("traffic.analysis.colAvgTtfb"), align: "right" },
    { key: "avg_tps", label: t("traffic.analysis.colAvgTps"), align: "right" },
  ];

  return (
    <div className="rounded-lg border border-border/60 overflow-hidden bg-card/30">
      {/* 表格标题 */}
      <div className="px-4 py-2.5 border-b border-border/60 bg-muted/20">
        <span className="text-sm font-medium tracking-wide">
          {t("traffic.analysis.providerLeaderboard")}
        </span>
      </div>

      {/* div-based grid 布局 */}
      <div
        className="grid"
        style={{
          gridTemplateColumns:
            "minmax(100px,1.8fr) minmax(70px,1fr) minmax(90px,1fr) minmax(70px,1fr) minmax(80px,1fr) minmax(80px,1fr)",
        }}
      >
        {/* 表头 */}
        {columns.map((col) => (
          <div
            key={col.key}
            className={[
              "px-3 py-2.5 text-xs text-muted-foreground/80 font-medium border-b border-border/40 bg-muted/10",
              "cursor-pointer select-none hover:text-foreground transition-colors",
              col.align === "right" ? "text-right" : "text-left",
            ].join(" ")}
            onClick={() => handleSort(col.key)}
          >
            {col.label}
            <SortIcon active={sortKey === col.key} order={sortOrder} />
          </div>
        ))}

        {/* 数据行 */}
        {sorted.map((stat, idx) => {
          const totalTokens = stat.total_input_tokens + stat.total_output_tokens;
          const successRate =
            stat.request_count > 0
              ? ((stat.success_count / stat.request_count) * 100).toFixed(1) + "%"
              : "--";
          const avgTtfb =
            stat.request_count > 0
              ? formatDuration(stat.sum_ttfb_ms / stat.request_count)
              : "--";
          const tpsVal = calcAvgTps(stat);
          const avgTps = tpsVal > 0 ? tpsVal.toFixed(1) + " t/s" : "--";
          const isLast = idx === sorted.length - 1;
          const borderClass = isLast ? "" : "border-b border-border/30";

          return (
            <div key={stat.provider_name} className="contents group">
              <div className={`px-3 py-2.5 text-sm ${borderClass} group-hover:bg-muted/30 transition-colors truncate font-medium`}>
                {stat.provider_name}
              </div>
              <div className={`px-3 py-2.5 text-sm text-right tabular-nums ${borderClass} group-hover:bg-muted/30 transition-colors`}>
                {stat.request_count}
              </div>
              <div className={`px-3 py-2.5 text-sm text-right tabular-nums ${borderClass} group-hover:bg-muted/30 transition-colors`}>
                {formatTokenCount(totalTokens)}
              </div>
              <div className={`px-3 py-2.5 text-sm text-right tabular-nums ${borderClass} group-hover:bg-muted/30 transition-colors`}>
                {successRate}
              </div>
              <div className={`px-3 py-2.5 text-sm text-right tabular-nums ${borderClass} group-hover:bg-muted/30 transition-colors`}>
                {avgTtfb}
              </div>
              <div className={`px-3 py-2.5 text-sm text-right tabular-nums ${borderClass} group-hover:bg-muted/30 transition-colors`}>
                {avgTps}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
