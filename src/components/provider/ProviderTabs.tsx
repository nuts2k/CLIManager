import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus } from "lucide-react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { ProviderList } from "@/components/provider/ProviderList";
import { useProviders } from "@/hooks/useProviders";
import { useSettings } from "@/hooks/useSettings";
import type { Provider } from "@/types/provider";

const CLI_TABS = [
  { id: "claude", labelKey: "tabs.claude" },
  { id: "codex", labelKey: "tabs.codex" },
] as const;

export function ProviderTabs() {
  const { t } = useTranslation();
  const [currentCliId, setCurrentCliId] = useState<string>("claude");
  const providerHook = useProviders(currentCliId);
  const {
    providers,
    loading,
    operationLoading,
    switchProvider,
    copyProvider,
    copyProviderTo,
    testProviderConnection,
  } = providerHook;
  const { getActiveProviderId, refresh: refreshSettings } = useSettings();

  // Dialog state -- placeholder setters until Task 2 wires real dialogs
  const [, setDialogMode] = useState<"create" | "edit" | null>(null);
  const [, setEditingProvider] = useState<Provider | null>(null);
  const [, setDeletingProvider] = useState<Provider | null>(null);

  const activeProviderId = getActiveProviderId(currentCliId);
  const currentTabLabel =
    CLI_TABS.find((tab) => tab.id === currentCliId)?.labelKey ?? currentCliId;

  const handleSwitch = (providerId: string) => {
    switchProvider(providerId, refreshSettings);
  };

  const handleDelete = (provider: Provider) => {
    setDeletingProvider(provider);
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

      {/* Dialogs will be rendered here in Task 2 */}
      {/* ProviderDialog: dialogMode, editingProvider */}
      {/* DeleteConfirmDialog: deletingProvider */}
    </div>
  );
}
