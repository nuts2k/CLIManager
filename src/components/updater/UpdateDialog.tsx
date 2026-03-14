import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { RESTART_REQUIRED_ERROR, type UpdateStatus } from "./useUpdater";

interface UpdateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  status: UpdateStatus;
  currentVersion: string;
  newVersion: string | null;
  progress: number;
  error: string | null;
  onUpdate: () => void;       // 开始下载安装
  onRemindLater: () => void;  // 稍后提醒
}

export function UpdateDialog({
  open,
  onOpenChange,
  status,
  currentVersion,
  newVersion,
  progress,
  error,
  onUpdate,
  onRemindLater,
}: UpdateDialogProps) {
  const { t } = useTranslation();
  const isRestartRequired = error === RESTART_REQUIRED_ERROR;

  // 下载/安装中不允许关闭
  const isLocked = status === "downloading" || status === "ready";

  const handleOpenChange = (val: boolean) => {
    if (isLocked) return;
    onOpenChange(val);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent showCloseButton={!isLocked}>
        {/* 状态：发现新版本 */}
        {status === "available" && (
          <>
            <DialogHeader>
              <DialogTitle>{t("updater.title")}</DialogTitle>
              <DialogDescription>
                {t("updater.description", {
                  current: currentVersion,
                  latest: newVersion ?? "",
                })}
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={onRemindLater}>
                {t("updater.remindLater")}
              </Button>
              <Button onClick={onUpdate}>
                {t("updater.updateNow")}
              </Button>
            </DialogFooter>
          </>
        )}

        {/* 状态：下载中 */}
        {status === "downloading" && (
          <>
            <DialogHeader>
              <DialogTitle>{t("updater.downloading")}</DialogTitle>
            </DialogHeader>
            <div className="py-2">
              {progress === -1 ? (
                /* 不确定态：脉冲动画 */
                <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
                  <div className="h-full w-1/3 animate-pulse rounded-full bg-primary" />
                </div>
              ) : (
                /* 确定态：百分比进度条 */
                <div className="space-y-1">
                  <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
                    <div
                      className="h-full rounded-full bg-primary transition-all duration-300"
                      style={{ width: `${progress}%` }}
                    />
                  </div>
                  <p className="text-right text-xs text-muted-foreground">
                    {progress}%
                  </p>
                </div>
              )}
            </div>
          </>
        )}

        {/* 状态：安装完成 */}
        {status === "ready" && (
          <DialogHeader>
            <DialogTitle>{t("updater.installing")}</DialogTitle>
          </DialogHeader>
        )}

        {/* 状态：更新失败 */}
        {status === "error" && (
          <>
            <DialogHeader>
              <DialogTitle>
                {isRestartRequired
                  ? t("updater.restartRequiredTitle")
                  : t("updater.error")}
              </DialogTitle>
              {isRestartRequired && (
                <DialogDescription>
                  {t("updater.restartRequiredDescription")}
                </DialogDescription>
              )}
              {!isRestartRequired && error && (
                <DialogDescription>{error}</DialogDescription>
              )}
            </DialogHeader>
            <DialogFooter showCloseButton>
              {/* showCloseButton 会渲染一个 Close 按钮 */}
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
