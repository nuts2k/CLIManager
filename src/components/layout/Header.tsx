import { Settings } from "lucide-react";
import { Button } from "@/components/ui/button";

interface HeaderProps {
  onNavigate: (view: "main" | "settings") => void;
}

export function Header({ onNavigate }: HeaderProps) {
  return (
    <header className="flex h-12 items-center justify-between border-b border-border bg-background px-4">
      <h1 className="text-base font-semibold">CLIManager</h1>
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
