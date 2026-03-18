import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { TrafficLog } from "@/types/traffic";
import {
  formatTime,
  formatDuration,
  calcTps,
  statusCodeClass,
  formatTokenCount,
} from "./formatters";

interface TrafficTableProps {
  /** 已筛选后的日志列表（最新在前） */
  logs: TrafficLog[];
}

/** 展开详情中的单个字段 */
function DetailField({
  label,
  value,
  className,
}: {
  label: string;
  value: string | null | undefined;
  className?: string;
}) {
  if (value === null || value === undefined) return null;
  return (
    <div>
      <dt className="text-muted-foreground mb-0.5">{label}</dt>
      <dd className={className}>{value}</dd>
    </div>
  );
}

export function TrafficTable({ logs }: TrafficTableProps) {
  const { t } = useTranslation();

  /** 当前展开行 id */
  const [expandedId, setExpandedId] = useState<number | null>(null);

  /** 滚动区域 ref */
  const scrollRef = useRef<HTMLDivElement>(null);
  /** 是否处于顶部（< 50px 时视为顶部） */
  const isAtTopRef = useRef(true);

  /** 每 30 秒刷新一次，使相对时间保持更新 */
  const [, setTick] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setTick((n) => n + 1), 30_000);
    return () => clearInterval(id);
  }, []);

  /** 监听滚动位置 */
  const handleScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    isAtTopRef.current = el.scrollTop < 50;
  };

  /** 新日志追加时，若用户在顶部则自动滚回顶部 */
  useEffect(() => {
    if (isAtTopRef.current && scrollRef.current) {
      scrollRef.current.scrollTo({ top: 0, behavior: "smooth" });
    }
  }, [logs.length]);

  /** 渲染时间列 */
  function renderTime(epochMs: number) {
    const ft = formatTime(epochMs);
    if (ft.type === "seconds")
      return t("traffic.table.secondsAgo", { count: ft.count });
    if (ft.type === "minutes")
      return t("traffic.table.minutesAgo", { count: ft.count });
    return ft.value;
  }

  return (
    <div
      ref={scrollRef}
      className="overflow-y-auto h-full"
      onScroll={handleScroll}
    >
      {/* 表格主体：div-based grid（避免 tr 内嵌套 div 的样式问题） */}
      <div
        className="grid min-w-[600px]"
        style={{
          gridTemplateColumns:
            "minmax(70px,auto) minmax(80px,1fr) minmax(80px,1fr) 60px minmax(100px,1fr) minmax(90px,1fr)",
        }}
      >
        {/* 表头 */}
        <div className="contents">
          {[
            t("traffic.table.time"),
            t("traffic.table.provider"),
            t("traffic.table.model"),
            t("traffic.table.statusCode"),
            t("traffic.table.tokens"),
            t("traffic.table.duration"),
          ].map((col, i) => (
            <div
              key={i}
              className="px-2 py-2 text-xs text-muted-foreground font-medium border-b border-border sticky top-0 bg-background z-10"
            >
              {col}
            </div>
          ))}
        </div>

        {/* 数据行 */}
        {logs.map((log) => {
          const isExpanded = expandedId === log.id;

          return (
            <div key={log.id} className="contents">
              {/* 数据单元格行 */}
              <div
                className="px-2 py-2 flex flex-col justify-center border-b border-border/50 hover:bg-muted/50 cursor-pointer transition-colors text-sm"
                onClick={() => setExpandedId(isExpanded ? null : log.id)}
              >
                <span className="text-xs text-muted-foreground whitespace-nowrap">
                  {renderTime(log.created_at)}
                </span>
              </div>

              <div
                className="px-2 py-2 flex flex-col justify-center border-b border-border/50 hover:bg-muted/50 cursor-pointer transition-colors"
                onClick={() => setExpandedId(isExpanded ? null : log.id)}
              >
                <span className="text-sm truncate">{log.provider_name}</span>
              </div>

              <div
                className="px-2 py-2 flex flex-col justify-center border-b border-border/50 hover:bg-muted/50 cursor-pointer transition-colors"
                onClick={() => setExpandedId(isExpanded ? null : log.id)}
              >
                <span className="text-sm truncate text-muted-foreground">
                  {log.request_model ?? "--"}
                </span>
              </div>

              <div
                className="px-2 py-2 flex flex-col justify-center border-b border-border/50 hover:bg-muted/50 cursor-pointer transition-colors"
                onClick={() => setExpandedId(isExpanded ? null : log.id)}
              >
                <span
                  className={`text-sm font-mono ${statusCodeClass(log.status_code)}`}
                >
                  {log.status_code ?? "--"}
                </span>
              </div>

              {/* Token 列：多行堆叠 */}
              <div
                className="px-2 py-2 flex flex-col justify-center border-b border-border/50 hover:bg-muted/50 cursor-pointer transition-colors"
                onClick={() => setExpandedId(isExpanded ? null : log.id)}
              >
                {log.input_tokens === null && log.output_tokens === null ? (
                  <span className="text-sm text-muted-foreground">
                    {t("traffic.table.placeholder")}
                  </span>
                ) : (
                  <span className="text-sm">
                    {formatTokenCount(log.input_tokens)}
                    {" / "}
                    {formatTokenCount(log.output_tokens)}
                  </span>
                )}
                {log.cache_read_tokens !== null &&
                  log.cache_read_tokens > 0 && (
                    <span className="text-xs text-muted-foreground">
                      {t("traffic.table.cacheRead")}{" "}
                      {formatTokenCount(log.cache_read_tokens)}
                    </span>
                  )}
              </div>

              {/* 耗时列：多行堆叠 */}
              <div
                className="px-2 py-2 flex flex-col justify-center border-b border-border/50 hover:bg-muted/50 cursor-pointer transition-colors"
                onClick={() => setExpandedId(isExpanded ? null : log.id)}
              >
                <span className="text-sm">{formatDuration(log.duration_ms)}</span>
                <span className="text-xs text-muted-foreground">
                  {t("traffic.table.ttfb")} {formatDuration(log.ttfb_ms)}
                </span>
                {log.output_tokens !== null && log.duration_ms !== null && (
                  <span className="text-xs text-muted-foreground">
                    {calcTps(log.output_tokens, log.duration_ms, log.ttfb_ms)}{" "}
                    {t("traffic.table.tps")}
                  </span>
                )}
              </div>

              {/* 展开详情区域（col-span-6） */}
              {isExpanded && (
                <div
                  className="col-span-6 px-4 py-3 bg-muted/50 rounded-md text-xs mb-1"
                  style={{ gridColumn: "1 / -1" }}
                >
                  <dl className="grid grid-cols-2 sm:grid-cols-4 gap-x-6 gap-y-2">
                    <DetailField
                      label={t("traffic.table.protocol")}
                      value={log.protocol_type}
                    />
                    <DetailField
                      label={t("traffic.table.upstreamModel")}
                      value={log.upstream_model}
                    />
                    <DetailField
                      label={t("traffic.table.cli")}
                      value={log.cli_id}
                    />
                    <DetailField
                      label={t("traffic.table.streaming") + " / " + t("traffic.table.nonStreaming")}
                      value={
                        log.is_streaming === 1
                          ? t("traffic.table.streaming")
                          : t("traffic.table.nonStreaming")
                      }
                    />
                    <DetailField
                      label={t("traffic.table.stopReason")}
                      value={log.stop_reason}
                    />
                    <DetailField
                      label="Path"
                      value={log.method + " " + log.path}
                    />
                    {log.error_message && (
                      <div className="col-span-2">
                        <dt className="text-muted-foreground mb-0.5">
                          {t("traffic.table.errorMessage")}
                        </dt>
                        <dd className="text-destructive break-all">
                          {log.error_message}
                        </dd>
                      </div>
                    )}
                  </dl>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
