import { useState, useEffect } from "react";
import { Header } from "@/components/layout/Header";
import { ProviderTabs } from "@/components/provider/ProviderTabs";
import { SettingsPage } from "@/components/settings/SettingsPage";
import { getLocalSettings } from "@/lib/tauri";
import i18n from "@/i18n";

export function AppShell() {
  const [view, setView] = useState<"main" | "settings">("main");

  // Restore persisted language on app startup
  useEffect(() => {
    getLocalSettings().then((s) => {
      const lang = s?.language;
      if (lang && lang !== i18n.language) {
        i18n.changeLanguage(lang);
      }
    }).catch(() => {
      // Settings not available yet, keep default
    });
  }, []);

  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <Header onNavigate={setView} />
      <main className="flex-1 overflow-hidden">
        {view === "main" && <ProviderTabs />}
        {view === "settings" && (
          <SettingsPage onBack={() => setView("main")} />
        )}
      </main>
    </div>
  );
}
