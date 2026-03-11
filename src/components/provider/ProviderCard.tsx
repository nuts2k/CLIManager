import { useTranslation } from "react-i18next";
import { Loader2, MoreVertical } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { Provider } from "@/types/provider";

const CLI_TABS: { id: string; label: string }[] = [
  { id: "claude", label: "Claude Code" },
  { id: "codex", label: "Codex" },
];

interface ProviderCardProps {
  provider: Provider;
  isActive: boolean;
  currentCliId: string;
  operationLoading: string | null;
  onSwitch: () => void;
  onEdit: () => void;
  onCopy: () => void;
  onCopyTo: (targetCliId: string) => void;
  onTest: () => void;
  onDelete: () => void;
}

export function ProviderCard({
  provider,
  isActive,
  currentCliId,
  operationLoading,
  onSwitch,
  onEdit,
  onCopy,
  onCopyTo,
  onTest,
  onDelete,
}: ProviderCardProps) {
  const { t } = useTranslation();
  const isLoading = operationLoading === provider.id;
  const otherClis = CLI_TABS.filter((cli) => cli.id !== currentCliId);

  const truncatedUrl =
    provider.base_url.length > 40
      ? provider.base_url.slice(0, 40) + "..."
      : provider.base_url;

  return (
    <div
      className={`group relative flex items-center gap-3 rounded-lg border px-4 py-3 transition-colors ${
        isActive
          ? "border-blue-500/50 bg-blue-500/5"
          : "border-border hover:border-border/80 hover:bg-accent/30"
      }`}
    >
      {/* Active indicator bar */}
      <div
        className={`absolute left-0 top-2 bottom-2 w-1 rounded-full transition-colors ${
          isActive ? "bg-blue-500" : "bg-transparent"
        }`}
      />

      {/* Content */}
      <div className="min-w-0 flex-1 pl-1">
        <div className="flex items-center gap-2">
          <span className="font-medium truncate">{provider.name}</span>
          {isActive && (
            <Badge variant="secondary" className="text-xs">
              {t("status.active")}
            </Badge>
          )}
        </div>
        <p className="mt-0.5 text-xs text-muted-foreground truncate">
          {truncatedUrl}
        </p>
      </div>

      {/* Action buttons (hover reveal) */}
      <div className="flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
        {isLoading && <Loader2 className="size-4 animate-spin text-muted-foreground" />}

        {!isActive && (
          <Button
            variant="default"
            size="sm"
            onClick={onSwitch}
            disabled={isLoading}
          >
            {t("actions.switch")}
          </Button>
        )}

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon-sm" disabled={isLoading}>
              <MoreVertical className="size-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={onEdit}>
              {t("actions.edit")}
            </DropdownMenuItem>
            <DropdownMenuItem onClick={onCopy}>
              {t("actions.copy")}
            </DropdownMenuItem>
            {otherClis.length > 0 && (
              <DropdownMenuSub>
                <DropdownMenuSubTrigger>
                  {t("actions.copyTo")}
                </DropdownMenuSubTrigger>
                <DropdownMenuSubContent>
                  {otherClis.map((cli) => (
                    <DropdownMenuItem
                      key={cli.id}
                      onClick={() => onCopyTo(cli.id)}
                    >
                      {cli.label}
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuSubContent>
              </DropdownMenuSub>
            )}
            <DropdownMenuItem onClick={onTest}>
              {t("actions.test")}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" onClick={onDelete}>
              {t("actions.delete")}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}
