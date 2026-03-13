import { invoke } from "@tauri-apps/api/core";
import type { Provider, CreateProviderInput, DetectedCliConfig, ProtocolType } from "@/types/provider";
import type { LocalSettings, TestResult } from "@/types/settings";

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
