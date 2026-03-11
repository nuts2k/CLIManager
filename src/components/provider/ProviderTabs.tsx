import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Plus } from "lucide-react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { ProviderList } from "@/components/provider/ProviderList";
import {
  ProviderDialog,
  type ProviderFormData,
} from "@/components/provider/ProviderDialog";
import { DeleteConfirmDialog } from "@/components/provider/DeleteConfirmDialog";
import { useProviders } from "@/hooks/useProviders";
import { useSettings } from "@/hooks/useSettings";
import { createProvider, updateProvider } from "@/lib/tauri";
import type { Provider } from "@/types/provider";

const CLI_TABS = [
  { id: "claude", labelKey: "tabs.claude" },
  { id: "codex", labelKey: "tabs.codex" },
] as const;

export function ProviderTabs() {
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
    setDialogMode(null);
    setEditingProvider(null);
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
                {t(tab.labelKey)}
              </TabsTrigger>
            ))}
          </TabsList>
          <Button size="sm" onClick={handleCreate}>
            <Plus className="size-4" />
            {t("actions.create")}
          </Button>
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
