export interface CliPaths {
  claude_config_dir?: string | null;
  codex_config_dir?: string | null;
}

export interface TestConfig {
  timeout_secs: number;
  test_model?: string | null;
}

export interface LocalSettings {
  active_providers: Record<string, string | null>;
  icloud_dir_override?: string | null;
  cli_paths: CliPaths;
  schema_version: number;
  language?: string | null;
  test_config?: TestConfig | null;
}

export interface TestResult {
  success: boolean;
  elapsed_ms: number;
  error?: string | null;
}

export interface ProxySettings {
  global_enabled: boolean;
  cli_enabled: Record<string, boolean>;
}

export interface CliProxyStatus {
  cli_id: string;
  enabled: boolean;
  active: boolean;
  has_provider: boolean;
  port: number | null;
}

export interface ProxyModeStatus {
  global_enabled: boolean;
  cli_statuses: CliProxyStatus[];
}
