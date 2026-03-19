import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useTrafficLogs } from "@/hooks/useTrafficLogs";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { TrafficStatsBar } from "./TrafficStatsBar";
import { TrafficFilter } from "./TrafficFilter";
import { TrafficTable } from "./TrafficTable";
import { TrafficEmptyState } from "./TrafficEmptyState";
import { StatsAnalysisTab } from "./StatsAnalysisTab";

export function TrafficPage() {
  const { t } = useTranslation();
  const { logs, loading, dbError } = useTrafficLogs();

  /** 当前选中的 Provider 筛选（"__all__" 表示全部） */
  const [selectedProvider, setSelectedProvider] = useState("__all__");

  /** 筛选后的日志列表，传给表格和统计卡片 */
  const filteredLogs = useMemo(() => {
    if (selectedProvider === "__all__") return logs;
    return logs.filter((l) => l.provider_name === selectedProvider);
  }, [logs, selectedProvider]);

  return (
    <div className="flex flex-col h-full">
      <Tabs defaultValue="logs" className="flex flex-col h-full">
        {/* 页面标题 + Tab 切换行 */}
        <div className="flex items-center px-6 pt-4 pb-2 gap-4">
          <h2 className="text-lg font-bold">{t("traffic.title")}</h2>
          <TabsList variant="line">
            <TabsTrigger value="logs">{t("traffic.tabLogs")}</TabsTrigger>
            <TabsTrigger value="stats">{t("traffic.tabStats")}</TabsTrigger>
          </TabsList>
        </div>

        {/* 实时日志 Tab：保留所有现有功能（统计卡片 + 筛选 + 表格） */}
        <TabsContent value="logs" className="flex flex-col flex-1 min-h-0">
          {/* DB 不可用时展示内联警告 banner（持续可见，与 toast 不同不会自动消失） */}
          {dbError && (
            <div className="mx-6 mt-2 px-4 py-3 rounded-md bg-destructive/10 border border-destructive/20 text-sm text-destructive">
              <span className="font-medium">{t("traffic.dbErrorTitle")}</span>
              <span className="ml-1 text-destructive/80">{t("traffic.dbErrorDesc")}</span>
            </div>
          )}

          {/* 5 张统计摘要卡片（基于筛选后日志） */}
          <TrafficStatsBar logs={filteredLogs} />

          {/* Provider 筛选下拉框（基于全量日志提取 provider 列表） */}
          <TrafficFilter
            logs={logs}
            selectedProvider={selectedProvider}
            onFilterChange={setSelectedProvider}
          />

          {/* 表格区域 / 空状态 */}
          <div className="flex-1 min-h-0 px-6 pb-4">
            {loading ? (
              /* 加载中：简洁骨架提示 */
              <div className="flex items-center justify-center h-full">
                <span className="text-sm text-muted-foreground animate-pulse">
                  Loading...
                </span>
              </div>
            ) : filteredLogs.length === 0 ? (
              <TrafficEmptyState />
            ) : (
              <TrafficTable logs={filteredLogs} />
            )}
          </div>
        </TabsContent>

        {/* 统计分析 Tab */}
        <TabsContent value="stats" className="flex-1 min-h-0 overflow-auto px-6 pb-4 scrollbar-thin">
          <StatsAnalysisTab />
        </TabsContent>
      </Tabs>
    </div>
  );
}
