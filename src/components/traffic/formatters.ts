/**
 * 流量监控页面格式化工具函数（纯函数，无 React 依赖）
 */

/** formatTime 返回值类型，交由组件层通过 i18n 拼接 */
export type FormattedTime =
  | { type: "seconds"; count: number }
  | { type: "minutes"; count: number }
  | { type: "absolute"; value: string };

/**
 * 将 token 数量格式化为易读字符串。
 * - null → "--"
 * - >= 1_000_000 → "X.XM"
 * - >= 1_000 → "X.Xk"
 * - 否则 → 原始数字字符串
 */
export function formatTokenCount(n: number | null): string {
  if (n === null) return "--";
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "k";
  return String(n);
}

/**
 * 将 epoch ms 时间戳解析为结构化相对/绝对时间对象。
 * - diff < 60s → { type: "seconds", count: N }
 * - diff < 3600s → { type: "minutes", count: N }
 * - 否则 → { type: "absolute", value: "HH:MM:SS" }
 *
 * 组件层使用 t() 函数将 type/count 拼接为本地化字符串。
 */
export function formatTime(epochMs: number): FormattedTime {
  const diffMs = Date.now() - epochMs;
  const diffSec = Math.floor(diffMs / 1000);

  if (diffSec < 60) {
    return { type: "seconds", count: Math.max(diffSec, 0) };
  }
  if (diffSec < 3600) {
    return { type: "minutes", count: Math.floor(diffSec / 60) };
  }

  // 超过 1 小时显示具体时间
  const d = new Date(epochMs);
  const pad = (n: number) => String(n).padStart(2, "0");
  const value = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  return { type: "absolute", value };
}

/**
 * 将毫秒时长格式化为易读字符串。
 * - null → "--"
 * - >= 1000ms → "X.Xs"
 * - 否则 → "Xms"
 */
export function formatDuration(ms: number | null): string {
  if (ms === null) return "--";
  if (ms >= 1000) return (ms / 1000).toFixed(1) + "s";
  return Math.round(ms) + "ms";
}

/**
 * 计算 tokens per second（TPS）。
 * - outputTokens 或 durationMs 为 null，或 durationMs === 0 → "--"
 * - 否则返回保留一位小数的数值字符串（不含 "t/s" 后缀，后缀在组件层通过 i18n 拼接）
 */
export function calcTps(
  outputTokens: number | null,
  durationMs: number | null
): string {
  if (outputTokens === null || durationMs === null || durationMs === 0)
    return "--";
  return (outputTokens / (durationMs / 1000)).toFixed(1);
}

/**
 * 根据 HTTP 状态码返回对应的 Tailwind 颜色 class。
 * - null → "text-muted-foreground"
 * - 2xx → "text-status-success"
 * - 4xx → "text-status-warning"
 * - 5xx → "text-destructive"
 * - 其他 → "text-foreground"
 */
export function statusCodeClass(code: number | null): string {
  if (code === null) return "text-muted-foreground";
  if (code >= 200 && code < 300) return "text-status-success";
  if (code >= 400 && code < 500) return "text-status-warning";
  if (code >= 500 && code < 600) return "text-destructive";
  return "text-foreground";
}
