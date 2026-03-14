use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use bytes::Bytes;
use serde_json::{json, Value};
use std::collections::HashMap;

use super::error::ProxyError;
use super::state::{ProxyState, UpstreamTarget};
use super::translate;
use crate::provider::ProtocolType;

/// 健康检查端点：GET /health -> {"status": "ok"}
pub async fn health_handler() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

/// 判断是否为 hop-by-hop header（代理不应转发）
fn is_hop_by_hop(header_name: &str) -> bool {
    matches!(
        header_name.to_lowercase().as_str(),
        "host" | "content-length" | "transfer-encoding" | "connection"
    )
}

/// 应用上游模型映射——三级优先级：精确匹配 > upstream_model 默认 > 保留原名
///
/// - 精确匹配：upstream_model_map 中有该模型名的条目时使用映射值
/// - 退回默认：无精确匹配但存在 upstream_model 时使用 upstream_model
/// - 保留原名：两者均为 None 时不修改 model 字段
fn apply_upstream_model_mapping(mut body: Value, upstream: &UpstreamTarget) -> Value {
    let original_model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();

    let mapped_model = if let Some(model_map) = &upstream.upstream_model_map {
        // 优先精确匹配
        model_map
            .get(&original_model)
            .cloned()
            .or_else(|| upstream.upstream_model.clone())
            .unwrap_or(original_model)
    } else {
        // 无 model_map，退回 upstream_model 或保留原名
        upstream
            .upstream_model
            .clone()
            .unwrap_or(original_model)
    };

    if let Some(obj) = body.as_object_mut() {
        obj.insert("model".to_string(), json!(mapped_model));
    }

    body
}

/// 全路径透传代理 handler
///
/// 接收所有未匹配 /health 的请求，替换凭据后转发到上游 Provider，
/// 流式透传响应（包括 SSE）。
pub async fn proxy_handler(
    State(state): State<ProxyState>,
    req: axum::extract::Request,
) -> Result<Response, ProxyError> {
    // 步骤 A：获取上游目标
    let upstream = state
        .get_upstream()
        .await
        .ok_or(ProxyError::NoUpstreamConfigured)?;

    // 步骤 B：提取原始请求信息
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let headers = req.headers().clone();

    // 步骤 C：读取请求 body（200MB 上限）
    let body_bytes = axum::body::to_bytes(req.into_body(), 200 * 1024 * 1024)
        .await
        .map_err(|e| ProxyError::Internal(format!("读取请求体失败: {}", e)))?;

    // 步骤 C 之后：协议路由分支
    let (upstream_url, final_body_bytes, is_streaming, request_model) =
        match upstream.protocol_type {
            ProtocolType::OpenAiChatCompletions => {
                // 1. 解析请求体
                let body_value: Value = serde_json::from_slice(&body_bytes)
                    .map_err(|e| ProxyError::TranslateError(format!("无法解析请求体: {}", e)))?;

                // 2. 提取 stream 标志（转换前读取）
                let is_streaming = body_value
                    .get("stream")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // 3. 提取原始模型名（用于流式 SSE 事件）
                let request_model = body_value
                    .get("model")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                // 4. 模型名映射（在转换前执行，MODL-03）
                let body_value = apply_upstream_model_mapping(body_value, &upstream);

                // 5. 请求转换 + 端点重写
                let openai_body = translate::request::anthropic_to_openai(body_value)?;
                let url = translate::request::build_proxy_endpoint_url(
                    &upstream.base_url,
                    "/chat/completions",
                );
                let new_bytes = serde_json::to_vec(&openai_body)
                    .map_err(|e| ProxyError::Internal(e.to_string()))?;

                (url, Bytes::from(new_bytes), is_streaming, request_model)
            }
            ProtocolType::Anthropic | ProtocolType::OpenAiResponses => {
                // 透传路径：URL 拼接与现有逻辑一致
                let url = format!(
                    "{}{}{}",
                    upstream.base_url.trim_end_matches('/'),
                    path,
                    query
                );
                (url, body_bytes, false, String::new())
            }
        };

    // 步骤 E & F：构建 reqwest 请求，透传 headers（跳过 hop-by-hop + 替换凭据）
    let mut req_builder = state.http_client.request(method, &upstream_url);

    // 跟踪是否已存在占位凭据（需要被替换）
    let mut needs_credential_injection = false;

    for (key, value) in headers.iter() {
        let key_str = key.as_str().to_lowercase();

        // 跳过 hop-by-hop headers
        if is_hop_by_hop(&key_str) {
            continue;
        }

        // 检查是否需要替换认证头
        if key_str == "x-api-key" || key_str == "authorization" {
            let val_str = value.to_str().unwrap_or("");
            if val_str == "PROXY_MANAGED" || val_str == "Bearer PROXY_MANAGED" {
                // 标记需要注入真实凭据，跳过占位头
                needs_credential_injection = true;
                continue;
            }
        }

        // 其他 headers 原样转发
        req_builder = req_builder.header(key, value);
    }

    // 步骤 G：注入真实凭据（仅当检测到占位值时）
    if needs_credential_injection {
        match upstream.protocol_type {
            ProtocolType::Anthropic => {
                req_builder = req_builder.header("x-api-key", &upstream.api_key);
            }
            ProtocolType::OpenAiChatCompletions | ProtocolType::OpenAiResponses => {
                req_builder =
                    req_builder.header("Authorization", format!("Bearer {}", upstream.api_key));
            }
        }
    }

    // 步骤 H：发送请求
    let upstream_resp = req_builder
        .body(final_body_bytes.to_vec())
        .send()
        .await
        .map_err(|e| ProxyError::UpstreamUnreachable(e.to_string()))?;

    // 步骤 I：构建响应——透传上游 status + headers
    let status = upstream_resp.status();
    let resp_headers = upstream_resp.headers().clone();

    let mut builder = Response::builder().status(status.as_u16());

    for (key, value) in resp_headers.iter() {
        let k = key.as_str().to_lowercase();
        // 过滤响应中的 hop-by-hop headers（不含 host，host 只在请求中过滤）
        if matches!(
            k.as_str(),
            "transfer-encoding" | "content-length" | "connection"
        ) {
            continue;
        }
        builder = builder.header(key, value);
    }

    // 步骤 J：按 protocol_type 分支处理响应体
    let body = match upstream.protocol_type {
        ProtocolType::OpenAiChatCompletions => {
            if !status.is_success() {
                // 4xx/5xx 直接透传（RESP-05）
                Body::from_stream(upstream_resp.bytes_stream())
            } else if is_streaming {
                // 流式：wrap 为 SSE 转换流
                Body::from_stream(translate::stream::create_anthropic_sse_stream(
                    upstream_resp.bytes_stream(),
                    request_model,
                ))
            } else {
                // 非流式：读完整响应，转换后返回
                let resp_bytes = upstream_resp
                    .bytes()
                    .await
                    .map_err(|e| ProxyError::Internal(format!("读取上游响应失败: {}", e)))?;
                let resp_value: Value = serde_json::from_slice(&resp_bytes)
                    .map_err(|e| ProxyError::TranslateError(format!("响应解析失败: {}", e)))?;
                let anthropic_resp = translate::response::openai_to_anthropic(resp_value)?;
                let resp_bytes = serde_json::to_vec(&anthropic_resp)
                    .map_err(|e| ProxyError::Internal(e.to_string()))?;
                Body::from(resp_bytes)
            }
        }
        _ => {
            // Anthropic / OpenAiResponses：透传（现有行为）
            Body::from_stream(upstream_resp.bytes_stream())
        }
    };

    builder
        .body(body)
        .map_err(|e| ProxyError::Internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::HeaderMap;

    // ── apply_upstream_model_mapping 单元测试 ──

    #[test]
    fn test_model_exact_match_wins_over_default() {
        // upstream_model_map 中有精确匹配条目时，使用映射值（优先级最高）
        let mut model_map = HashMap::new();
        model_map.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            "gpt-4o".to_string(),
        );
        let upstream = UpstreamTarget {
            api_key: "key".to_string(),
            base_url: "http://example.com".to_string(),
            protocol_type: ProtocolType::OpenAiChatCompletions,
            upstream_model: Some("gpt-3.5-turbo".to_string()), // 默认值，应被精确匹配覆盖
            upstream_model_map: Some(model_map),
        };
        let body = json!({"model": "claude-3-5-sonnet-20241022", "messages": []});
        let result = apply_upstream_model_mapping(body, &upstream);
        assert_eq!(result["model"], "gpt-4o");
    }

    #[test]
    fn test_model_fallback_to_upstream_model() {
        // map 中无精确匹配时，退回 upstream_model 默认模型
        let mut model_map = HashMap::new();
        model_map.insert("claude-3-opus".to_string(), "gpt-4-turbo".to_string());
        let upstream = UpstreamTarget {
            api_key: "key".to_string(),
            base_url: "http://example.com".to_string(),
            protocol_type: ProtocolType::OpenAiChatCompletions,
            upstream_model: Some("gpt-4o-mini".to_string()),
            upstream_model_map: Some(model_map),
        };
        // 请求的模型名不在 map 中
        let body = json!({"model": "claude-3-5-sonnet-20241022", "messages": []});
        let result = apply_upstream_model_mapping(body, &upstream);
        assert_eq!(result["model"], "gpt-4o-mini");
    }

    #[test]
    fn test_model_preserved_when_no_mapping() {
        // upstream_model 和 upstream_model_map 均为 None 时保留原模型名
        let upstream = UpstreamTarget {
            api_key: "key".to_string(),
            base_url: "http://example.com".to_string(),
            protocol_type: ProtocolType::OpenAiChatCompletions,
            upstream_model: None,
            upstream_model_map: None,
        };
        let body = json!({"model": "claude-3-5-sonnet-20241022", "messages": []});
        let result = apply_upstream_model_mapping(body, &upstream);
        assert_eq!(result["model"], "claude-3-5-sonnet-20241022");
    }

    #[tokio::test]
    async fn test_health_handler_returns_ok() {
        let (status, json) = health_handler().await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json.0["status"], "ok");
    }

    #[test]
    fn test_is_hop_by_hop() {
        assert!(is_hop_by_hop("host"));
        assert!(is_hop_by_hop("Host"));
        assert!(is_hop_by_hop("HOST"));
        assert!(is_hop_by_hop("content-length"));
        assert!(is_hop_by_hop("Content-Length"));
        assert!(is_hop_by_hop("transfer-encoding"));
        assert!(is_hop_by_hop("connection"));
        assert!(is_hop_by_hop("Connection"));

        // 非 hop-by-hop headers
        assert!(!is_hop_by_hop("content-type"));
        assert!(!is_hop_by_hop("x-api-key"));
        assert!(!is_hop_by_hop("authorization"));
        assert!(!is_hop_by_hop("accept"));
    }

    /// 测试凭据替换逻辑：模拟 headers 检查替换行为
    #[test]
    fn test_credential_replacement_anthropic_placeholder() {
        // 模拟 Anthropic 协议的占位凭据检测
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "PROXY_MANAGED".parse().unwrap());
        headers.insert("content-type", "application/json".parse().unwrap());

        let mut needs_credential_injection = false;
        let mut forwarded_headers: Vec<String> = Vec::new();

        for (key, value) in headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if is_hop_by_hop(&key_str) {
                continue;
            }
            if key_str == "x-api-key" || key_str == "authorization" {
                let val_str = value.to_str().unwrap_or("");
                if val_str == "PROXY_MANAGED" || val_str == "Bearer PROXY_MANAGED" {
                    needs_credential_injection = true;
                    continue;
                }
            }
            forwarded_headers.push(key_str);
        }

        assert!(needs_credential_injection);
        // x-api-key 不应被转发（它是占位值）
        assert!(!forwarded_headers.contains(&"x-api-key".to_string()));
        // content-type 应被保留
        assert!(forwarded_headers.contains(&"content-type".to_string()));
    }

    #[test]
    fn test_credential_replacement_openai_placeholder() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer PROXY_MANAGED".parse().unwrap());

        let mut needs_credential_injection = false;

        for (key, value) in headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if key_str == "x-api-key" || key_str == "authorization" {
                let val_str = value.to_str().unwrap_or("");
                if val_str == "PROXY_MANAGED" || val_str == "Bearer PROXY_MANAGED" {
                    needs_credential_injection = true;
                    continue;
                }
            }
        }

        assert!(needs_credential_injection);
    }

    #[test]
    fn test_non_placeholder_credential_preserved() {
        // 非 PROXY_MANAGED 的认证头应该被保留
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "sk-real-key-123".parse().unwrap());

        let mut needs_credential_injection = false;
        let mut forwarded_headers: Vec<String> = Vec::new();

        for (key, value) in headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if is_hop_by_hop(&key_str) {
                continue;
            }
            if key_str == "x-api-key" || key_str == "authorization" {
                let val_str = value.to_str().unwrap_or("");
                if val_str == "PROXY_MANAGED" || val_str == "Bearer PROXY_MANAGED" {
                    needs_credential_injection = true;
                    continue;
                }
            }
            forwarded_headers.push(key_str);
        }

        // 非占位值不应触发凭据注入
        assert!(!needs_credential_injection);
        // 原始 key 应被保留转发
        assert!(forwarded_headers.contains(&"x-api-key".to_string()));
    }

    #[test]
    fn test_hop_by_hop_headers_filtered() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "localhost:15800".parse().unwrap());
        headers.insert("content-length", "42".parse().unwrap());
        headers.insert("transfer-encoding", "chunked".parse().unwrap());
        headers.insert("connection", "keep-alive".parse().unwrap());
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("x-custom-header", "value".parse().unwrap());

        let mut forwarded: Vec<String> = Vec::new();
        for (key, _) in headers.iter() {
            let key_str = key.as_str().to_lowercase();
            if !is_hop_by_hop(&key_str) {
                forwarded.push(key_str);
            }
        }

        // hop-by-hop headers 不应出现
        assert!(!forwarded.contains(&"host".to_string()));
        assert!(!forwarded.contains(&"content-length".to_string()));
        assert!(!forwarded.contains(&"transfer-encoding".to_string()));
        assert!(!forwarded.contains(&"connection".to_string()));
        // 普通 headers 应保留
        assert!(forwarded.contains(&"content-type".to_string()));
        assert!(forwarded.contains(&"x-custom-header".to_string()));
    }
}
