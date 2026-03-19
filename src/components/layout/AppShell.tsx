import { useState, useEffect, useCallback, useRef } from "react";
import { Header } from "@/components/layout/Header";
import { ProviderTabs } from "@/components/provider/ProviderTabs";
import { ImportDialog } from "@/components/provider/ImportDialog";
import { SettingsPage } from "@/components/settings/SettingsPage";
import { TrafficPage } from "@/components/traffic/TrafficPage";
import { UpdateDialog } from "@/components/updater/UpdateDialog";
import { useUpdater } from "@/components/updater/useUpdater";
import { getLocalSettings, listProviders, scanCliConfigs, syncActiveProviders } from "@/lib/tauri";
import { useSettings } from "@/hooks/useSettings";
import { useSyncListener } from "@/hooks/useSyncListener";
import i18n from "@/i18n";
import type { DetectedCliConfig } from "@/types/provider";

type AppView = "main" | "traffic" | "settings";

const VIEW_TRANSITION_MS = 150;

export function AppShell() {
  const [view, setView] = useState<AppView>("main");
  const [exitingView, setExitingView] = useState<AppView | null>(null);
  const [syncKey, setSyncKey] = useState(0);
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [importConfigs, setImportConfigs] = useState<DetectedCliConfig[]>([]);
  const [showUpdateDialog, setShowUpdateDialog] = useState(false);
  const viewTransitionTimerRef = useRef<ReturnType<typeof globalThis.setTimeout> | null>(null);
  const { refresh: refreshSettings } = useSettings();

  // 更新检查
  const updater = useUpdater();

  // 当检测到新版本时自动弹出更新对话框
  useEffect(() => {
    if (updater.status === "available") {
      setShowUpdateDialog(true);
    }
  }, [updater.status]);

  const refreshAll = useCallback(async () => {
    setSyncKey((k) => k + 1);
  }, []);

  useSyncListener(refreshAll, refreshSettings);

  // Restore persisted language, sync active providers, and check onboarding on app startup
  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      try {
        const settings = await getLocalSettings();
        const lang = settings?.language;
        if (!cancelled && lang && lang !== i18n.language) {
          i18n.changeLanguage(lang);
        }
      } catch {
        // Silently ignore persisted language failures
      }

      try {
        await syncActiveProviders();
      } catch {
        // Silently ignore active provider sync failures
      }

      if (cancelled) {
        return;
      }

      await refreshSettings();
      await refreshAll();

      // Onboarding check: if no providers exist, scan for CLI configs
      try {
        const claudeProviders = await listProviders("claude");
        const codexProviders = await listProviders("codex");
        if (cancelled) {
          return;
        }
        if (claudeProviders.length === 0 && codexProviders.length === 0) {
          const configs = await scanCliConfigs();
          if (cancelled) {
            return;
          }
          if (configs.length > 0) {
            setImportConfigs(configs);
            setShowImportDialog(true);
          }
        }
      } catch {
        // Silently ignore onboarding check failures
      }

      // 启动时检查更新（静默，失败不影响主流程）
      updater.checkForUpdate().catch(() => {
        // 已在 hook 内部 catch，这里双保险
      });
    }

    bootstrap();

    return () => {
      cancelled = true;
    };
  }, [refreshAll, refreshSettings, updater.checkForUpdate]);

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

  useEffect(() => {
    return () => {
      if (viewTransitionTimerRef.current) {
        clearTimeout(viewTransitionTimerRef.current);
      }
    };
  }, []);

  const handleNavigate = useCallback((nextView: AppView) => {
    if (nextView === view) {
      return;
    }

    const leavingView = view;

    if (viewTransitionTimerRef.current) {
      clearTimeout(viewTransitionTimerRef.current);
    }

    setExitingView(leavingView);
    setView(nextView);

    viewTransitionTimerRef.current = globalThis.setTimeout(() => {
      setExitingView((current) => (current === leavingView ? null : current));
      viewTransitionTimerRef.current = null;
    }, VIEW_TRANSITION_MS);
  }, [view]);

  const showMainView = view === "main" || exitingView === "main";
  const showTrafficView = view === "traffic" || exitingView === "traffic";
  const showSettingsView = view === "settings" || exitingView === "settings";

  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <Header onNavigate={handleNavigate} currentView={view} />
      <main className="relative flex-1 overflow-hidden">
        {showMainView ? (
          <div
            inert={view !== "main"}
            aria-hidden={view !== "main"}
            className={`absolute inset-0 transition-opacity duration-150 ease-out ${
              view === "main" ? "opacity-100" : "opacity-0 pointer-events-none"
            }`}
          >
            <ProviderTabs refreshTrigger={syncKey} />
          </div>
        ) : null}
        {showTrafficView ? (
          <div
            inert={view !== "traffic"}
            aria-hidden={view !== "traffic"}
            className={`absolute inset-0 transition-opacity duration-150 ease-out ${
              view === "traffic" ? "opacity-100" : "opacity-0 pointer-events-none"
            }`}
          >
            <TrafficPage />
          </div>
        ) : null}
        {showSettingsView ? (
          <div
            inert={view !== "settings"}
            aria-hidden={view !== "settings"}
            className={`absolute inset-0 transition-opacity duration-150 ease-out ${
              view === "settings" ? "opacity-100" : "opacity-0 pointer-events-none"
            }`}
          >
            <SettingsPage
              onShowImport={handleShowImport}
            />
          </div>
        ) : null}
      </main>
      <ImportDialog
        open={showImportDialog}
        onOpenChange={setShowImportDialog}
        configs={importConfigs}
        onImportComplete={handleImportComplete}
      />
      <UpdateDialog
        open={showUpdateDialog}
        onOpenChange={setShowUpdateDialog}
        status={updater.status}
        currentVersion={updater.currentVersion}
        newVersion={updater.newVersion}
        progress={updater.progress}
        error={updater.error}
        onUpdate={updater.downloadAndInstall}
        onRemindLater={() => {
          updater.dismissUpdate();
          setShowUpdateDialog(false);
        }}
      />
    </div>
  );
}
