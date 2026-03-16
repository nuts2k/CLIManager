import { useState, useRef, useCallback } from "react";
import { getVersion } from "@tauri-apps/api/app";

// 更新状态类型
export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error";

export interface UseUpdaterReturn {
  status: UpdateStatus;
  newVersion: string | null;       // 新版本号
  currentVersion: string;          // 当前版本号
  progress: number;                // 下载进度 0-100，-1 表示不确定
  error: string | null;
  checkForUpdate: () => Promise<void>;
  downloadAndInstall: () => Promise<void>;
  dismissUpdate: () => void;       // 稍后提醒
}

export const RESTART_REQUIRED_ERROR = "__restart_required__";

export function useUpdater(): UseUpdaterReturn {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [newVersion, setNewVersion] = useState<string | null>(null);
  const [currentVersion, setCurrentVersion] = useState<string>("0.2.0");
  const [progress, setProgress] = useState<number>(0);
  const [error, setError] = useState<string | null>(null);

  // 存储 Update 对象供 downloadAndInstall 使用
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const updateRef = useRef<any>(null);
  // 标记本次启动是否已被用户关闭弹窗
  const dismissedThisSession = useRef<boolean>(false);

  const checkForUpdate = useCallback(async () => {
    // 如果本次启动已被 dismiss，不重新弹窗（仅限自动检查；手动触发可重置）
    setStatus("checking");
    setError(null);

    try {
      // 获取当前版本
      const version = await getVersion();
      setCurrentVersion(version);
    } catch {
      // 忽略版本获取失败
    }

    try {
      // 动态导入避免开发模式下模块不存在时报错
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();

      if (update) {
        updateRef.current = update;
        setNewVersion(update.version);
        setStatus("available");
      } else {
        setStatus("idle");
      }
    } catch {
      // 更新检查失败静默处理（开发模式 / 无网络 / 无 Release）
      setStatus("idle");
    }
  }, []);

  const downloadAndInstall = useCallback(async () => {
    if (!updateRef.current) {
      return;
    }

    setStatus("downloading");
    setProgress(0);
    setError(null);

    try {
      let contentLength: number | undefined;
      let downloaded = 0;

      await updateRef.current.downloadAndInstall(
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (event: any) => {
          if (event.event === "Started") {
            contentLength = event.data?.contentLength;
            if (!contentLength) {
              // 不确定总大小，用 -1 表示不确定态
              setProgress(-1);
            } else {
              setProgress(0);
            }
          } else if (event.event === "Progress") {
            downloaded += event.data?.chunkLength ?? 0;
            if (contentLength && contentLength > 0) {
              const pct = Math.round((downloaded / contentLength) * 100);
              setProgress(Math.min(pct, 99));
            }
            // 不确定态下保持 -1
          } else if (event.event === "Finished") {
            setProgress(100);
            setStatus("ready");
          }
        },
      );

      // 安装完成后重启
      try {
        const { relaunch, exit } = await import("@tauri-apps/plugin-process");
        await relaunch();
        // 若 relaunch resolve 后进程仍未终止，强制退出（新进程已启动）
        await exit(0);
      } catch {
        // 自动重启失败时恢复为可关闭错误态，提示用户手动重启
        setStatus("error");
        setError(RESTART_REQUIRED_ERROR);
      }
    } catch (err) {
      setStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const dismissUpdate = useCallback(() => {
    dismissedThisSession.current = true;
    setStatus("idle");
  }, []);

  return {
    status,
    newVersion,
    currentVersion,
    progress,
    error,
    checkForUpdate,
    downloadAndInstall,
    dismissUpdate,
  };
}
