import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { listProviders, importProvider, refreshTrayMenu } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import type { DetectedCliConfig } from "@/types/provider";

interface ImportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  configs: DetectedCliConfig[];
  onImportComplete: () => void;
}

/** Config is complete when all required fields are present */
function isConfigComplete(config: DetectedCliConfig): boolean {
  return config.has_api_key && config.base_url.trim().length > 0;
}

function maskApiKey(key: string): string {
  if (key.length <= 8) {
    return key.length >= 4
      ? `${key.slice(0, 2)}...${key.slice(-2)}`
      : key;
  }
  return `${key.slice(0, 8)}...${key.slice(-4)}`;
}

export function ImportDialog({
  open,
  onOpenChange,
  configs,
  onImportComplete,
}: ImportDialogProps) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<Record<number, boolean>>({});
  const [importing, setImporting] = useState(false);

  // Default: only select configs with all required fields present
  const effectiveSelected = useMemo(() => {
    const result: Record<number, boolean> = {};
    configs.forEach((config, i) => {
      const isComplete = isConfigComplete(config);
      result[i] = isComplete && (selected[i] ?? true);
    });
    return result;
  }, [configs, selected]);

  const hasSelection = Object.values(effectiveSelected).some(Boolean);

  const handleToggle = (index: number, checked: boolean) => {
    setSelected((prev) => ({ ...prev, [index]: checked }));
  };

  const handleImport = async () => {
    setImporting(true);
    try {
      // Fetch existing providers for dedup
      const existing = await listProviders();
      let importCount = 0;

      for (let i = 0; i < configs.length; i++) {
        if (!effectiveSelected[i]) continue;

        const config = configs[i];
        if (!isConfigComplete(config)) continue;

        // Dedup check: skip if any existing provider has same api_key AND base_url
        const isDuplicate = existing.some(
          (p) => p.api_key === config.api_key && p.base_url === config.base_url,
        );
        if (isDuplicate) continue;

        const name = `${config.cli_name} ${t("import.defaultSuffix")}`;
        await importProvider({
          name,
          protocolType: config.protocol_type,
          apiKey: config.api_key,
          baseUrl: config.base_url,
          cliId: config.cli_id,
        });
        importCount++;
      }

      if (importCount > 0) {
        toast.success(t("import.importSuccess", { count: importCount }));
      } else {
        toast.info(t("import.noNewConfigs"));
      }

      onImportComplete();
      refreshTrayMenu().catch(() => {});
      setImporting(false);
      onOpenChange(false);
    } catch (err) {
      toast.error(t("import.importError", { error: String(err) }));
      setImporting(false);
    }
  };

  const handleSkip = () => {
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={importing ? undefined : onOpenChange}>
      <DialogContent
        showCloseButton={!importing}
        onInteractOutside={(e) => {
          if (importing) e.preventDefault();
        }}
        className="sm:max-w-md"
      >
        <DialogHeader>
          <DialogTitle>{t("import.title")}</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-3 py-2">
          {configs.map((config, index) => {
            const isComplete = isConfigComplete(config);

            return (
              <label
                key={index}
                className={cn(
                  "flex items-center gap-3 rounded-md border border-border p-3",
                  isComplete
                    ? "cursor-pointer hover:bg-accent/50"
                    : "cursor-not-allowed opacity-60",
                )}
              >
                <Checkbox
                  checked={effectiveSelected[index]}
                  onCheckedChange={(checked) =>
                    handleToggle(index, checked === true)
                  }
                  disabled={importing || !isComplete}
                />
                <div className="flex flex-col gap-0.5 min-w-0 flex-1">
                  <span className="text-sm font-medium">{config.cli_name}</span>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    {config.has_api_key ? (
                      <span className="font-mono truncate">
                        {maskApiKey(config.api_key)}
                      </span>
                    ) : (
                      <span className="text-status-warning">
                        {t("import.missingApiKey")}
                      </span>
                    )}
                    <span className="text-border">|</span>
                    {config.base_url ? (
                      <span className="truncate">
                        {config.base_url}
                      </span>
                    ) : (
                      <span className="text-status-warning">
                        {t("import.missingBaseUrl")}
                      </span>
                    )}
                  </div>
                </div>
              </label>
            );
          })}
        </div>

        <DialogFooter>
          <Button
            variant="ghost"
            onClick={handleSkip}
            disabled={importing}
          >
            {t("import.skip")}
          </Button>
          <Button
            onClick={handleImport}
            disabled={importing || !hasSelection}
          >
            {importing && <Loader2 className="size-4 animate-spin" />}
            {importing ? t("import.importing") : t("import.importSelected")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
