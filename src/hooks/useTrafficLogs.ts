import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getRecentLogs } from "@/lib/tauri";
import type { TrafficLog } from "@/types/traffic";

/** 内存中最多保留的日志条数，超出时丢弃最旧条目 */
const MAX_LOGS = 500;

/**
 * 双轨数据 hook：
 * - 初始化时通过 Tauri command 拉取最近 100 条历史日志
 * - 监听 traffic-log 事件进行增量追加或更新
 */
export function useTrafficLogs(): { logs: TrafficLog[]; loading: boolean } {
  const [logs, setLogs] = useState<TrafficLog[]>([]);
  const [loading, setLoading] = useState(true);

  // 初始拉取历史日志
  useEffect(() => {
    getRecentLogs(100)
      .then((history) => {
        setLogs(history);
      })
      .catch((err) => {
        console.error("Failed to fetch recent traffic logs:", err);
      })
      .finally(() => {
        setLoading(false);
      });
  }, []);

  // 实时监听增量事件
  useEffect(() => {
    const unlisten = listen<TrafficLog>("traffic-log", (event) => {
      const payload = event.payload;

      if (payload.type === "new") {
        // 置顶插入，超过上限时丢弃最旧条目
        setLogs((prev) => {
          const next = [payload, ...prev];
          return next.length > MAX_LOGS ? next.slice(0, MAX_LOGS) : next;
        });
      } else if (payload.type === "update") {
        // 替换同 id 条目；找不到则静默忽略（避免竞态问题）
        setLogs((prev) => {
          const idx = prev.findIndex((log) => log.id === payload.id);
          if (idx === -1) {
            return prev;
          }
          const next = [...prev];
          next[idx] = payload;
          return next;
        });
      }
      // type === "history" 在初始拉取中已处理，实时事件中忽略
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return { logs, loading };
}
