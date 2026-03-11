import { useState, useEffect, useCallback } from "react";
import { getLocalSettings, updateLocalSettings } from "@/lib/tauri";
import type { LocalSettings } from "@/types/settings";

export function useSettings() {
  const [settings, setSettings] = useState<LocalSettings | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getLocalSettings();
      setSettings(data);
    } catch (err) {
      console.error("Failed to load settings:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const updateSettings = useCallback(
    async (partial: Partial<LocalSettings>) => {
      if (!settings) return;
      const merged = { ...settings, ...partial };
      try {
        const updated = await updateLocalSettings(merged);
        setSettings(updated);
        return updated;
      } catch (err) {
        console.error("Failed to update settings:", err);
        throw err;
      }
    },
    [settings],
  );

  const getActiveProviderId = useCallback(
    (cliId: string): string | null => {
      return settings?.active_providers[cliId] ?? null;
    },
    [settings],
  );

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { settings, loading, refresh, updateSettings, getActiveProviderId };
}
