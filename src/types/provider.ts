export type ProtocolType =
  | "anthropic"
  | "open_ai_chat_completions"
  | "open_ai_responses";

export interface ModelConfig {
  haiku_model?: string | null;
  sonnet_model?: string | null;
  opus_model?: string | null;
  reasoning_effort?: string | null;
}

export interface Provider {
  id: string;
  name: string;
  cli_id: string;
  protocol_type: ProtocolType;
  api_key: string;
  base_url: string;
  model: string;
  model_config?: ModelConfig | null;
  notes?: string | null;
  test_model?: string | null;
  upstream_model?: string | null;
  upstream_model_map?: Record<string, string> | null;
  created_at: number;
  updated_at: number;
  schema_version: number;
}

export interface CreateProviderInput {
  name: string;
  protocolType: ProtocolType;
  apiKey: string;
  baseUrl: string;
  model: string;
  cliId: string;
  modelConfig?: ModelConfig | null;
  notes?: string | null;
  testModel?: string | null;
  upstreamModel?: string | null;
  upstreamModelMap?: Record<string, string> | null;
}

export interface DetectedCliConfig {
  cli_id: string;
  cli_name: string;
  api_key: string;
  base_url: string;
  protocol_type: ProtocolType;
  has_api_key: boolean;
}
