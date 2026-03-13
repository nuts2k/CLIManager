use std::sync::Arc;
use tokio::sync::RwLock;

use crate::provider::ProtocolType;

/// 上游目标：包含代理转发所需的全部信息
#[derive(Debug, Clone)]
pub struct UpstreamTarget {
    pub api_key: String,
    pub base_url: String,
    pub protocol_type: ProtocolType,
}

/// 代理共享状态：持有当前上游目标和 HTTP 客户端
#[derive(Clone)]
pub struct ProxyState {
    upstream: Arc<RwLock<Option<UpstreamTarget>>>,
    pub http_client: reqwest::Client,
}

impl ProxyState {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            upstream: Arc::new(RwLock::new(None)),
            http_client: client,
        }
    }

    /// 获取当前上游目标（clone 返回）
    pub async fn get_upstream(&self) -> Option<UpstreamTarget> {
        self.upstream.read().await.clone()
    }

    /// 更新上游目标（Provider 切换时调用）
    pub async fn update_upstream(&self, target: UpstreamTarget) {
        *self.upstream.write().await = Some(target);
    }

    /// 清除上游目标
    pub async fn clear_upstream(&self) {
        *self.upstream.write().await = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_target() -> UpstreamTarget {
        UpstreamTarget {
            api_key: "sk-test-key".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            protocol_type: ProtocolType::Anthropic,
        }
    }

    #[tokio::test]
    async fn test_new_state_has_no_upstream() {
        let state = ProxyState::new(reqwest::Client::new());
        assert!(state.get_upstream().await.is_none());
    }

    #[tokio::test]
    async fn test_update_upstream() {
        let state = ProxyState::new(reqwest::Client::new());
        let target = make_target();

        state.update_upstream(target.clone()).await;
        let upstream = state.get_upstream().await.unwrap();

        assert_eq!(upstream.api_key, "sk-test-key");
        assert_eq!(upstream.base_url, "https://api.anthropic.com");
        assert!(matches!(upstream.protocol_type, ProtocolType::Anthropic));
    }

    #[tokio::test]
    async fn test_clear_upstream() {
        let state = ProxyState::new(reqwest::Client::new());
        state.update_upstream(make_target()).await;
        assert!(state.get_upstream().await.is_some());

        state.clear_upstream().await;
        assert!(state.get_upstream().await.is_none());
    }

    #[tokio::test]
    async fn test_update_upstream_replaces_previous() {
        let state = ProxyState::new(reqwest::Client::new());
        state.update_upstream(make_target()).await;

        let new_target = UpstreamTarget {
            api_key: "sk-new-key".to_string(),
            base_url: "https://api.openai.com".to_string(),
            protocol_type: ProtocolType::OpenAiCompatible,
        };
        state.update_upstream(new_target).await;

        let upstream = state.get_upstream().await.unwrap();
        assert_eq!(upstream.api_key, "sk-new-key");
        assert_eq!(upstream.base_url, "https://api.openai.com");
    }

    #[tokio::test]
    async fn test_upstream_target_fields() {
        let target = UpstreamTarget {
            api_key: "key-123".to_string(),
            base_url: "https://example.com".to_string(),
            protocol_type: ProtocolType::OpenAiCompatible,
        };
        assert_eq!(target.api_key, "key-123");
        assert_eq!(target.base_url, "https://example.com");
        assert!(matches!(target.protocol_type, ProtocolType::OpenAiCompatible));
    }
}
