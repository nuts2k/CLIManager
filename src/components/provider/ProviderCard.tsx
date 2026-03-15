import { useTranslation } from "react-i18next";
import { Loader2, MoreVertical, Pencil, Copy, Play, Trash2, ArrowRightLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { DropdownMenuItem } from "@/components/ui/dropdown-menu";
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
      className={`group relative flex items-center gap-3 rounded-lg border px-4 py-3 shadow-sm transition-all duration-200 hover:shadow-md hover:-translate-y-0.5 ${
        isActive
          ? "border-status-active/50 bg-status-active/5 hover:border-status-active/70"
          : "border-border hover:border-border/80 hover:bg-accent/30"
      }`}
    >
      {/* 活跃指示条 */}
      <div
        className={`absolute left-0 top-2 bottom-2 w-1 rounded-full transition-colors ${
          isActive ? "bg-status-active" : "bg-transparent"
        }`}
      />

      {/* 卡片内容 */}
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

      {/* 操作按钮区域（始终可见） */}
      <TooltipProvider delayDuration={300}>
        <div className="flex items-center gap-0.5">
          {isLoading && <Loader2 className="size-4 animate-spin text-muted-foreground mr-1" />}

          {/* 切换按钮（仅非活跃卡片显示） */}
          {!isActive && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={onSwitch}
                  disabled={isLoading}
                >
                  <ArrowRightLeft className="size-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t("actions.switch")}</TooltipContent>
            </Tooltip>
          )}

          {/* 编辑按钮 */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={onEdit}
                disabled={isLoading}
              >
                <Pencil className="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t("actions.edit")}</TooltipContent>
          </Tooltip>

          {/* 复制按钮 */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={onCopy}
                disabled={isLoading}
              >
                <Copy className="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t("actions.copy")}</TooltipContent>
          </Tooltip>

          {/* 测试按钮 */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={onTest}
                disabled={isLoading}
              >
                <Play className="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t("actions.test")}</TooltipContent>
          </Tooltip>

          {/* 删除按钮（hover 时变红） */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={onDelete}
                disabled={isLoading}
                className="hover:text-destructive"
              >
                <Trash2 className="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t("actions.delete")}</TooltipContent>
          </Tooltip>

          {/* 复制到（保留在三点菜单中，仅有其他 CLI 时显示） */}
          {otherClis.length > 0 && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon-sm" disabled={isLoading}>
                  <MoreVertical className="size-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
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
              </DropdownMenuContent>
            </DropdownMenu>
          )}
        </div>
      </TooltipProvider>
    </div>
  );
}
