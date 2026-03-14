use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("服务器已在运行")]
    AlreadyRunning,

    #[error("服务器未运行")]
    NotRunning,

    #[error("地址绑定失败: {0}")]
    BindFailed(String),

    #[error("停止超时")]
    StopTimeout,

    #[error("停止失败: {0}")]
    StopFailed(String),

    #[error("上游不可达: {0}")]
    UpstreamUnreachable(String),

    #[error("未配置上游目标")]
    NoUpstreamConfigured,

    #[error("健康检查失败: {0}")]
    HealthCheckFailed(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ProxyError::UpstreamUnreachable(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            ProxyError::NoUpstreamConfigured => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = json!({
            "error": {
                "type": "proxy_error",
                "message": message,
            }
        });

        (status, Json(body)).into_response()
    }
}

// 与 AppError 保持一致的序列化模式：序列化为字符串
// IntoResponse 是给 axum handler 用的（HTTP 响应），Serialize 是给 Tauri command 用的（前端错误）
impl serde::Serialize for ProxyError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    /// 辅助函数：将 Response 转换为 (StatusCode, JSON body)
    async fn response_parts(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        (status, json)
    }

    #[tokio::test]
    async fn test_upstream_unreachable_returns_502() {
        let err = ProxyError::UpstreamUnreachable("连接超时".to_string());
        let resp = err.into_response();
        let (status, json) = response_parts(resp).await;

        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(json["error"]["type"], "proxy_error");
        assert!(json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("上游不可达"));
    }

    #[tokio::test]
    async fn test_no_upstream_configured_returns_503() {
        let err = ProxyError::NoUpstreamConfigured;
        let resp = err.into_response();
        let (status, json) = response_parts(resp).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["error"]["type"], "proxy_error");
    }

    #[tokio::test]
    async fn test_bind_failed_returns_500() {
        let err = ProxyError::BindFailed("端口被占用".to_string());
        let resp = err.into_response();
        let (status, json) = response_parts(resp).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["error"]["type"], "proxy_error");
    }

    #[tokio::test]
    async fn test_error_json_format() {
        let err = ProxyError::Internal("测试错误".to_string());
        let resp = err.into_response();
        let (_, json) = response_parts(resp).await;

        // 验证 JSON 结构：{"error": {"type": "proxy_error", "message": "..."}}
        assert!(json.get("error").is_some());
        assert_eq!(json["error"]["type"], "proxy_error");
        assert!(json["error"]["message"].as_str().is_some());
    }

    #[test]
    fn test_serialize_as_string() {
        let err = ProxyError::UpstreamUnreachable("测试".to_string());
        let json_str = serde_json::to_string(&err).unwrap();
        // Serialize 输出应该是字符串格式
        assert!(json_str.contains("上游不可达"));
    }
}
