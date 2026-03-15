import { Settings } from "lucide-react";
import { Button } from "@/components/ui/button";

interface HeaderProps {
  onNavigate: (view: "main" | "settings") => void;
}

export function Header({ onNavigate }: HeaderProps) {
  return (
    <header className="flex h-12 items-center justify-between border-b border-border bg-header-bg px-4">
      <div className="flex items-center gap-2">
        <img src="/icon.png" alt="CLIManager" className="size-5" />
        <h1 className="text-base font-bold">
          <span className="text-brand-accent">CLI</span>
          <span>Manager</span>
        </h1>
      </div>
      <Button
        variant="ghost"
        size="icon"
        onClick={() => onNavigate("settings")}
      >
        <Settings className="size-4" />
      </Button>
    </header>
  );
}
