import { useState } from "react";
import { Header } from "@/components/layout/Header";
import { ProviderTabs } from "@/components/provider/ProviderTabs";

export function AppShell() {
  const [view, setView] = useState<"main" | "settings">("main");

  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <Header onNavigate={setView} />
      <main className="flex-1 overflow-hidden">
        {view === "main" && <ProviderTabs />}
        {view === "settings" && (
          <div className="flex h-full items-center justify-center p-8">
            <p className="text-muted-foreground">Settings (coming soon)</p>
          </div>
        )}
      </main>
    </div>
  );
}
