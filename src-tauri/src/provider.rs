use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    Anthropic,
    OpenAiCompatible,
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

impl Provider {
    pub fn new(name: String, protocol_type: ProtocolType, api_key: String, base_url: String, model: String, cli_id: String) -> Self {
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
    fn test_protocol_type_openai_compatible_serde() {
        let json = serde_json::to_string(&ProtocolType::OpenAiCompatible).unwrap();
        assert_eq!(json, "\"open_ai_compatible\"");
        let deserialized: ProtocolType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ProtocolType::OpenAiCompatible);
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
}
