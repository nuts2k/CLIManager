use std::collections::HashMap;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::Mutex;

pub mod error;
pub mod handler;
pub mod server;
pub mod state;

pub use error::ProxyError;
pub use handler::{health_handler, proxy_handler};
pub use server::ProxyServer;
pub use state::{ProxyState, UpstreamTarget};

/// Claude Code 代理固定端口
pub const PROXY_PORT_CLAUDE: u16 = 15800;
/// Codex 代理固定端口
pub const PROXY_PORT_CODEX: u16 = 15801;

/// 根据 cli_id 获取对应的代理端口
pub fn proxy_port_for_cli(cli_id: &str) -> Option<u16> {
    match cli_id {
        "claude" => Some(PROXY_PORT_CLAUDE),
        "codex" => Some(PROXY_PORT_CODEX),
        _ => None,
    }
}

/// 代理服务状态信息（单个服务器）
#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub cli_id: String,
    pub port: u16,
    pub running: bool,
}

/// 代理服务总状态信息
#[derive(Debug, Clone, Serialize)]
pub struct ProxyStatusInfo {
    pub servers: Vec<ServerStatus>,
}

/// 多端口代理服务管理器
///
/// 管理多个 CLI 对应的 ProxyServer 实例，按 cli_id 独立启停。
/// 作为 Tauri 托管状态使用。
pub struct ProxyService {
    servers: Mutex<HashMap<String, ProxyServer>>,
    http_client: reqwest::Client,
}

impl ProxyService {
    /// 创建新的代理服务管理器
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .tcp_keepalive(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .no_proxy()
            .build()
            .expect("创建 HTTP 客户端失败");

        Self {
            servers: Mutex::new(HashMap::new()),
            http_client,
        }
    }

    /// 启动指定 CLI 的代理服务器
    ///
    /// 绑定 127.0.0.1:{port}，设置上游目标并启动。
    /// 如果该 cli_id 已在运行，返回 AlreadyRunning。
    pub async fn start(
        &self,
        cli_id: &str,
        port: u16,
        upstream: UpstreamTarget,
    ) -> Result<(), ProxyError> {
        let mut servers = self.servers.lock().await;

        // 检查是否已在运行
        if let Some(existing) = servers.get(cli_id) {
            if existing.is_running() {
                return Err(ProxyError::AlreadyRunning);
            }
        }

        let mut server = ProxyServer::new(port, self.http_client.clone());
        server.state().update_upstream(upstream).await;
        server.start().await?;

        log::info!("代理已启动: cli_id={}, port={}", cli_id, server.port());
        servers.insert(cli_id.to_string(), server);
        Ok(())
    }

    /// 停止指定 CLI 的代理服务器
    ///
    /// 停止并移除该 cli_id 对应的 ProxyServer。
    /// 如果该 cli_id 不存在，返回 NotRunning。
    pub async fn stop(&self, cli_id: &str) -> Result<(), ProxyError> {
        let mut servers = self.servers.lock().await;
        let stop_result = {
            let server = servers.get_mut(cli_id).ok_or(ProxyError::NotRunning)?;
            server.stop().await
        };

        if stop_result.is_ok() {
            servers.remove(cli_id);
        }

        match stop_result {
            Ok(()) => {
                log::info!("代理已停止: cli_id={}", cli_id);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// 停止所有正在运行的代理服务器
    ///
    /// 返回每个 cli_id 的停止结果。
    pub async fn stop_all(&self) -> Vec<(String, Result<(), ProxyError>)> {
        let mut servers = self.servers.lock().await;
        let mut results = Vec::new();
        let mut stopped_cli_ids = Vec::new();

        let keys: Vec<String> = servers.keys().cloned().collect();
        for cli_id in keys {
            if let Some(server) = servers.get_mut(&cli_id) {
                let result = server.stop().await;
                if result.is_ok() {
                    stopped_cli_ids.push(cli_id.clone());
                }
                log::info!("代理停止: cli_id={}, 结果={:?}", cli_id, result);
                results.push((cli_id, result));
            }
        }

        for cli_id in stopped_cli_ids {
            servers.remove(&cli_id);
        }

        results
    }

    /// 获取所有代理服务器的运行状态
    pub async fn status(&self) -> ProxyStatusInfo {
        let servers = self.servers.lock().await;
        let mut statuses = Vec::new();

        for (cli_id, server) in servers.iter() {
            statuses.push(ServerStatus {
                cli_id: cli_id.clone(),
                port: server.port(),
                running: server.is_running(),
            });
        }

        ProxyStatusInfo { servers: statuses }
    }

    /// 动态更新指定 CLI 的上游目标
    ///
    /// 不需要重启代理服务器，运行时切换上游。
    /// 如果该 cli_id 不存在，返回 NotRunning。
    pub async fn update_upstream(
        &self,
        cli_id: &str,
        upstream: UpstreamTarget,
    ) -> Result<(), ProxyError> {
        let servers = self.servers.lock().await;

        let server = servers.get(cli_id).ok_or(ProxyError::NotRunning)?;
        server.state().update_upstream(upstream).await;

        log::info!("上游已更新: cli_id={}", cli_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, Bytes};
    use crate::provider::ProtocolType;
    use axum::Json;
    use axum::response::Response;
    use axum::Router;
    use futures::StreamExt;
    use serde_json::{json, Value};
    use std::convert::Infallible;
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use tokio::sync::{oneshot, Mutex as TokioMutex};

    /// 辅助函数：创建不走系统代理的 HTTP 客户端
    fn test_client() -> reqwest::Client {
        reqwest::Client::builder().no_proxy().build().unwrap()
    }

    /// 辅助函数：创建测试用 UpstreamTarget
    fn make_upstream(base_url: &str) -> UpstreamTarget {
        UpstreamTarget {
            api_key: "sk-test".to_string(),
            base_url: base_url.to_string(),
            protocol_type: ProtocolType::Anthropic,
        }
    }

    /// 辅助函数：启动 mock 上游服务器
    async fn start_mock_upstream_router(handler: Router) -> (u16, oneshot::Sender<()>) {
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

        tokio::time::sleep(Duration::from_millis(50)).await;
        (port, tx)
    }

    async fn start_mock_upstream(response_body: Value) -> (u16, oneshot::Sender<()>) {
        let handler = Router::new().fallback(move || {
            let body = response_body.clone();
            async move { Json(body) }
        });

        start_mock_upstream_router(handler).await
    }

    #[tokio::test]
    async fn test_proxy_service_start_stop() {
        let service = ProxyService::new();

        // 启动两个代理（使用动态端口 0）
        let upstream = make_upstream("http://127.0.0.1:1");
        service.start("claude", 0, upstream.clone()).await.unwrap();
        service.start("codex", 0, upstream.clone()).await.unwrap();

        // 验证 status 显示两个 running
        let status = service.status().await;
        assert_eq!(status.servers.len(), 2);
        assert!(status.servers.iter().all(|s| s.running));

        // 停止一个
        service.stop("claude").await.unwrap();

        // 另一个仍在运行
        let status = service.status().await;
        assert_eq!(status.servers.len(), 1);
        assert_eq!(status.servers[0].cli_id, "codex");
        assert!(status.servers[0].running);

        // 清理
        service.stop("codex").await.unwrap();
    }

    #[tokio::test]
    async fn test_proxy_service_dual_port() {
        // 启动两个 mock 上游，返回不同数据
        let (upstream1_port, shutdown1) =
            start_mock_upstream(json!({"source": "claude_upstream"})).await;
        let (upstream2_port, shutdown2) =
            start_mock_upstream(json!({"source": "codex_upstream"})).await;

        let service = ProxyService::new();

        // 启动 claude 和 codex 代理，各自指向不同上游
        service
            .start(
                "claude",
                0,
                make_upstream(&format!("http://127.0.0.1:{}", upstream1_port)),
            )
            .await
            .unwrap();
        service
            .start(
                "codex",
                0,
                make_upstream(&format!("http://127.0.0.1:{}", upstream2_port)),
            )
            .await
            .unwrap();

        // 获取两个代理的端口
        let status = service.status().await;
        let claude_port = status
            .servers
            .iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;
        let codex_port = status
            .servers
            .iter()
            .find(|s| s.cli_id == "codex")
            .unwrap()
            .port;

        // 各自转发正确
        let resp1: Value = test_client()
            .get(format!("http://127.0.0.1:{}/v1/messages", claude_port))
            .header("x-api-key", "PROXY_MANAGED")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(resp1["source"], "claude_upstream");

        let resp2: Value = test_client()
            .get(format!("http://127.0.0.1:{}/v1/messages", codex_port))
            .header("x-api-key", "PROXY_MANAGED")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(resp2["source"], "codex_upstream");

        // 清理
        service.stop_all().await;
        let _ = shutdown1.send(());
        let _ = shutdown2.send(());
    }

    #[tokio::test]
    async fn test_proxy_service_stop_all() {
        let service = ProxyService::new();
        let upstream = make_upstream("http://127.0.0.1:1");

        // 启动多个代理
        service.start("claude", 0, upstream.clone()).await.unwrap();
        service.start("codex", 0, upstream.clone()).await.unwrap();

        // 验证都在运行
        let status = service.status().await;
        assert_eq!(status.servers.len(), 2);

        // stop_all
        let results = service.stop_all().await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(_, r)| r.is_ok()));

        // 验证全部停止
        let status = service.status().await;
        assert_eq!(status.servers.len(), 0);
    }

    #[tokio::test]
    async fn test_proxy_service_update_upstream() {
        // 启动两个 mock 上游
        let (upstream1_port, shutdown1) =
            start_mock_upstream(json!({"source": "original"})).await;
        let (upstream2_port, shutdown2) =
            start_mock_upstream(json!({"source": "updated"})).await;

        let service = ProxyService::new();

        // 启动代理
        service
            .start(
                "claude",
                0,
                make_upstream(&format!("http://127.0.0.1:{}", upstream1_port)),
            )
            .await
            .unwrap();

        let status = service.status().await;
        let proxy_port = status.servers[0].port;

        // 验证初始上游
        let resp: Value = test_client()
            .get(format!("http://127.0.0.1:{}/test", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(resp["source"], "original");

        // 动态更新上游
        service
            .update_upstream(
                "claude",
                make_upstream(&format!("http://127.0.0.1:{}", upstream2_port)),
            )
            .await
            .unwrap();

        // 验证新请求走新上游
        let resp: Value = test_client()
            .get(format!("http://127.0.0.1:{}/test", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(resp["source"], "updated");

        // 清理
        service.stop("claude").await.unwrap();
        let _ = shutdown1.send(());
        let _ = shutdown2.send(());
    }

    #[tokio::test]
    async fn test_proxy_service_already_running() {
        let service = ProxyService::new();
        let upstream = make_upstream("http://127.0.0.1:1");

        // 第一次启动成功
        service.start("claude", 0, upstream.clone()).await.unwrap();

        // 第二次启动同一 cli_id 应返回 AlreadyRunning
        let result = service.start("claude", 0, upstream).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProxyError::AlreadyRunning => {}
            other => panic!("期望 AlreadyRunning，得到 {:?}", other),
        }

        // 清理
        service.stop("claude").await.unwrap();
    }

    #[tokio::test]
    async fn test_proxy_service_stop_not_running() {
        let service = ProxyService::new();

        // 停止不存在的 cli_id 应返回 NotRunning
        let result = service.stop("nonexistent").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProxyError::NotRunning => {}
            other => panic!("期望 NotRunning，得到 {:?}", other),
        }
    }

    #[test]
    fn test_proxy_port_for_cli_claude() {
        assert_eq!(proxy_port_for_cli("claude"), Some(PROXY_PORT_CLAUDE));
        assert_eq!(proxy_port_for_cli("claude"), Some(15800));
    }

    #[test]
    fn test_proxy_port_for_cli_codex() {
        assert_eq!(proxy_port_for_cli("codex"), Some(PROXY_PORT_CODEX));
        assert_eq!(proxy_port_for_cli("codex"), Some(15801));
    }

    #[test]
    fn test_proxy_port_for_cli_unknown() {
        assert_eq!(proxy_port_for_cli("unknown"), None);
        assert_eq!(proxy_port_for_cli(""), None);
        assert_eq!(proxy_port_for_cli("cursor"), None);
    }

    #[tokio::test]
    async fn test_proxy_service_update_upstream_not_running() {
        let service = ProxyService::new();

        // 更新不存在的 cli_id 应返回 NotRunning
        let result = service
            .update_upstream("nonexistent", make_upstream("http://127.0.0.1:1"))
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ProxyError::NotRunning => {}
            other => panic!("期望 NotRunning，得到 {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_proxy_service_stop_timeout_keeps_server_retriable() {
        let (release_tx, release_rx) = oneshot::channel::<()>();
        let blocker = Arc::new(TokioMutex::new(Some(release_rx)));

        let mock_app = Router::new().fallback({
            let blocker = blocker.clone();
            move || {
                let blocker = blocker.clone();
                async move {
                    let wait_rx = blocker
                        .lock()
                        .await
                        .take()
                        .expect("仅应消费一次阻塞信号");

                    let stream = futures::stream::once(async {
                        Ok::<Bytes, Infallible>(Bytes::from_static(b"data: start\n\n"))
                    })
                    .chain(futures::stream::once(async move {
                        let _ = wait_rx.await;
                        Ok::<Bytes, Infallible>(Bytes::from_static(b"data: done\n\n"))
                    }));

                    Response::builder()
                        .status(200)
                        .header("content-type", "text/event-stream")
                        .body(Body::from_stream(stream))
                        .unwrap()
                }
            }
        });
        let (upstream_port, upstream_shutdown) = start_mock_upstream_router(mock_app).await;

        let service = ProxyService::new();
        service
            .start(
                "claude",
                0,
                make_upstream(&format!("http://127.0.0.1:{}", upstream_port)),
            )
            .await
            .unwrap();

        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|server| server.cli_id == "claude")
            .unwrap()
            .port;

        let resp = test_client()
            .get(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let stop_result = service.stop("claude").await;
        assert!(matches!(stop_result, Err(ProxyError::StopTimeout)));

        let status = service.status().await;
        assert_eq!(status.servers.len(), 1);
        assert_eq!(status.servers[0].cli_id, "claude");
        assert!(status.servers[0].running);

        let _ = release_tx.send(());
        let body = resp.text().await.unwrap();
        assert!(body.contains("data: start"));
        assert!(body.contains("data: done"));

        service.stop("claude").await.unwrap();
        assert!(service.status().await.servers.is_empty());

        let _ = upstream_shutdown.send(());
    }
}
