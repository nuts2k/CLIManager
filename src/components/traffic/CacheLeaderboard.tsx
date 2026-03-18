import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronUp, ChevronDown, ChevronsUpDown } from "lucide-react";
import type { ProviderStat } from "@/types/traffic";
import { formatTokenCount } from "./formatters";

interface CacheLeaderboardProps {
  data: ProviderStat[];
}

/**
 * 缓存命中率排行榜表格。
 *
 * 列：Provider | 缓存触发数 | 命中率 | 缓存读取 Token | 总 Token
 * 默认排序：命中率降序。点击表头切换排序列和方向。
 */

type SortKey =
  | "provider_name"
  | "cache_triggered_count"
  | "cache_hit_rate"
  | "total_cache_read_tokens"
  | "total_tokens";

type SortOrder = "asc" | "desc";

/** 排序指示图标 */
function SortIcon({ active, order }: { active: boolean; order: SortOrder }) {
  if (!active) return <ChevronsUpDown className="inline ml-1 size-3 opacity-40" />;
  if (order === "desc") return <ChevronDown className="inline ml-1 size-3" />;
  return <ChevronUp className="inline ml-1 size-3" />;
}

export function CacheLeaderboard({ data }: CacheLeaderboardProps) {
  const { t } = useTranslation();
  const [sortKey, setSortKey] = useState<SortKey>("cache_hit_rate");
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
      case "cache_triggered_count":
        va = a.cache_triggered_count;
        vb = b.cache_triggered_count;
        break;
      case "cache_hit_rate":
        va =
          a.cache_triggered_count > 0
            ? a.cache_hit_count / a.cache_triggered_count
            : -1;
        vb =
          b.cache_triggered_count > 0
            ? b.cache_hit_count / b.cache_triggered_count
            : -1;
        break;
      case "total_cache_read_tokens":
        va = a.total_cache_read_tokens;
        vb = b.total_cache_read_tokens;
        break;
      case "total_tokens":
        va = a.total_input_tokens + a.total_output_tokens;
        vb = b.total_input_tokens + b.total_output_tokens;
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
    { key: "cache_triggered_count", label: t("traffic.analysis.colCacheTriggered"), align: "right" },
    { key: "cache_hit_rate", label: t("traffic.analysis.colCacheHitRate"), align: "right" },
    { key: "total_cache_read_tokens", label: t("traffic.analysis.colCacheReadTokens"), align: "right" },
    { key: "total_tokens", label: t("traffic.analysis.colTotalTokens"), align: "right" },
  ];

  return (
    <div className="rounded-md border border-border overflow-hidden">
      {/* 表格标题 */}
      <div className="px-3 py-2 border-b border-border bg-muted/30">
        <span className="text-sm font-medium">
          {t("traffic.analysis.cacheLeaderboard")}
        </span>
      </div>

      {/* div-based grid 布局（与 TrafficTable 保持一致） */}
      <div
        className="grid"
        style={{
          gridTemplateColumns:
            "minmax(80px,1.5fr) minmax(80px,1fr) minmax(60px,1fr) minmax(100px,1fr) minmax(80px,1fr)",
        }}
      >
        {/* 表头 */}
        {columns.map((col) => (
          <div
            key={col.key}
            className={[
              "px-2 py-2 text-xs text-muted-foreground font-medium border-b border-border",
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
        {sorted.map((stat) => {
          const hitRate =
            stat.cache_triggered_count > 0
              ? ((stat.cache_hit_count / stat.cache_triggered_count) * 100).toFixed(1) + "%"
              : "--";
          const totalTokens = stat.total_input_tokens + stat.total_output_tokens;

          return (
            <div key={stat.provider_name} className="contents group">
              <div className="px-2 py-2 text-sm border-b border-border/50 group-hover:bg-muted/40 transition-colors truncate">
                {stat.provider_name}
              </div>
              <div className="px-2 py-2 text-sm text-right border-b border-border/50 group-hover:bg-muted/40 transition-colors">
                {stat.cache_triggered_count}
              </div>
              <div className="px-2 py-2 text-sm text-right border-b border-border/50 group-hover:bg-muted/40 transition-colors">
                {hitRate}
              </div>
              <div className="px-2 py-2 text-sm text-right border-b border-border/50 group-hover:bg-muted/40 transition-colors">
                {formatTokenCount(stat.total_cache_read_tokens)}
              </div>
              <div className="px-2 py-2 text-sm text-right border-b border-border/50 group-hover:bg-muted/40 transition-colors">
                {formatTokenCount(totalTokens)}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
