import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { TrafficLog } from "@/types/traffic";

interface TrafficFilterProps {
  /** 全量日志列表（用于提取 distinct provider） */
  logs: TrafficLog[];
  /** 当前选中的 provider（"__all__" 表示全部） */
  selectedProvider: string;
  /** 筛选变化回调 */
  onFilterChange: (provider: string) => void;
}

export function TrafficFilter({
  logs,
  selectedProvider,
  onFilterChange,
}: TrafficFilterProps) {
  const { t } = useTranslation();

  /** 从全量日志提取 distinct provider 并排序 */
  const providers = useMemo(() => {
    const names = Array.from(new Set(logs.map((l) => l.provider_name)));
    return names.sort();
  }, [logs]);

  return (
    <div className="flex items-center gap-2 px-6 pb-3">
      <span className="text-sm text-muted-foreground">
        {t("traffic.filter.provider")}
      </span>
      <Select value={selectedProvider} onValueChange={onFilterChange}>
        <SelectTrigger className="w-48">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="__all__">
            {t("traffic.filter.allProviders")}
          </SelectItem>
          {providers.map((name) => (
            <SelectItem key={name} value={name}>
              {name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
