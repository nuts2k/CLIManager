import { useState, useEffect, useCallback } from "react";
import { Header } from "@/components/layout/Header";
import { ProviderTabs } from "@/components/provider/ProviderTabs";
import { ImportDialog } from "@/components/provider/ImportDialog";
import { SettingsPage } from "@/components/settings/SettingsPage";
import { getLocalSettings, listProviders, scanCliConfigs, syncActiveProviders } from "@/lib/tauri";
import { useSettings } from "@/hooks/useSettings";
import { useSyncListener } from "@/hooks/useSyncListener";
import i18n from "@/i18n";
import type { DetectedCliConfig } from "@/types/provider";

export function AppShell() {
  const [view, setView] = useState<"main" | "settings">("main");
  const [syncKey, setSyncKey] = useState(0);
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [importConfigs, setImportConfigs] = useState<DetectedCliConfig[]>([]);
  const { refresh: refreshSettings } = useSettings();

  const refreshAll = useCallback(async () => {
    setSyncKey((k) => k + 1);
  }, []);

  useSyncListener(refreshAll, refreshSettings);

  // Restore persisted language, sync active providers, and check onboarding on app startup
  useEffect(() => {
    getLocalSettings().then((s) => {
      const lang = s?.language;
      if (lang && lang !== i18n.language) {
        i18n.changeLanguage(lang);
      }
    }).catch(() => {});
    syncActiveProviders().catch(() => {});

    // Onboarding check: if no providers exist, scan for CLI configs
    async function checkOnboarding() {
      try {
        const claudeProviders = await listProviders("claude");
        const codexProviders = await listProviders("codex");
        if (claudeProviders.length === 0 && codexProviders.length === 0) {
          const configs = await scanCliConfigs();
          if (configs.length > 0) {
            setImportConfigs(configs);
            setShowImportDialog(true);
          }
        }
      } catch {
        // Silently ignore onboarding check failures
      }
    }
    checkOnboarding();
  }, []);

  const handleImportComplete = useCallback(() => {
    setSyncKey((k) => k + 1);
  }, []);

  const handleShowImport = useCallback(async () => {
    try {
      const configs = await scanCliConfigs();
      if (configs.length > 0) {
        setImportConfigs(configs);
        setShowImportDialog(true);
      }
    } catch {
      // Silently ignore scan failures
    }
  }, []);

  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <Header onNavigate={setView} />
      <main className="flex-1 overflow-hidden">
        {view === "main" && <ProviderTabs refreshTrigger={syncKey} />}
        {view === "settings" && (
          <SettingsPage
            onBack={() => setView("main")}
            onShowImport={handleShowImport}
          />
        )}
      </main>
      <ImportDialog
        open={showImportDialog}
        onOpenChange={setShowImportDialog}
        configs={importConfigs}
        onImportComplete={handleImportComplete}
      />
    </div>
  );
}
