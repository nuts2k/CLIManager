import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

interface ProvidersChangedPayload {
  changed_files: string[];
  repatched: boolean;
}

export function useSyncListener(
  refreshProviders: () => Promise<void>,
  refreshSettings: () => Promise<void>,
) {
  const { t } = useTranslation();

  useEffect(() => {
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

    const unlistenRepatchFail = listen<string>(
      "sync-repatch-failed",
      (event) => {
        toast.error(t("sync.repatchFailed"), { duration: 5000 });
        console.error("Sync re-patch failed:", event.payload);
      },
    );

    return () => {
      unlistenProviders.then((fn) => fn());
      unlistenRepatchFail.then((fn) => fn());
    };
  }, [refreshProviders, refreshSettings, t]);
}
