import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { proxyGetModeStatus } from "@/lib/tauri";
import type { ProxyModeStatus, CliProxyStatus } from "@/types/settings";

export function useProxyStatus() {
  const [proxyStatus, setProxyStatus] = useState<ProxyModeStatus | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const status = await proxyGetModeStatus();
      setProxyStatus(status);
    } catch (err) {
      console.error("获取代理状态失败:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  // 初始化时获取状态
  useEffect(() => {
    refresh();
  }, [refresh]);

  // 监听 proxy-mode-changed 和 providers-changed 事件，自动刷新状态
  // providers-changed：激活/删除 Provider 会影响 has_provider 字段
  useEffect(() => {
    const unlistenProxy = listen<void>("proxy-mode-changed", async () => {
      await refresh();
    });
    const unlistenProviders = listen<void>("providers-changed", async () => {
      await refresh();
    });

    return () => {
      unlistenProxy.then((fn) => fn());
      unlistenProviders.then((fn) => fn());
    };
  }, [refresh]);

  const getCliStatus = useCallback(
    (cliId: string): CliProxyStatus | undefined => {
      return proxyStatus?.cli_statuses.find((s) => s.cli_id === cliId);
    },
    [proxyStatus],
  );

  return { proxyStatus, loading, refresh, getCliStatus };
}
