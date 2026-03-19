use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::provider::ProtocolType;
use crate::traffic::log::LogEntry;

/// 上游目标：包含代理转发所需的全部信息
#[derive(Debug, Clone)]
pub struct UpstreamTarget {
    pub api_key: String,
    pub base_url: String,
    pub protocol_type: ProtocolType,
    /// 上游模型名（Provider 配置的固定映射，优先级低于 upstream_model_map）
    pub upstream_model: Option<String>,
    /// 客户端模型名到上游模型名的映射表
    pub upstream_model_map: Option<HashMap<String, String>>,
    /// Provider 名称（用于日志记录）—— Phase 27 新增
    pub provider_name: String,
}

/// 代理共享状态：持有当前上游目标、HTTP 客户端、日志 sender 和 CLI 标识
#[derive(Clone)]
pub struct ProxyState {
    upstream: Arc<RwLock<Option<UpstreamTarget>>>,
    pub http_client: reqwest::Client,
    /// mpsc sender，用于非阻塞地将日志条目发送到后台写入 worker —— Phase 27 新增
    log_tx: Option<tokio::sync::mpsc::Sender<LogEntry>>,
    /// CLI 标识（如 "claude"、"codex"）—— Phase 27 新增
    cli_id: String,
    /// Tauri AppHandle，用于后台 task 中 try_state/emit —— Phase 28 新增
    app_handle: Option<tauri::AppHandle>,
}

impl ProxyState {
    pub fn new(
        client: reqwest::Client,
        cli_id: String,
        log_tx: Option<tokio::sync::mpsc::Sender<LogEntry>>,
        app_handle: Option<tauri::AppHandle>,
    ) -> Self {
        Self {
            upstream: Arc::new(RwLock::new(None)),
            http_client: client,
            log_tx,
            cli_id,
            app_handle,
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

    /// 获取日志 sender 引用
    pub fn log_sender(&self) -> Option<&tokio::sync::mpsc::Sender<LogEntry>> {
        self.log_tx.as_ref()
    }

    /// 获取 CLI 标识
    pub fn cli_id(&self) -> &str {
        &self.cli_id
    }

    /// 获取 AppHandle 引用（Phase 28 后台 task 使用）
    pub fn app_handle(&self) -> Option<&tauri::AppHandle> {
        self.app_handle.as_ref()
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
            upstream_model: None,
            upstream_model_map: None,
            provider_name: "test".to_string(),
        }
    }

    #[tokio::test]
    async fn test_new_state_has_no_upstream() {
        let state = ProxyState::new(reqwest::Client::new(), "claude".to_string(), None, None);
        assert!(state.get_upstream().await.is_none());
    }

    #[tokio::test]
    async fn test_update_upstream() {
        let state = ProxyState::new(reqwest::Client::new(), "claude".to_string(), None, None);
        let target = make_target();

        state.update_upstream(target.clone()).await;
        let upstream = state.get_upstream().await.unwrap();

        assert_eq!(upstream.api_key, "sk-test-key");
        assert_eq!(upstream.base_url, "https://api.anthropic.com");
        assert!(matches!(upstream.protocol_type, ProtocolType::Anthropic));
    }

    #[tokio::test]
    async fn test_clear_upstream() {
        let state = ProxyState::new(reqwest::Client::new(), "claude".to_string(), None, None);
        state.update_upstream(make_target()).await;
        assert!(state.get_upstream().await.is_some());

        state.clear_upstream().await;
        assert!(state.get_upstream().await.is_none());
    }

    #[tokio::test]
    async fn test_update_upstream_replaces_previous() {
        let state = ProxyState::new(reqwest::Client::new(), "claude".to_string(), None, None);
        state.update_upstream(make_target()).await;

        let new_target = UpstreamTarget {
            api_key: "sk-new-key".to_string(),
            base_url: "https://api.openai.com".to_string(),
            protocol_type: ProtocolType::OpenAiChatCompletions,
            upstream_model: None,
            upstream_model_map: None,
            provider_name: "test".to_string(),
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
            protocol_type: ProtocolType::OpenAiChatCompletions,
            upstream_model: None,
            upstream_model_map: None,
            provider_name: "test".to_string(),
        };
        assert_eq!(target.api_key, "key-123");
        assert_eq!(target.base_url, "https://example.com");
        assert!(matches!(
            target.protocol_type,
            ProtocolType::OpenAiChatCompletions
        ));
    }

    #[tokio::test]
    async fn test_log_sender_and_cli_id() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<LogEntry>(10);
        let state = ProxyState::new(reqwest::Client::new(), "claude".to_string(), Some(tx), None);
        assert!(state.log_sender().is_some());
        assert_eq!(state.cli_id(), "claude");
    }
}
