import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Plus } from "lucide-react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { ProviderList } from "@/components/provider/ProviderList";
import {
  ProviderDialog,
  type ProviderFormData,
} from "@/components/provider/ProviderDialog";
import { DeleteConfirmDialog } from "@/components/provider/DeleteConfirmDialog";
import { useProviders } from "@/hooks/useProviders";
import { useSettings } from "@/hooks/useSettings";
import { useProxyStatus } from "@/hooks/useProxyStatus";
import { createProvider, updateProvider, refreshTrayMenu, proxyEnable, proxyDisable } from "@/lib/tauri";
import type { Provider } from "@/types/provider";

const CLI_TABS = [
  { id: "claude", labelKey: "tabs.claude" },
  { id: "codex", labelKey: "tabs.codex" },
] as const;

interface ProviderTabsProps {
  refreshTrigger?: number;
}

export function ProviderTabs({ refreshTrigger }: ProviderTabsProps) {
  const { t } = useTranslation();
  const [currentCliId, setCurrentCliId] = useState<string>("claude");
  const {
    providers,
    loading,
    operationLoading,
    refresh,
    switchProvider,
    removeProvider,
    copyProvider,
    copyProviderTo,
    testProviderConnection,
  } = useProviders(currentCliId);
  const { getActiveProviderId, refresh: refreshSettings } = useSettings();
  const { proxyStatus } = useProxyStatus();

  // 当前 CLI 的代理状态
  const cliStatus = proxyStatus?.cli_statuses.find(
    (s) => s.cli_id === currentCliId,
  );
  const globalEnabled = proxyStatus?.global_enabled ?? false;
  const cliProxyActive = cliStatus?.active ?? false;
  const hasProvider = cliStatus?.has_provider ?? false;
  const switchDisabled = !globalEnabled || !hasProvider;
  const tooltipText = !globalEnabled
    ? t("proxy.globalDisabled")
    : !hasProvider
      ? t("proxy.noProvider")
      : "";

  const handleCliProxyToggle = async () => {
    try {
      if (cliProxyActive) {
        await proxyDisable(currentCliId);
        toast.success(t("proxy.disableSuccess"));
      } else {
        await proxyEnable(currentCliId);
        toast.success(t("proxy.enableSuccess"));
      }
    } catch (err) {
      const errorStr = String(err);
      if (
        errorStr.includes("绑定失败") ||
        errorStr.includes("Address already in use") ||
        errorStr.includes("address already in use")
      ) {
        const port = cliStatus?.port ?? "unknown";
        toast.error(t("proxy.portInUse", { port }));
      } else {
        toast.error(t("proxy.enableFailed") + ": " + errorStr);
      }
    }
  };

  // Re-fetch when sync trigger changes (skip initial render)
  const isInitialMount = useRef(true);
  useEffect(() => {
    if (isInitialMount.current) {
      isInitialMount.current = false;
      return;
    }
    refresh();
    refreshSettings();
  }, [refreshTrigger, refresh, refreshSettings]);

  // Dialog state
  const [dialogMode, setDialogMode] = useState<"create" | "edit" | null>(null);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [deletingProvider, setDeletingProvider] = useState<Provider | null>(
    null,
  );

  const activeProviderId = getActiveProviderId(currentCliId);
  const currentTabLabel =
    CLI_TABS.find((tab) => tab.id === currentCliId)?.labelKey ?? currentCliId;

  const handleSwitch = (providerId: string) => {
    switchProvider(providerId, refreshSettings);
  };

  const handleDelete = (provider: Provider) => {
    setDeletingProvider(provider);
  };

  const handleConfirmDelete = async () => {
    if (!deletingProvider) return;
    await removeProvider(deletingProvider.id, refreshSettings);
    setDeletingProvider(null);
  };

  const handleEdit = (provider: Provider) => {
    setEditingProvider(provider);
    setDialogMode("edit");
  };

  const handleCopy = (provider: Provider) => {
    copyProvider(provider);
  };

  const handleCopyTo = (provider: Provider, targetCliId: string) => {
    copyProviderTo(provider, targetCliId, t(currentTabLabel));
  };

  const handleTest = (providerId: string) => {
    testProviderConnection(providerId);
  };

  const handleCreate = () => {
    setEditingProvider(null);
    setDialogMode("create");
  };

  const handleSave = async (data: ProviderFormData) => {
    try {
      const modelConfig =
        data.haikuModel || data.sonnetModel || data.opusModel || data.reasoningEffort
          ? {
              haiku_model: data.haikuModel || null,
              sonnet_model: data.sonnetModel || null,
              opus_model: data.opusModel || null,
              reasoning_effort: data.reasoningEffort || null,
            }
          : null;

      if (dialogMode === "create") {
        const created = await createProvider({
          name: data.name,
          protocolType: data.protocolType,
          apiKey: data.apiKey,
          baseUrl: data.baseUrl,
          model: data.model,
          cliId: currentCliId,
        });
        // If model_config or notes need to be set, do an update right after creation
        if (modelConfig || data.notes) {
          await updateProvider({
            ...created,
            model_config: modelConfig,
            notes: data.notes || null,
          });
        }
        toast.success(t("status.createSuccess", { name: data.name }));
      } else if (dialogMode === "edit" && editingProvider) {
        await updateProvider({
          ...editingProvider,
          name: data.name,
          api_key: data.apiKey,
          base_url: data.baseUrl,
          model: data.model,
          protocol_type: data.protocolType,
          notes: data.notes || null,
          model_config: modelConfig,
        });
        toast.success(t("status.updateSuccess", { name: data.name }));
      }

      await refreshSettings();
      await refresh();
      refreshTrayMenu().catch(() => {});
    } catch (err) {
      toast.error(String(err));
    } finally {
      setDialogMode(null);
      setEditingProvider(null);
    }
  };

  return (
    <div className="flex h-full flex-col p-4">
      <Tabs
        value={currentCliId}
        onValueChange={(v) => setCurrentCliId(v)}
        className="flex flex-1 flex-col"
      >
        <div className="flex items-center justify-between">
          <TabsList>
            {CLI_TABS.map((tab) => (
              <TabsTrigger key={tab.id} value={tab.id}>
                <span className="flex items-center gap-1.5">
                  {t(tab.labelKey)}
                  {proxyStatus?.cli_statuses.find((s) => s.cli_id === tab.id)
                    ?.active && (
                    <span className="size-2 rounded-full bg-green-500" />
                  )}
                </span>
              </TabsTrigger>
            ))}
          </TabsList>
          <Button size="sm" onClick={handleCreate}>
            <Plus className="size-4" />
            {t("actions.create")}
          </Button>
        </div>

        {/* CLI 代理开关行 */}
        <div className="flex items-center gap-3 px-1 py-2">
          <span className="text-sm text-muted-foreground">
            {t("settings.proxyMode")}
          </span>
          {switchDisabled ? (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span>
                    <Switch checked={cliProxyActive} disabled />
                  </span>
                </TooltipTrigger>
                <TooltipContent>
                  <p>{tooltipText}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          ) : (
            <Switch
              checked={cliProxyActive}
              onCheckedChange={handleCliProxyToggle}
            />
          )}
        </div>

        {CLI_TABS.map((tab) => (
          <TabsContent key={tab.id} value={tab.id} className="flex-1">
            <ProviderList
              providers={providers}
              activeProviderId={activeProviderId}
              loading={loading}
              currentCliId={currentCliId}
              operationLoading={operationLoading}
              onCreate={handleCreate}
              onSwitch={handleSwitch}
              onEdit={handleEdit}
              onCopy={handleCopy}
              onCopyTo={handleCopyTo}
              onTest={handleTest}
              onDelete={handleDelete}
            />
          </TabsContent>
        ))}
      </Tabs>

      <ProviderDialog
        open={dialogMode !== null}
        onOpenChange={(open) => {
          if (!open) {
            setDialogMode(null);
            setEditingProvider(null);
          }
        }}
        mode={dialogMode ?? "create"}
        provider={editingProvider}
        cliId={currentCliId}
        onSave={handleSave}
      />

      <DeleteConfirmDialog
        open={deletingProvider !== null}
        onOpenChange={(open) => {
          if (!open) setDeletingProvider(null);
        }}
        providerName={deletingProvider?.name ?? ""}
        onConfirm={handleConfirmDelete}
      />
    </div>
  );
}
