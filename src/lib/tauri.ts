import { invoke } from "@tauri-apps/api/core";
import type { Provider, CreateProviderInput, DetectedCliConfig, ProtocolType } from "@/types/provider";
import type { LocalSettings, TestResult, ProxyModeStatus } from "@/types/settings";
import type { TrafficLog, ProviderStat, TimeStat, TimeRange } from "@/types/traffic";

export async function listProviders(cliId?: string): Promise<Provider[]> {
  return invoke("list_providers", { cliId });
}

export async function createProvider(input: CreateProviderInput): Promise<Provider> {
  return invoke("create_provider", { ...input });
}

export async function updateProvider(provider: Provider): Promise<Provider> {
  return invoke("update_provider", { provider });
}

export async function deleteProvider(id: string): Promise<void> {
  return invoke("delete_provider", { id });
}

export async function setActiveProvider(
  cliId: string,
  providerId: string | null,
): Promise<LocalSettings> {
  return invoke("set_active_provider", { cliId, providerId });
}

export async function getLocalSettings(): Promise<LocalSettings> {
  return invoke("get_local_settings");
}

export async function updateLocalSettings(
  settings: LocalSettings,
): Promise<LocalSettings> {
  return invoke("update_local_settings", { settings });
}

export async function testProvider(providerId: string): Promise<TestResult> {
  return invoke("test_provider", { providerId });
}

export async function syncActiveProviders(): Promise<void> {
  return invoke("sync_active_providers");
}

export async function scanCliConfigs(): Promise<DetectedCliConfig[]> {
  return invoke("scan_cli_configs");
}

export async function importProvider(input: {
  name: string;
  protocolType: ProtocolType;
  apiKey: string;
  baseUrl: string;
  cliId: string;
}): Promise<Provider> {
  return invoke("import_provider", { ...input });
}

export async function refreshTrayMenu(): Promise<void> {
  return invoke("refresh_tray_menu");
}

export async function proxyEnable(cliId: string): Promise<void> {
  return invoke("proxy_enable", { cliId });
}

export async function proxyDisable(cliId: string): Promise<void> {
  return invoke("proxy_disable", { cliId });
}

export async function proxySetGlobal(enabled: boolean): Promise<void> {
  return invoke("proxy_set_global", { enabled });
}

export async function proxyGetModeStatus(): Promise<ProxyModeStatus> {
  return invoke("proxy_get_mode_status");
}

// Claude Settings Overlay 类型与 invoke 封装

export type ClaudeSettingsOverlayStorage = {
  location: "icloud" | "local_fallback";
  file_path: string;
  sync_enabled: boolean;
};

export type ClaudeSettingsOverlayState = {
  content: string | null;
  storage: ClaudeSettingsOverlayStorage;
};

export async function getClaudeSettingsOverlay(): Promise<ClaudeSettingsOverlayState> {
  return invoke("get_claude_settings_overlay");
}

export async function setClaudeSettingsOverlay(
  overlayJson: string,
): Promise<{ storage: ClaudeSettingsOverlayStorage }> {
  return invoke("set_claude_settings_overlay", { overlayJson });
}

// ============================================================
// Overlay apply 通知类型（与后端 ClaudeOverlayApplyNotification 对应）
// ============================================================

export type ClaudeOverlayApplyNotificationKind =
  | "success"
  | "failed"
  | "protected_fields_ignored";

export type ClaudeOverlayApplySource = "save" | "startup" | "watcher";

export type ClaudeOverlayApplyNotification = {
  kind: ClaudeOverlayApplyNotificationKind;
  source: ClaudeOverlayApplySource;
  settings_path?: string;
  error?: string;
  paths?: string[];
};

/// 一次性拉取并清空 startup 期间积累的 overlay apply 通知（take 语义）。
/// useSyncListener 挂载完成后调用，保证 startup 结果不因时序丢失。
export async function takeClaudeOverlayStartupNotifications(): Promise<ClaudeOverlayApplyNotification[]> {
  return invoke("take_claude_overlay_startup_notifications");
}

// ============================================================
// Traffic 流量日志
// ============================================================

/// 拉取最近 N 条流量日志（历史数据初始加载）
export async function getRecentLogs(limit?: number): Promise<TrafficLog[]> {
  return invoke("get_recent_logs", { limit });
}

/// 拉取指定时间范围的 Provider 聚合统计
export async function getProviderStats(range: TimeRange): Promise<ProviderStat[]> {
  return invoke("get_provider_stats", { range });
}

/// 拉取指定时间范围的时间趋势数据
export async function getTimeTrend(range: TimeRange): Promise<TimeStat[]> {
  return invoke("get_time_trend", { range });
}
