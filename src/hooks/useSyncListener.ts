import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  takeClaudeOverlayStartupNotifications,
  type ClaudeOverlayApplyNotification,
  type ClaudeOverlayApplySource,
} from "@/lib/tauri";

interface ProvidersChangedPayload {
  changed_files: string[];
  repatched: boolean;
}

interface ActiveProviderChangedPayload {
  cli_id: string;
  provider_id: string;
  source: string;
}

export function useSyncListener(
  refreshProviders: () => Promise<void>,
  refreshSettings: () => Promise<void>,
) {
  const { t } = useTranslation();

  useEffect(() => {
    // ============================================================
    // 共享 overlay 通知处理 helper（供实时事件与 startup replay 共用）
    // ============================================================
    const sourceLabel = (source: ClaudeOverlayApplySource): string =>
      t(`claudeOverlayApply.sourceLabel.${source}`);

    const showClaudeOverlayNotification = (
      notification: ClaudeOverlayApplyNotification,
    ) => {
      const src = sourceLabel(notification.source);

      switch (notification.kind) {
        case "success":
          toast.success(t("claudeOverlayApply.success", { source: src }), {
            duration: 3000,
          });
          // apply 成功后刷新 settings（settings.json 已变）
          refreshSettings();
          break;

        case "failed":
          toast.error(
            t("claudeOverlayApply.failed", {
              source: src,
              error: notification.error ?? "unknown error",
            }),
            { duration: 6000 },
          );
          console.error("Claude overlay apply failed:", notification);
          break;

        case "protected_fields_ignored":
          toast.warning(
            t("claudeOverlayApply.protectedFieldsIgnored", {
              source: src,
              paths: (notification.paths ?? []).join(", "),
            }),
            { duration: 5000 },
          );
          break;
      }
    };

    // ============================================================
    // 实时事件监听：save / watcher 触发的通知
    // ============================================================
    const unlistenOverlaySuccess = listen<ClaudeOverlayApplyNotification>(
      "claude-overlay-apply-success",
      (event) => {
        showClaudeOverlayNotification(event.payload);
      },
    );

    const unlistenOverlayFailed = listen<ClaudeOverlayApplyNotification>(
      "claude-overlay-apply-failed",
      (event) => {
        showClaudeOverlayNotification(event.payload);
      },
    );

    const unlistenOverlayProtected = listen<ClaudeOverlayApplyNotification>(
      "claude-overlay-protected-fields-ignored",
      (event) => {
        showClaudeOverlayNotification(event.payload);
      },
    );

    // ============================================================
    // providers / sync 事件监听（现有逻辑）
    // ============================================================
    const unlistenProviders = listen<ProvidersChangedPayload>(
      "providers-changed",
      async (event) => {
        await refreshProviders();
        await refreshSettings();
        const count = event.payload.changed_files.length;
        if (count === 1) {
          toast.info(
            t("sync.providerUpdated", {
              name: event.payload.changed_files[0],
            }),
            { duration: 3000 },
          );
        } else {
          toast.info(t("sync.providersUpdated", { count }), {
            duration: 3000,
          });
        }
        // Show additional toast when CLI config is re-patched after sync
        if (event.payload.repatched) {
          toast.info(t("sync.repatchSuccess"), { duration: 3000 });
        }
      },
    );

    const unlistenActiveProvider = listen<ActiveProviderChangedPayload>(
      "active-provider-changed",
      async (_event) => {
        await refreshProviders();
        await refreshSettings();
      },
    );

    const unlistenRepatchFail = listen<string>(
      "sync-repatch-failed",
      (event) => {
        toast.error(t("sync.repatchFailed"), { duration: 5000 });
        console.error("Sync re-patch failed:", event.payload);
      },
    );

    // ============================================================
    // startup 缓存通知 take/replay：三个实时 listener 注册完成后拉取
    // ============================================================
    // 因为后端 take 具备清空语义，effect 因语言切换重跑时不会重复弹旧 toast。
    takeClaudeOverlayStartupNotifications()
      .then((notifications) => {
        for (const notification of notifications) {
          showClaudeOverlayNotification(notification);
        }
      })
      .catch((err) => {
        console.error("Failed to take startup overlay notifications:", err);
      });

    return () => {
      unlistenProviders.then((fn) => fn());
      unlistenActiveProvider.then((fn) => fn());
      unlistenRepatchFail.then((fn) => fn());
      unlistenOverlaySuccess.then((fn) => fn());
      unlistenOverlayFailed.then((fn) => fn());
      unlistenOverlayProtected.then((fn) => fn());
    };
  }, [refreshProviders, refreshSettings, t]);
}
