use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 规范化 base_url 为 origin 形式：scheme + host + optional port。
pub fn normalize_origin_base_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Provider base URL cannot be empty".to_string());
    }

    let url = reqwest::Url::parse(trimmed)
        .map_err(|_| "Provider base URL must be a valid absolute URL".to_string())?;

    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err("Provider base URL must start with http:// or https://".to_string());
        }
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err("Provider base URL must not include username or password".to_string());
    }

    if url.host_str().is_none() {
        return Err("Provider base URL must include a host".to_string());
    }

    if url.path() != "/" && !url.path().is_empty() {
        return Err("Provider base URL must not contain a path".to_string());
    }

    if url.query().is_some() {
        return Err("Provider base URL must not contain a query string".to_string());
    }

    if url.fragment().is_some() {
        return Err("Provider base URL must not contain a fragment".to_string());
    }

    Ok(url.as_str().trim_end_matches('/').to_string())
}

/// 从任意绝对 URL 中提取 origin，兼容旧版本遗留的 path/query/fragment。
///
/// 用于代理上游这类只需要 origin 的场景，不用于用户输入校验。
pub fn extract_origin_base_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Provider base URL cannot be empty".to_string());
    }

    let mut url = reqwest::Url::parse(trimmed)
        .map_err(|_| "Provider base URL must be a valid absolute URL".to_string())?;

    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err("Provider base URL must start with http:// or https://".to_string());
        }
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err("Provider base URL must not include username or password".to_string());
    }

    if url.host_str().is_none() {
        return Err("Provider base URL must include a host".to_string());
    }

    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);

    Ok(url.as_str().trim_end_matches('/').to_string())
}

/// 规范化 OpenAI 系列 Provider 的 base_url。
///
/// 与 Anthropic 不同，OpenAI 兼容服务常见部署为带路径前缀的形式，
/// 例如 `https://openrouter.ai/api/v1` 或 `https://gateway.example.com/openai/v1`。
pub fn normalize_openai_base_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Provider base URL cannot be empty".to_string());
    }

    let url = reqwest::Url::parse(trimmed)
        .map_err(|_| "Provider base URL must be a valid absolute URL".to_string())?;

    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err("Provider base URL must start with http:// or https://".to_string());
        }
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err("Provider base URL must not include username or password".to_string());
    }

    if url.host_str().is_none() {
        return Err("Provider base URL must include a host".to_string());
    }

    if url.query().is_some() {
        return Err("Provider base URL must not contain a query string".to_string());
    }

    if url.fragment().is_some() {
        return Err("Provider base URL must not contain a fragment".to_string());
    }

    Ok(url.as_str().trim_end_matches('/').to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    Anthropic,
    #[serde(alias = "open_ai_compatible")]
    OpenAiChatCompletions,
    OpenAiResponses,
}

/// 按协议规范化 Provider base_url。
pub fn normalize_base_url_for_protocol(
    input: &str,
    protocol_type: &ProtocolType,
) -> Result<String, String> {
    match protocol_type {
        ProtocolType::Anthropic => normalize_origin_base_url(input),
        ProtocolType::OpenAiChatCompletions | ProtocolType::OpenAiResponses => {
            normalize_openai_base_url(input)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haiku_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sonnet_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opus_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Provider {
    pub id: String,
    #[serde(default = "default_cli_id")]
    pub cli_id: String,
    pub name: String,
    pub protocol_type: ProtocolType,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub model_config: Option<ModelConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_model_map: Option<HashMap<String, String>>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_cli_id() -> String {
    "claude".to_string()
}

fn default_schema_version() -> u32 {
    1
}

pub fn suggested_test_model(protocol_type: &ProtocolType) -> &'static str {
    match protocol_type {
        ProtocolType::Anthropic => "claude-sonnet-4-6",
        ProtocolType::OpenAiChatCompletions | ProtocolType::OpenAiResponses => "gpt-5.2",
    }
}

pub fn suggested_upstream_model(protocol_type: &ProtocolType) -> Option<&'static str> {
    match protocol_type {
        ProtocolType::Anthropic => None,
        ProtocolType::OpenAiChatCompletions | ProtocolType::OpenAiResponses => Some("gpt-5.2"),
    }
}

impl Provider {
    pub fn new(
        name: String,
        protocol_type: ProtocolType,
        api_key: String,
        base_url: String,
        model: String,
        cli_id: String,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            cli_id,
            name,
            protocol_type,
            api_key,
            base_url,
            model,
            model_config: None,
            notes: None,
            test_model: None,
            upstream_model: None,
            upstream_model_map: None,
            created_at: now,
            updated_at: now,
            schema_version: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_provider() -> Provider {
        Provider {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            cli_id: "claude".to_string(),
            name: "My Anthropic Direct".to_string(),
            protocol_type: ProtocolType::Anthropic,
            api_key: "sk-ant-test123".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            model_config: Some(ModelConfig {
                haiku_model: Some("claude-haiku-4-20250514".to_string()),
                sonnet_model: Some("claude-sonnet-4-20250514".to_string()),
                opus_model: Some("claude-opus-4-20250514".to_string()),
                reasoning_effort: None,
            }),
            notes: Some("Test provider".to_string()),
            test_model: None,
            upstream_model: None,
            upstream_model_map: None,
            created_at: 1710000000000,
            updated_at: 1710000000000,
            schema_version: 1,
        }
    }

    #[test]
    fn test_provider_round_trip() {
        let provider = sample_provider();
        let json = serde_json::to_string_pretty(&provider).unwrap();
        let deserialized: Provider = serde_json::from_str(&json).unwrap();
        assert_eq!(provider, deserialized);
    }

    #[test]
    fn test_protocol_type_anthropic_serde() {
        let json = serde_json::to_string(&ProtocolType::Anthropic).unwrap();
        assert_eq!(json, "\"anthropic\"");
        let deserialized: ProtocolType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ProtocolType::Anthropic);
    }

    #[test]
    fn test_protocol_type_openai_chat_completions_serde() {
        // 新名称序列化为 "open_ai_chat_completions"
        let json = serde_json::to_string(&ProtocolType::OpenAiChatCompletions).unwrap();
        assert_eq!(json, "\"open_ai_chat_completions\"");
        let deserialized: ProtocolType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ProtocolType::OpenAiChatCompletions);
    }

    #[test]
    fn test_protocol_type_openai_compatible_alias_forward_compat() {
        // 旧 JSON 中的 "open_ai_compatible" 通过 alias 反序列化为 OpenAiChatCompletions
        let legacy_json = "\"open_ai_compatible\"";
        let deserialized: ProtocolType = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(deserialized, ProtocolType::OpenAiChatCompletions);
    }

    #[test]
    fn test_protocol_type_openai_responses_serde() {
        let json = serde_json::to_string(&ProtocolType::OpenAiResponses).unwrap();
        assert_eq!(json, "\"open_ai_responses\"");
        let deserialized: ProtocolType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ProtocolType::OpenAiResponses);
    }

    #[test]
    fn test_provider_upstream_model_present() {
        let mut provider = sample_provider();
        provider.upstream_model = Some("gpt-4o".to_string());
        let json = serde_json::to_string(&provider).unwrap();
        assert!(json.contains("upstream_model"));
        assert!(json.contains("gpt-4o"));
    }

    #[test]
    fn test_provider_upstream_model_none_skipped() {
        let provider = sample_provider();
        // sample_provider 未设 upstream_model，应为 None，序列化时 skip
        let json = serde_json::to_string(&provider).unwrap();
        assert!(!json.contains("upstream_model"));
    }

    #[test]
    fn test_provider_upstream_model_map_present() {
        let mut provider = sample_provider();
        let mut map = HashMap::new();
        map.insert("claude-3-5-sonnet".to_string(), "gpt-4o".to_string());
        provider.upstream_model_map = Some(map);
        let json = serde_json::to_string(&provider).unwrap();
        assert!(json.contains("upstream_model_map"));
        assert!(json.contains("claude-3-5-sonnet"));
        assert!(json.contains("gpt-4o"));
    }

    #[test]
    fn test_provider_upstream_model_map_none_skipped() {
        let provider = sample_provider();
        let json = serde_json::to_string(&provider).unwrap();
        assert!(!json.contains("upstream_model_map"));
    }

    #[test]
    fn test_provider_test_model_present() {
        let mut provider = sample_provider();
        provider.test_model = Some("claude-sonnet-4-6".to_string());
        let json = serde_json::to_string(&provider).unwrap();
        assert!(json.contains("test_model"));
        assert!(json.contains("claude-sonnet-4-6"));
    }

    #[test]
    fn test_provider_test_model_none_skipped() {
        let provider = sample_provider();
        let json = serde_json::to_string(&provider).unwrap();
        assert!(!json.contains("test_model"));
    }

    #[test]
    fn test_provider_old_json_without_upstream_fields_deserializes() {
        // 旧 JSON（无 test_model/upstream_model/upstream_model_map）反序列化不崩溃
        let json = r#"{
            "id": "test-id",
            "name": "Test",
            "protocol_type": "anthropic",
            "api_key": "sk-test",
            "base_url": "https://api.example.com",
            "model": "test-model",
            "created_at": 1710000000000,
            "updated_at": 1710000000000
        }"#;
        let provider: Provider = serde_json::from_str(json).unwrap();
        assert_eq!(provider.test_model, None);
        assert_eq!(provider.upstream_model, None);
        assert_eq!(provider.upstream_model_map, None);
    }

    #[test]
    fn test_provider_round_trip_with_new_fields() {
        let mut provider = sample_provider();
        let mut map = HashMap::new();
        map.insert("claude-3-5-sonnet".to_string(), "gpt-4o".to_string());
        provider.test_model = Some("gpt-5.2".to_string());
        provider.upstream_model = Some("gpt-4o-mini".to_string());
        provider.upstream_model_map = Some(map);
        let json = serde_json::to_string_pretty(&provider).unwrap();
        let deserialized: Provider = serde_json::from_str(&json).unwrap();
        assert_eq!(provider, deserialized);
    }

    #[test]
    fn test_model_config_optional_fields() {
        let config = ModelConfig {
            haiku_model: Some("haiku".to_string()),
            sonnet_model: None,
            opus_model: None,
            reasoning_effort: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        // Only haiku_model should be present (skip_serializing_if)
        assert!(json.contains("haiku_model"));
        assert!(!json.contains("sonnet_model"));

        // Deserialize with missing fields
        let partial_json = r#"{"haiku_model": "haiku"}"#;
        let deserialized: ModelConfig = serde_json::from_str(partial_json).unwrap();
        assert_eq!(deserialized.haiku_model, Some("haiku".to_string()));
        assert_eq!(deserialized.sonnet_model, None);
    }

    #[test]
    fn test_schema_version_always_present() {
        let provider = sample_provider();
        let json = serde_json::to_string(&provider).unwrap();
        assert!(json.contains("\"schema_version\":1") || json.contains("\"schema_version\": 1"));
    }

    #[test]
    fn test_schema_version_defaults_to_1() {
        // JSON without schema_version should default to 1
        let json = r#"{
            "id": "test-id",
            "name": "Test",
            "protocol_type": "anthropic",
            "api_key": "sk-test",
            "base_url": "https://api.example.com",
            "model": "test-model",
            "created_at": 1710000000000,
            "updated_at": 1710000000000
        }"#;
        let provider: Provider = serde_json::from_str(json).unwrap();
        assert_eq!(provider.schema_version, 1);
    }

    #[test]
    fn test_provider_cli_id_serializes_deserializes() {
        let mut provider = sample_provider();
        provider.cli_id = "codex".to_string();
        let json = serde_json::to_string_pretty(&provider).unwrap();
        assert!(json.contains("\"cli_id\""));
        let deserialized: Provider = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cli_id, "codex");
    }

    #[test]
    fn test_provider_without_cli_id_defaults_to_claude() {
        // JSON without cli_id field should deserialize with default "claude"
        let json = r#"{
            "id": "test-id",
            "name": "Test",
            "protocol_type": "anthropic",
            "api_key": "sk-test",
            "base_url": "https://api.example.com",
            "model": "test-model",
            "created_at": 1710000000000,
            "updated_at": 1710000000000
        }"#;
        let provider: Provider = serde_json::from_str(json).unwrap();
        assert_eq!(provider.cli_id, "claude");
    }

    #[test]
    fn test_provider_new_accepts_cli_id() {
        let provider = Provider::new(
            "Test".to_string(),
            ProtocolType::Anthropic,
            "sk-test".to_string(),
            "https://api.anthropic.com".to_string(),
            "claude-sonnet-4-20250514".to_string(),
            "codex".to_string(),
        );
        assert_eq!(provider.cli_id, "codex");
    }

    #[test]
    fn test_suggested_test_model_matches_protocol() {
        assert_eq!(
            suggested_test_model(&ProtocolType::Anthropic),
            "claude-sonnet-4-6"
        );
        assert_eq!(
            suggested_test_model(&ProtocolType::OpenAiResponses),
            "gpt-5.2"
        );
    }

    #[test]
    fn test_suggested_upstream_model_only_for_openai() {
        assert_eq!(suggested_upstream_model(&ProtocolType::Anthropic), None);
        assert_eq!(
            suggested_upstream_model(&ProtocolType::OpenAiChatCompletions),
            Some("gpt-5.2")
        );
    }

    #[test]
    fn test_normalize_origin_base_url_strips_trailing_slash() {
        let normalized = normalize_origin_base_url("https://api.openai.com/").unwrap();
        assert_eq!(normalized, "https://api.openai.com");
    }

    #[test]
    fn test_normalize_origin_base_url_keeps_port() {
        let normalized = normalize_origin_base_url("http://127.0.0.1:8080/").unwrap();
        assert_eq!(normalized, "http://127.0.0.1:8080");
    }

    #[test]
    fn test_normalize_origin_base_url_rejects_path() {
        let err = normalize_origin_base_url("https://api.openai.com/v1").unwrap_err();
        assert_eq!(err, "Provider base URL must not contain a path");
    }

    #[test]
    fn test_normalize_origin_base_url_rejects_query() {
        let err = normalize_origin_base_url("https://api.openai.com?foo=bar").unwrap_err();
        assert_eq!(err, "Provider base URL must not contain a query string");
    }

    #[test]
    fn test_normalize_openai_base_url_preserves_path_prefix() {
        let normalized =
            normalize_openai_base_url("https://openrouter.ai/api/v1/chat/completions/").unwrap();
        assert_eq!(normalized, "https://openrouter.ai/api/v1/chat/completions");
    }

    #[test]
    fn test_normalize_openai_base_url_rejects_query() {
        let err = normalize_openai_base_url("https://api.openai.com/v1?foo=bar").unwrap_err();
        assert_eq!(err, "Provider base URL must not contain a query string");
    }

    #[test]
    fn test_normalize_base_url_for_protocol_uses_protocol_specific_rules() {
        let anthropic_err = normalize_base_url_for_protocol(
            "https://api.anthropic.com/v1/messages",
            &ProtocolType::Anthropic,
        )
        .unwrap_err();
        assert_eq!(anthropic_err, "Provider base URL must not contain a path");

        let openai = normalize_base_url_for_protocol(
            "https://gateway.example.com/openai/v1",
            &ProtocolType::OpenAiResponses,
        )
        .unwrap();
        assert_eq!(openai, "https://gateway.example.com/openai/v1");
    }

    #[test]
    fn test_extract_origin_base_url_strips_legacy_path_query_and_fragment() {
        let normalized =
            extract_origin_base_url("https://api.openai.com/v1/chat?foo=bar#frag").unwrap();
        assert_eq!(normalized, "https://api.openai.com");
    }
}
