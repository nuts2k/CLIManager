import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  listProviders,
  createProvider,
  deleteProvider,
  setActiveProvider,
  testProvider,
  updateProvider,
  refreshTrayMenu,
} from "@/lib/tauri";
import type { Provider, CreateProviderInput } from "@/types/provider";

export function useProviders(cliId: string) {
  const { t } = useTranslation();
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(true);
  const [operationLoading, setOperationLoading] = useState<string | null>(null);
  const cliIdRef = useRef(cliId);
  cliIdRef.current = cliId;

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await listProviders(cliIdRef.current);
      setProviders(data);
    } catch (err) {
      console.error("Failed to load providers:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const switchProvider = useCallback(
    async (providerId: string, refreshSettings?: () => Promise<void>) => {
      const provider = providers.find((p) => p.id === providerId);
      setOperationLoading(providerId);
      try {
        await setActiveProvider(cliIdRef.current, providerId);
        if (refreshSettings) await refreshSettings();
        await refresh();
        refreshTrayMenu().catch(() => {});
        toast.success(t("status.switchSuccess", { name: provider?.name }));
      } catch (err) {
        toast.error(
          t("status.switchError", { error: String(err) }),
        );
      } finally {
        setOperationLoading(null);
      }
    },
    [providers, refresh, t],
  );

  const removeProvider = useCallback(
    async (id: string, refreshSettings?: () => Promise<void>) => {
      const provider = providers.find((p) => p.id === id);
      setOperationLoading(id);
      try {
        await deleteProvider(id);
        if (refreshSettings) await refreshSettings();
        await refresh();
        refreshTrayMenu().catch(() => {});
        toast.success(t("status.deleteSuccess", { name: provider?.name }));
      } catch (err) {
        toast.error(String(err));
      } finally {
        setOperationLoading(null);
      }
    },
    [providers, refresh, t],
  );

  const copyProvider = useCallback(
    async (provider: Provider) => {
      setOperationLoading(provider.id);
      try {
        const input: CreateProviderInput = {
          name: `${provider.name} (copy)`,
          protocolType: provider.protocol_type,
          apiKey: provider.api_key,
          baseUrl: provider.base_url,
          model: provider.model,
          cliId: provider.cli_id,
        };
        const created = await createProvider(input);
        if (provider.notes != null || provider.model_config != null) {
          await updateProvider({
            ...created,
            notes: provider.notes ?? null,
            model_config: provider.model_config ?? null,
          });
        }
        await refresh();
        refreshTrayMenu().catch(() => {});
        toast.success(t("status.copySuccess", { name: provider.name }));
      } catch (err) {
        toast.error(String(err));
      } finally {
        setOperationLoading(null);
      }
    },
    [refresh, t],
  );

  const copyProviderTo = useCallback(
    async (provider: Provider, targetCliId: string, sourceTabName: string) => {
      setOperationLoading(provider.id);
      try {
        const input: CreateProviderInput = {
          name: `${provider.name} (copy from ${sourceTabName})`,
          protocolType: provider.protocol_type,
          apiKey: provider.api_key,
          baseUrl: provider.base_url,
          model: provider.model,
          cliId: targetCliId,
        };
        const created = await createProvider(input);
        if (provider.notes != null || provider.model_config != null) {
          await updateProvider({
            ...created,
            notes: provider.notes ?? null,
            model_config: provider.model_config ?? null,
          });
        }
        if (targetCliId === cliIdRef.current) {
          await refresh();
        }
        refreshTrayMenu().catch(() => {});
        toast.success(t("status.copySuccess", { name: provider.name }));
      } catch (err) {
        toast.error(String(err));
      } finally {
        setOperationLoading(null);
      }
    },
    [refresh, t],
  );

  const testProviderConnection = useCallback(
    async (providerId: string) => {
      setOperationLoading(providerId);
      try {
        const result = await testProvider(providerId);
        if (result.success) {
          toast.success(t("status.testSuccess", { time: result.elapsed_ms }));
        } else {
          toast.error(
            t("status.testError", { error: result.error ?? "Unknown" }),
          );
        }
      } catch (err) {
        toast.error(t("status.testError", { error: String(err) }));
      } finally {
        setOperationLoading(null);
      }
    },
    [t],
  );

  useEffect(() => {
    refresh();
  }, [cliId, refresh]);

  return {
    providers,
    loading,
    operationLoading,
    refresh,
    switchProvider,
    removeProvider,
    copyProvider,
    copyProviderTo,
    testProviderConnection,
  };
}
