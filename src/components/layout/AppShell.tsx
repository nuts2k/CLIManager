import { useState, useEffect, useCallback } from "react";
import { Header } from "@/components/layout/Header";
import { ProviderTabs } from "@/components/provider/ProviderTabs";
import { SettingsPage } from "@/components/settings/SettingsPage";
import { getLocalSettings, syncActiveProviders } from "@/lib/tauri";
import { useSettings } from "@/hooks/useSettings";
import { useSyncListener } from "@/hooks/useSyncListener";
import i18n from "@/i18n";

export function AppShell() {
  const [view, setView] = useState<"main" | "settings">("main");
  const [syncKey, setSyncKey] = useState(0);
  const { refresh: refreshSettings } = useSettings();

  const refreshAll = useCallback(async () => {
    setSyncKey((k) => k + 1);
  }, []);

  useSyncListener(refreshAll, refreshSettings);

  // Restore persisted language and sync active providers on app startup
  useEffect(() => {
    getLocalSettings().then((s) => {
      const lang = s?.language;
      if (lang && lang !== i18n.language) {
        i18n.changeLanguage(lang);
      }
    }).catch(() => {});
    syncActiveProviders().catch(() => {});
  }, []);

  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <Header onNavigate={setView} />
      <main className="flex-1 overflow-hidden">
        {view === "main" && <ProviderTabs refreshTrigger={syncKey} />}
        {view === "settings" && (
          <SettingsPage onBack={() => setView("main")} />
        )}
      </main>
    </div>
  );
}
