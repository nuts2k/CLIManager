use std::time::Duration;

use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use super::error::ProxyError;
use super::handler::{health_handler, proxy_handler};
use super::state::ProxyState;

/// 构建代理路由
fn build_router(state: ProxyState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .fallback(proxy_handler)
        .with_state(state)
}

/// 单端口代理服务器
///
/// 管理 axum 服务器的启停生命周期，提供优雅停机和健康自检。
pub struct ProxyServer {
    shutdown_tx: Option<oneshot::Sender<()>>,
    server_handle: Option<JoinHandle<()>>,
    state: ProxyState,
    port: u16,
}

impl ProxyServer {
    /// 创建新的代理服务器实例
    pub fn new(port: u16, client: reqwest::Client) -> Self {
        Self {
            shutdown_tx: None,
            server_handle: None,
            state: ProxyState::new(client),
            port,
        }
    }

    /// 获取代理共享状态引用（用于 update_upstream）
    pub fn state(&self) -> &ProxyState {
        &self.state
    }

    /// 启动代理服务器
    ///
    /// 绑定 127.0.0.1:{port}，启动 axum 服务，执行健康自检。
    /// 失败时自动清理资源。
    pub async fn start(&mut self) -> Result<(), ProxyError> {
        if self.shutdown_tx.is_some() {
            return Err(ProxyError::AlreadyRunning);
        }

        let (tx, rx) = oneshot::channel::<()>();
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| ProxyError::BindFailed(format!("{}: {}", addr, e)))?;

        // 获取实际绑定的端口（支持动态端口分配）
        let actual_port = listener.local_addr().map(|a| a.port()).unwrap_or(self.port);
        self.port = actual_port;

        let app = build_router(self.state.clone());

        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });

        self.shutdown_tx = Some(tx);
        self.server_handle = Some(handle);

        // 启动后健康自检
        if let Err(e) = health_check(self.port).await {
            // 健康检查失败，自动停止并返回错误
            let _ = self.stop().await;
            return Err(e);
        }

        Ok(())
    }

    /// 停止代理服务器（优雅停机，5 秒超时）
    pub async fn stop(&mut self) -> Result<(), ProxyError> {
        let tx = self
            .shutdown_tx
            .take()
            .ok_or(ProxyError::NotRunning)?;

        // 发送停机信号
        let _ = tx.send(());

        if let Some(handle) = self.server_handle.take() {
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(ProxyError::StopFailed(e.to_string())),
                Err(_) => Err(ProxyError::StopTimeout),
            }
        } else {
            Ok(())
        }
    }

    /// 服务器是否正在运行
    pub fn is_running(&self) -> bool {
        self.shutdown_tx.is_some()
    }

    /// 获取当前监听端口
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// 健康自检：确认代理端口可达
async fn health_check(port: u16) -> Result<(), ProxyError> {
    let url = format!("http://127.0.0.1:{}/health", port);
    // 使用 no_proxy 避免系统代理拦截本地健康检查
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| ProxyError::Internal(format!("创建 HTTP 客户端失败: {}", e)))?;

    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|e| ProxyError::HealthCheckFailed(format!("无法连接: {}", e)))?;

    if resp.status() != 200 {
        return Err(ProxyError::HealthCheckFailed(format!(
            "状态码: {}",
            resp.status()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProtocolType;
    use crate::proxy::state::UpstreamTarget;
    use axum::body::Bytes;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use axum::Json;
    use serde_json::{json, Value};
    use std::time::Duration;

    /// 辅助函数：获取可用动态端口
    async fn get_free_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    /// 辅助函数：创建不走系统代理的 HTTP 客户端（避免 Surge 等代理软件拦截）
    fn test_client() -> reqwest::Client {
        reqwest::Client::builder().no_proxy().build().unwrap()
    }

    /// 辅助函数：启动 mock 上游服务器
    /// 返回 (端口, 停机信号发送端)
    async fn start_mock_upstream(
        handler: Router,
    ) -> (u16, oneshot::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            axum::serve(listener, handler)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });

        // 等待服务器就绪
        tokio::time::sleep(Duration::from_millis(50)).await;

        (port, tx)
    }

    #[tokio::test]
    async fn test_server_start_stop() {
        let port = get_free_port().await;
        let mut server = ProxyServer::new(port, test_client());

        // 启动
        server.start().await.unwrap();
        assert!(server.is_running());

        // 健康检查应该能通过
        let resp = test_client()
            .get(format!("http://127.0.0.1:{}/health", port))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        // 停止
        server.stop().await.unwrap();
        assert!(!server.is_running());

        // 停止后端口应该释放（等一小段时间让系统回收端口）
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_proxy_forward() {
        // 创建 mock 上游服务器
        let mock_app = Router::new().route(
            "/v1/messages",
            get(|| async {
                Json(json!({"id": "msg_123", "content": "Hello from upstream"}))
            }),
        );
        let (upstream_port, upstream_shutdown) = start_mock_upstream(mock_app).await;

        // 启动代理
        let proxy_port = get_free_port().await;
        let mut server = ProxyServer::new(proxy_port, test_client());
        server
            .state()
            .update_upstream(UpstreamTarget {
                api_key: "sk-test".to_string(),
                base_url: format!("http://127.0.0.1:{}", upstream_port),
                protocol_type: ProtocolType::Anthropic,
            })
            .await;
        server.start().await.unwrap();

        // 通过代理发请求
        let resp = test_client()
            .get(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["id"], "msg_123");
        assert_eq!(body["content"], "Hello from upstream");

        // 清理
        server.stop().await.unwrap();
        let _ = upstream_shutdown.send(());
    }

    #[tokio::test]
    async fn test_credential_replacement_e2e() {
        // 创建 mock 上游，检查收到的 headers
        let mock_app = Router::new().fallback(
            |req: Request| async move {
                // 检查是否收到了真实 API key（而非 PROXY_MANAGED）
                let api_key = req
                    .headers()
                    .get("x-api-key")
                    .map(|v| v.to_str().unwrap_or("").to_string())
                    .unwrap_or_default();

                Json(json!({
                    "received_api_key": api_key,
                }))
            },
        );
        let (upstream_port, upstream_shutdown) = start_mock_upstream(mock_app).await;

        // 启动代理
        let proxy_port = get_free_port().await;
        let mut server = ProxyServer::new(proxy_port, test_client());
        server
            .state()
            .update_upstream(UpstreamTarget {
                api_key: "sk-real-secret-key".to_string(),
                base_url: format!("http://127.0.0.1:{}", upstream_port),
                protocol_type: ProtocolType::Anthropic,
            })
            .await;
        server.start().await.unwrap();

        // 发送带占位凭据的请求
        let resp = test_client()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .body(r#"{"test": true}"#)
            .send()
            .await
            .unwrap();

        let body: Value = resp.json().await.unwrap();
        // 上游应该收到真实 key，而非 PROXY_MANAGED
        assert_eq!(body["received_api_key"], "sk-real-secret-key");

        // 清理
        server.stop().await.unwrap();
        let _ = upstream_shutdown.send(());
    }

    #[tokio::test]
    async fn test_sse_streaming() {
        use std::convert::Infallible;

        // 创建 mock 上游返回 SSE 流式响应
        let mock_app = Router::new().fallback(|| async {
            let chunks = vec![
                "data: {\"type\":\"content_block_start\"}\n\n",
                "data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello\"}}\n\n",
                "data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\" World\"}}\n\n",
                "data: [DONE]\n\n",
            ];

            let stream = futures::stream::iter(chunks.into_iter().map(|chunk| {
                Ok::<Bytes, Infallible>(Bytes::from(chunk))
            }));

            let body = axum::body::Body::from_stream(stream);
            axum::response::Response::builder()
                .status(200)
                .header("content-type", "text/event-stream")
                .header("cache-control", "no-cache")
                .body(body)
                .unwrap()
        });
        let (upstream_port, upstream_shutdown) = start_mock_upstream(mock_app).await;

        // 启动代理
        let proxy_port = get_free_port().await;
        let mut server = ProxyServer::new(proxy_port, test_client());
        server
            .state()
            .update_upstream(UpstreamTarget {
                api_key: "sk-test".to_string(),
                base_url: format!("http://127.0.0.1:{}", upstream_port),
                protocol_type: ProtocolType::Anthropic,
            })
            .await;
        server.start().await.unwrap();

        // 通过代理请求 SSE 流式响应
        let resp = test_client()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        // 验证 Content-Type 被透传
        let content_type = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(content_type.contains("text/event-stream"));

        // 读取完整响应体验证内容
        let body = resp.text().await.unwrap();
        assert!(body.contains("content_block_start"));
        assert!(body.contains("Hello"));
        assert!(body.contains(" World"));
        assert!(body.contains("[DONE]"));

        // 清理
        server.stop().await.unwrap();
        let _ = upstream_shutdown.send(());
    }

    #[tokio::test]
    async fn test_upstream_unreachable() {
        // 获取一个空闲端口作为 "不存在的上游"（获取后立即释放）
        let dead_port = get_free_port().await;

        // 启动代理，指向一个不存在的上游地址
        let proxy_port = get_free_port().await;
        let mut server = ProxyServer::new(proxy_port, test_client());
        server
            .state()
            .update_upstream(UpstreamTarget {
                api_key: "sk-test".to_string(),
                base_url: format!("http://127.0.0.1:{}", dead_port),
                protocol_type: ProtocolType::Anthropic,
            })
            .await;
        server.start().await.unwrap();

        // 发送请求——应该得到 502（代理内部用 no_proxy 客户端，确保直连到不可达地址）
        let resp = test_client()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .body(r#"{"test": true}"#)
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["error"]["type"], "proxy_error");

        // 清理
        server.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_bind_failed() {
        // 先占用一个端口
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // 尝试绑定已占用端口
        let mut server = ProxyServer::new(port, test_client());
        let result = server.start().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ProxyError::BindFailed(msg) => {
                assert!(msg.contains(&port.to_string()));
            }
            other => panic!("期望 BindFailed，得到 {:?}", other),
        }

        drop(listener);
    }

    #[tokio::test]
    async fn test_double_start() {
        let port = get_free_port().await;
        let mut server = ProxyServer::new(port, test_client());

        // 第一次启动成功
        server.start().await.unwrap();
        assert!(server.is_running());

        // 第二次启动应该返回 AlreadyRunning
        let result = server.start().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProxyError::AlreadyRunning => {}
            other => panic!("期望 AlreadyRunning，得到 {:?}", other),
        }

        // 清理
        server.stop().await.unwrap();
    }
}
