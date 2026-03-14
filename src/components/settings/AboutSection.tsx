import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import type { UpdateStatus } from "@/components/updater/useUpdater";

interface AboutSectionProps {
  onCheckUpdate: () => void;
  updateStatus: UpdateStatus;
  currentVersion: string;
  newVersion: string | null;
  onUpdate?: () => void;  // 点击"更新到 vX.X.X"触发下载安装
}

export function AboutSection({
  onCheckUpdate,
  updateStatus,
  currentVersion,
  newVersion,
  onUpdate,
}: AboutSectionProps) {
  const { t } = useTranslation();

  // 打开关于区域时自动触发检查更新
  useEffect(() => {
    onCheckUpdate();
    // 仅在组件挂载时触发一次
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleViewReleases = async () => {
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl("https://github.com/nuts2k/CLIManager/releases");
    } catch {
      // 回退：用浏览器默认方式打开
      window.open("https://github.com/nuts2k/CLIManager/releases", "_blank");
    }
  };

  return (
    <div className="space-y-3">
      <div className="space-y-1 text-sm text-muted-foreground">
        <p>
          {t("settings.version")}: {currentVersion}
        </p>
        <p>CLIManager - CLI Provider Management Tool</p>
      </div>

      {/* 更新状态区域 */}
      <div className="flex flex-wrap items-center gap-2">
        {/* 正在检查 */}
        {updateStatus === "checking" && (
          <span className="text-sm text-muted-foreground">
            {t("updater.checking")}
          </span>
        )}

        {/* 有新版本：显示更新按钮 */}
        {updateStatus === "available" && newVersion && (
          <Button size="sm" onClick={onUpdate}>
            {t("updater.updateAvailable", { version: newVersion })}
          </Button>
        )}

        {/* 已是最新（idle 且非初始状态，即检查过后结果为无更新） */}
        {updateStatus === "idle" && (
          <span className="text-sm text-muted-foreground">
            {t("updater.upToDate")}
          </span>
        )}

        {/* 手动触发检查更新按钮 */}
        {(updateStatus === "idle" || updateStatus === "error") && (
          <Button
            size="sm"
            variant="outline"
            onClick={onCheckUpdate}
            disabled={updateStatus === "checking"}
          >
            {t("updater.checkUpdate")}
          </Button>
        )}
      </div>

      {/* GitHub Releases 链接 */}
      <div>
        <Button variant="link" size="sm" className="px-0" onClick={handleViewReleases}>
          {t("updater.viewReleases")}
        </Button>
      </div>
    </div>
  );
}
