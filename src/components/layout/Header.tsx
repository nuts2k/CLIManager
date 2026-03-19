import { useTranslation } from "react-i18next";
import { Home, Activity, Settings } from "lucide-react";

type AppView = "main" | "traffic" | "settings";

interface HeaderProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
}

const NAV_ITEMS: { view: AppView; icon: typeof Home; labelKey: string }[] = [
  { view: "main", icon: Home, labelKey: "nav.home" },
  { view: "traffic", icon: Activity, labelKey: "nav.traffic" },
  { view: "settings", icon: Settings, labelKey: "nav.settings" },
];

export function Header({ currentView, onNavigate }: HeaderProps) {
  const { t } = useTranslation();

  return (
    <header className="flex h-12 items-center border-b border-border bg-header-bg px-4">
      {/* Logo */}
      <div className="flex items-center gap-2 mr-6">
        <img src="/icon.png" alt="CLIManager" className="size-5" />
        <h1 className="text-base font-bold">
          <span className="text-brand-accent">CLI</span>
          <span>Manager</span>
        </h1>
      </div>

      {/* 导航标签 */}
      <nav className="flex items-center h-full">
        {NAV_ITEMS.map(({ view, icon: Icon, labelKey }) => {
          const active = currentView === view;
          return (
            <button
              key={view}
              onClick={() => onNavigate(view)}
              className={[
                "relative flex items-center gap-1.5 px-3 h-full text-sm transition-colors",
                active
                  ? "text-foreground"
                  : "text-muted-foreground hover:text-foreground/80",
              ].join(" ")}
            >
              <Icon className="size-3.5" />
              <span>{t(labelKey)}</span>
              {/* 选中态底部指示线 */}
              {active && (
                <span className="absolute bottom-0 left-2 right-2 h-0.5 rounded-full bg-brand-accent" />
              )}
            </button>
          );
        })}
      </nav>
    </header>
  );
}
