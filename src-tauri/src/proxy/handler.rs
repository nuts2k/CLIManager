use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use bytes::Bytes;
use serde_json::{json, Value};

use super::error::ProxyError;
use super::state::{ProxyState, UpstreamTarget};
use super::translate;
use crate::provider::ProtocolType;
use crate::traffic::log::LogEntry;

/// 健康检查端点：GET /health -> {"status": "ok"}
pub async fn health_handler() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

#[derive(Debug)]
enum ResponseTranslationMode {
    Passthrough,
    /// Anthropic 协议透传 + 模型反向映射
    AnthropicPassthrough {
        request_model: String,
    },
    OpenAiChatCompletions {
        request_model: String,
    },
    OpenAiResponses {
        request_model: String,
    },
}

/// 判断是否为 hop-by-hop header（代理不应转发）
fn is_hop_by_hop(header_name: &str) -> bool {
    matches!(
        header_name.to_lowercase().as_str(),
        "host" | "content-length" | "transfer-encoding" | "connection"
    )
}

/// 仅 `/v1/messages` 需要做 Anthropic ↔ OpenAI 协议转换
fn should_translate_messages_request(protocol_type: &ProtocolType, path: &str) -> bool {
    matches!(
        protocol_type,
        ProtocolType::OpenAiChatCompletions | ProtocolType::OpenAiResponses
    ) && path == "/v1/messages"
}

/// 根据上游实际 Content-Type 判断是否为 SSE 响应
fn is_sse_response(headers: &reqwest::header::HeaderMap) -> bool {
    headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().contains("text/event-stream"))
}

/// 应用上游模型映射——三级优先级：精确匹配 > upstream_model 默认 > 保留原名
///
/// - 精确匹配：upstream_model_map 中有该模型名的条目时使用映射值
/// - 退回默认：无精确匹配但存在 upstream_model 时使用 upstream_model
/// - 保留原名：两者均为 None 时不修改 model 字段
fn apply_upstream_model_mapping(mut body: Value, upstream: &UpstreamTarget) -> Value {
    let original_model = extract_model_from_json_value(&body).unwrap_or_default();

    let mapped_model = if let Some(model_map) = &upstream.upstream_model_map {
        // 优先精确匹配
        model_map
            .get(&original_model)
            .cloned()
            .or_else(|| upstream.upstream_model.clone())
            .unwrap_or(original_model)
    } else {
        // 无 model_map，退回 upstream_model 或保留原名
        upstream.upstream_model.clone().unwrap_or(original_model)
    };

    if let Some(obj) = body.as_object_mut() {
        obj.insert("model".to_string(), json!(mapped_model));
    }

    body
}

fn extract_model_from_json_value(body: &Value) -> Option<String> {
    body.get("model")
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
}

fn extract_model_from_json_bytes(body_bytes: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(body_bytes)
        .ok()
        .and_then(|v| extract_model_from_json_value(&v))
}

/// 非流式 Anthropic 响应中将 model 字段替换回原始请求模型名
///
/// 仅替换顶层 `model` 字段；响应中不含 model 字段时保持原样。
fn reverse_model_in_response(mut body: Value, original_model: &str) -> Value {
    if let Some(obj) = body.as_object_mut() {
        if obj.contains_key("model") {
            obj.insert("model".to_string(), json!(original_model));
        }
    }
    body
}

/// 流式 SSE 行中将 model 字段值替换回原始模型名
///
/// - 仅处理 `data: ` 开头的行中的 JSON，其他行（event:、空行等）原样返回
/// - 替换顶层 `model` 字段（如果存在）
/// - 替换 `message.model` 嵌套字段（Anthropic message_start 事件格式）
fn reverse_model_in_sse_line(line: &str, original_model: &str) -> String {
    if let Some(json_str) = line.strip_prefix("data: ") {
        if let Ok(mut value) = serde_json::from_str::<Value>(json_str) {
            let mut modified = false;

            // 替换顶层 model 字段
            if let Some(obj) = value.as_object_mut() {
                if obj.contains_key("model") {
                    obj.insert("model".to_string(), json!(original_model));
                    modified = true;
                }
            }

            // 替换 message.model 嵌套字段（Anthropic message_start 事件）
            if let Some(msg_obj) = value.get_mut("message").and_then(|m| m.as_object_mut()) {
                if msg_obj.contains_key("model") {
                    msg_obj.insert("model".to_string(), json!(original_model));
                    modified = true;
                }
            }

            if modified {
                let new_json =
                    serde_json::to_string(&value).unwrap_or_else(|_| json_str.to_string());
                return format!("data: {}", new_json);
            }
        }
    }
    line.to_string()
}

/// 判断是否存在实际生效的上游模型映射配置。
///
/// - `upstream_model` 非空时视为启用
/// - `upstream_model_map` 仅在至少存在一条映射时视为启用
fn has_effective_upstream_model_mapping(upstream: &UpstreamTarget) -> bool {
    upstream
        .upstream_model
        .as_deref()
        .is_some_and(|model| !model.trim().is_empty())
        || upstream
            .upstream_model_map
            .as_ref()
            .is_some_and(|model_map| !model_map.is_empty())
}

/// 处理单行 SSE 字节数据。
///
/// 仅在该行是合法 UTF-8 时尝试解析并替换 `model` 字段；否则原样透传，
/// 避免因为异常字节导致再次损坏内容。
fn reverse_model_in_sse_line_bytes(line_bytes: &[u8], original_model: &str) -> Bytes {
    let trimmed = line_bytes.strip_suffix(b"\r").unwrap_or(line_bytes);

    match std::str::from_utf8(trimmed) {
        Ok(line) => Bytes::from(reverse_model_in_sse_line(line, original_model)),
        Err(_) => Bytes::copy_from_slice(trimmed),
    }
}

/// 创建 Anthropic 透传 SSE 流的 model 反向映射流
///
/// 将上游字节流按行分割，对每个 `data: {...}` 行调用 `reverse_model_in_sse_line`
/// 替换顶层 model 字段，其他行原样透传。
///
/// 内部维护行缓冲区，处理跨 chunk 的不完整行。
/// stream EOF 后通过 token_tx 回传 StreamTokenData。
fn create_anthropic_reverse_model_stream(
    upstream: impl futures::stream::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    original_model: String,
    token_tx: tokio::sync::oneshot::Sender<crate::traffic::log::StreamTokenData>,
) -> impl futures::stream::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send {
    async_stream::stream! {
        let mut token_tx = Some(token_tx);
        // 分两阶段积累 token 数据：
        // - message_start 事件提供 input_tokens + cache 字段
        // - message_delta 事件提供 output_tokens + stop_reason
        // 最终合并为一条 StreamTokenData 回传
        let mut partial_input_tokens: Option<i64> = None;
        let mut partial_cache_creation: Option<i64> = None;
        let mut partial_cache_read: Option<i64> = None;
        let mut collected_token_data: Option<crate::traffic::log::StreamTokenData> = None;
        let mut buffer = Vec::new();
        futures::pin_mut!(upstream);

        while let Some(chunk) = futures::StreamExt::next(&mut upstream).await {
            match chunk {
                Ok(bytes) => {
                    buffer.extend_from_slice(&bytes);

                    let mut processed_until = 0;

                    for (idx, byte) in buffer.iter().enumerate() {
                        if *byte != b'\n' {
                            continue;
                        }

                        let line_bytes = buffer[processed_until..idx].to_vec();
                        let processed =
                            reverse_model_in_sse_line_bytes(&line_bytes, &original_model);

                        // 从 SSE data 行提取 token 数据
                        if let Ok(line_str) = std::str::from_utf8(&line_bytes) {
                            if let Some(json_str) = line_str.strip_prefix("data: ") {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                                    match val.get("type").and_then(|t| t.as_str()) {
                                        Some("message_start") => {
                                            // message_start 包含 input_tokens 和 cache 字段
                                            // 路径：message.usage.{input_tokens, cache_creation_input_tokens, cache_read_input_tokens}
                                            let usage = val.get("message").and_then(|m| m.get("usage"));
                                            partial_input_tokens = usage.and_then(|u| u.get("input_tokens")).and_then(|v| v.as_i64());
                                            partial_cache_creation = usage.and_then(|u| u.get("cache_creation_input_tokens")).and_then(|v| v.as_i64());
                                            partial_cache_read = usage.and_then(|u| u.get("cache_read_input_tokens")).and_then(|v| v.as_i64());
                                        }
                                        Some("message_delta") => {
                                            // 某些 Anthropic 兼容上游会把最终 usage 放在 message_delta.usage 中，
                                            // 需要覆盖 message_start 的占位 0 值。
                                            let usage = val.get("usage");
                                            let input_tokens = usage.and_then(|u| u.get("input_tokens")).and_then(|v| v.as_i64()).or(partial_input_tokens);
                                            let output_tokens = usage.and_then(|u| u.get("output_tokens")).and_then(|v| v.as_i64());
                                            let cache_creation_tokens = usage.and_then(|u| u.get("cache_creation_input_tokens")).and_then(|v| v.as_i64()).or(partial_cache_creation);
                                            let cache_read_tokens = usage.and_then(|u| u.get("cache_read_input_tokens")).and_then(|v| v.as_i64()).or(partial_cache_read);
                                            let stop_reason = val.get("delta").and_then(|d| d.get("stop_reason")).and_then(|s| s.as_str()).map(|s| s.to_string());
                                            collected_token_data = Some(crate::traffic::log::StreamTokenData {
                                                input_tokens,
                                                output_tokens,
                                                cache_creation_tokens,
                                                cache_read_tokens,
                                                stop_reason,
                                            });
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }

                        let mut output = processed.to_vec();
                        output.push(b'\n');

                        yield Ok::<bytes::Bytes, std::io::Error>(bytes::Bytes::from(output));
                        processed_until = idx + 1;
                    }

                    if processed_until > 0 {
                        buffer.drain(..processed_until);
                    }
                }
                Err(err) => {
                    // 上游错误：向客户端透传读取失败，而不是伪装成正常 EOF。
                    yield Err(std::io::Error::other(format!(
                        "Anthropic SSE 上游读取失败: {}",
                        err
                    )));
                    break;
                }
            }
        }

        // 处理缓冲区中最后的不完整行
        if !buffer.is_empty() {
            let processed = reverse_model_in_sse_line_bytes(&buffer, &original_model);
            yield Ok::<bytes::Bytes, std::io::Error>(processed);
        }

        // 流结束，回传 token 数据
        if let Some(data) = collected_token_data {
            if let Some(tx) = token_tx.take() {
                let _ = tx.send(data);
            }
        }
    }
}

/// 将 ProtocolType 转换为小写字符串，用于存入 DB 的 protocol_type 列
fn protocol_type_str(pt: &ProtocolType) -> &'static str {
    match pt {
        ProtocolType::Anthropic => "anthropic",
        ProtocolType::OpenAiChatCompletions => "open_ai_chat_completions",
        ProtocolType::OpenAiResponses => "open_ai_responses",
    }
}

/// 从原始上游响应中提取 token 用量和 stop_reason。
///
/// 根据 ResponseTranslationMode 判断协议类型，从原始（转换前）响应 JSON 中提取字段。
/// 返回 (input_tokens, output_tokens, cache_creation, cache_read, stop_reason)。
/// 任何字段提取失败时返回 None（不 panic）。
fn extract_tokens_from_response(
    resp_value: &Value,
    response_mode: &ResponseTranslationMode,
) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>) {
    match response_mode {
        ResponseTranslationMode::AnthropicPassthrough { .. } => {
            extract_anthropic_tokens(resp_value)
        }
        ResponseTranslationMode::OpenAiChatCompletions { .. } => {
            extract_openai_chat_tokens(resp_value)
        }
        ResponseTranslationMode::OpenAiResponses { .. } => {
            extract_responses_tokens(resp_value)
        }
        ResponseTranslationMode::Passthrough => {
            // 透传请求（非 /v1/messages），尝试 Anthropic 格式
            extract_anthropic_tokens(resp_value)
        }
    }
}

/// Anthropic 协议 token 提取
fn extract_anthropic_tokens(
    v: &Value,
) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>) {
    let usage = v.get("usage");
    let input = usage
        .and_then(|u| u.get("input_tokens"))
        .and_then(|v| v.as_i64());
    let output = usage
        .and_then(|u| u.get("output_tokens"))
        .and_then(|v| v.as_i64());
    let cache_creation = usage
        .and_then(|u| u.get("cache_creation_input_tokens"))
        .and_then(|v| v.as_i64());
    let cache_read = usage
        .and_then(|u| u.get("cache_read_input_tokens"))
        .and_then(|v| v.as_i64());
    let stop_reason = v
        .get("stop_reason")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    (input, output, cache_creation, cache_read, stop_reason)
}

/// OpenAI Chat Completions token 提取（从原始 OpenAI 响应，非转换后的 Anthropic 格式）
fn extract_openai_chat_tokens(
    v: &Value,
) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>) {
    let usage = v.get("usage");
    let input = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|v| v.as_i64());
    let output = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|v| v.as_i64());
    // OpenAI 缓存 token: usage.prompt_tokens_details.cached_tokens
    let cache_read = usage
        .and_then(|u| u.get("prompt_tokens_details"))
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_i64());
    // OpenAI Chat Completions 无 cache_creation 概念
    let cache_creation = None;
    // stop_reason 从 choices[0].finish_reason 提取原始值（不做映射）
    let stop_reason = v
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("finish_reason"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    (input, output, cache_creation, cache_read, stop_reason)
}

/// OpenAI Responses API token 提取
fn extract_responses_tokens(
    v: &Value,
) -> (Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>) {
    let usage = v.get("usage");
    let input = usage
        .and_then(|u| u.get("input_tokens"))
        .and_then(|v| v.as_i64());
    let output = usage
        .and_then(|u| u.get("output_tokens"))
        .and_then(|v| v.as_i64());
    // Responses API 缓存字段暂不提取（Phase 27 留 null）
    let cache_creation = None;
    let cache_read = None;
    // Responses API 无统一 stop_reason 字段，留 None
    let stop_reason = None;
    (input, output, cache_creation, cache_read, stop_reason)
}

/// 在请求失败时（upstream 获取后）发送错误日志
fn send_error_log(
    state: &ProxyState,
    request_start_ms: i64,
    start_time: &std::time::Instant,
    upstream: &UpstreamTarget,
    method: &axum::http::Method,
    path: &str,
    request_model: &Option<String>,
    upstream_model: &Option<String>,
    is_streaming: bool,
    error: &ProxyError,
) {
    if let Some(tx) = state.log_sender() {
        let entry = LogEntry {
            created_at: request_start_ms,
            provider_name: upstream.provider_name.clone(),
            cli_id: state.cli_id().to_string(),
            method: method.to_string(),
            path: path.to_string(),
            status_code: None,
            is_streaming: if is_streaming { 1 } else { 0 },
            request_model: request_model.clone(),
            upstream_model: upstream_model.clone(),
            protocol_type: protocol_type_str(&upstream.protocol_type).to_string(),
            input_tokens: None,
            output_tokens: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            ttfb_ms: None,
            duration_ms: Some(start_time.elapsed().as_millis() as i64),
            stop_reason: None,
            error_message: Some(format!("{}", error)),
        };
        let _ = tx.try_send(entry);
    }
}

/// 全路径透传代理 handler
///
/// 接收所有未匹配 /health 的请求，替换凭据后转发到上游 Provider，
/// 流式透传响应（包括 SSE）。
pub async fn proxy_handler(
    State(state): State<ProxyState>,
    req: axum::extract::Request,
) -> Result<Response, ProxyError> {
    // 日志计时：在请求最开始记录，供后续日志发送使用
    let start_time = std::time::Instant::now();
    let request_start_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    // 步骤 A：获取上游目标
    // 注意：NoUpstreamConfigured 时不记录流量日志 —— 有意设计。
    // 此时无可用 UpstreamTarget（无 provider_name / api_key），LogEntry 的 provider_name
    // 是 NOT NULL 字段无法填写有意义的值。此错误属于配置问题而非请求错误。
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

    // 在 body_bytes 被 move 之前提取 request_model（用于日志）
    let log_request_model = extract_model_from_json_bytes(&body_bytes);

    // 步骤 C 之后：协议路由分支
    let (upstream_url, final_body_bytes, response_mode) = match upstream.protocol_type {
        ProtocolType::OpenAiChatCompletions
            if should_translate_messages_request(&upstream.protocol_type, &path) =>
        {
            // 1. 解析请求体
            let body_value: Value = serde_json::from_slice(&body_bytes)
                .map_err(|e| ProxyError::TranslateError(format!("无法解析请求体: {}", e)))?;

            // 2. 提取原始模型名（用于流式 SSE 事件）
            let request_model = body_value
                .get("model")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown")
                .to_string();

            // 3. 模型名映射（在转换前执行，MODL-03）
            let body_value = apply_upstream_model_mapping(body_value, &upstream);

            // 4. 请求转换 + 端点重写
            let openai_body = translate::request::anthropic_to_openai(body_value)?;
            let url = translate::request::build_proxy_endpoint_url(
                &upstream.base_url,
                "/chat/completions",
            );
            let new_bytes = serde_json::to_vec(&openai_body)
                .map_err(|e| ProxyError::Internal(e.to_string()))?;

            (
                url,
                Bytes::from(new_bytes),
                ResponseTranslationMode::OpenAiChatCompletions { request_model },
            )
        }
        ProtocolType::OpenAiResponses
            if should_translate_messages_request(&upstream.protocol_type, &path) =>
        {
            // 1. 解析请求体
            let body_value: Value = serde_json::from_slice(&body_bytes)
                .map_err(|e| ProxyError::TranslateError(format!("无法解析请求体: {}", e)))?;

            // 2. 提取原始模型名（用于流式 SSE 事件）
            let request_model = body_value
                .get("model")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown")
                .to_string();

            // 3. 模型名映射（在转换前执行）
            let body_value = apply_upstream_model_mapping(body_value, &upstream);

            // 4. 请求转换 + 端点重写为 /responses
            let responses_body = translate::responses_request::anthropic_to_responses(body_value)?;
            let url =
                translate::request::build_proxy_endpoint_url(&upstream.base_url, "/responses");
            let new_bytes = serde_json::to_vec(&responses_body)
                .map_err(|e| ProxyError::Internal(e.to_string()))?;

            (
                url,
                Bytes::from(new_bytes),
                ResponseTranslationMode::OpenAiResponses { request_model },
            )
        }
        ProtocolType::Anthropic if path == "/v1/messages" => {
            // Anthropic 协议 + /v1/messages：统一走 AnthropicPassthrough 以支持 token 提取
            // 从 body 中提取 model 名（GET 请求或空 body 时优雅降级，不中断请求）
            let request_model: String = serde_json::from_slice::<Value>(&body_bytes)
                .ok()
                .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());
            let has_mapping = has_effective_upstream_model_mapping(&upstream);
            let url = translate::request::build_upstream_url(&upstream.base_url, &path, &query);
            if has_mapping {
                // 有映射时解析请求体、替换模型名、重新序列化
                let body_value: Value = serde_json::from_slice(&body_bytes)
                    .map_err(|e| ProxyError::TranslateError(format!("无法解析请求体: {}", e)))?;
                let mapped = apply_upstream_model_mapping(body_value, &upstream);
                let new_bytes = serde_json::to_vec(&mapped)
                    .map_err(|e| ProxyError::Internal(e.to_string()))?;
                (
                    url,
                    Bytes::from(new_bytes),
                    ResponseTranslationMode::AnthropicPassthrough { request_model },
                )
            } else {
                // 无映射时原样透传 body bytes（避免重新序列化损失格式）
                (url, body_bytes, ResponseTranslationMode::AnthropicPassthrough { request_model })
            }
        }
        _ => {
            // 非 `/v1/messages` 请求保持透传，避免误改写 token_count / complete 等端点
            let url = translate::request::build_upstream_url(&upstream.base_url, &path, &query);
            (url, body_bytes, ResponseTranslationMode::Passthrough)
        }
    };
    let log_upstream_model = extract_model_from_json_bytes(&final_body_bytes);

    // 步骤 E & F：构建 reqwest 请求，透传 headers（跳过 hop-by-hop + 替换凭据）
    let mut req_builder = state.http_client.request(method.clone(), &upstream_url);

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
        .map_err(|e| {
            let err = ProxyError::UpstreamUnreachable(e.to_string());
            send_error_log(
                &state,
                request_start_ms,
                &start_time,
                &upstream,
                &method,
                &path,
                &log_request_model,
                &log_upstream_model,
                false,
                &err,
            );
            err
        })?;

    // 步骤 I：构建响应——透传上游 status + headers
    let ttfb_ms = start_time.elapsed().as_millis() as i64;

    let status = upstream_resp.status();
    let resp_headers = upstream_resp.headers().clone();
    let upstream_is_sse = is_sse_response(&resp_headers);

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
    // 日志 token 变量：非流式分支中赋值，流式和透传分支保持 None
    let mut log_input_tokens: Option<i64> = None;
    let mut log_output_tokens: Option<i64> = None;
    let mut log_cache_creation: Option<i64> = None;
    let mut log_cache_read: Option<i64> = None;
    let mut log_stop_reason: Option<String> = None;

    // 流式请求的 oneshot receiver（stream EOF 后由后台 task 接收 token 数据并 UPDATE DB）
    let mut streaming_token_rx: Option<tokio::sync::oneshot::Receiver<crate::traffic::log::StreamTokenData>> = None;

    let body = match response_mode {
        ResponseTranslationMode::OpenAiChatCompletions { request_model } => {
            if !status.is_success() {
                // 4xx/5xx 直接透传（RESP-05）
                Body::from_stream(upstream_resp.bytes_stream())
            } else if upstream_is_sse {
                // 流式：创建 oneshot channel，wrap 为 SSE 转换流
                let (tx, rx) = tokio::sync::oneshot::channel::<crate::traffic::log::StreamTokenData>();
                streaming_token_rx = Some(rx);
                Body::from_stream(translate::stream::create_anthropic_sse_stream(
                    upstream_resp.bytes_stream(),
                    request_model,
                    tx,
                ))
            } else {
                // 非流式：读完整响应，提取 token，转换后返回
                let resp_bytes = upstream_resp
                    .bytes()
                    .await
                    .map_err(|e| ProxyError::Internal(format!("读取上游响应失败: {}", e)))?;
                let resp_value: Value = serde_json::from_slice(&resp_bytes)
                    .map_err(|e| ProxyError::TranslateError(format!("响应解析失败: {}", e)))?;
                // 在 resp_value 被 move 之前提取 token（从原始 OpenAI 格式）
                let (it, ot, cc, cr, sr) =
                    extract_openai_chat_tokens(&resp_value);
                log_input_tokens = it;
                log_output_tokens = ot;
                log_cache_creation = cc;
                log_cache_read = cr;
                log_stop_reason = sr;
                let anthropic_resp = translate::response::openai_to_anthropic(resp_value)?;
                let resp_bytes = serde_json::to_vec(&anthropic_resp)
                    .map_err(|e| ProxyError::Internal(e.to_string()))?;
                Body::from(resp_bytes)
            }
        }
        ResponseTranslationMode::OpenAiResponses { request_model } => {
            if !status.is_success() {
                // 4xx/5xx 直接透传（RESP-05）
                Body::from_stream(upstream_resp.bytes_stream())
            } else if upstream_is_sse {
                // 流式：创建 oneshot channel，wrap 为 Responses API -> Anthropic SSE 转换流
                let (tx, rx) = tokio::sync::oneshot::channel::<crate::traffic::log::StreamTokenData>();
                streaming_token_rx = Some(rx);
                Body::from_stream(
                    translate::responses_stream::create_responses_anthropic_sse_stream(
                        upstream_resp.bytes_stream(),
                        request_model,
                        tx,
                    ),
                )
            } else {
                // 非流式：读完整响应，提取 token，转换后返回
                let resp_bytes = upstream_resp
                    .bytes()
                    .await
                    .map_err(|e| ProxyError::Internal(format!("读取上游响应失败: {}", e)))?;
                let resp_value: Value = serde_json::from_slice(&resp_bytes)
                    .map_err(|e| ProxyError::TranslateError(format!("响应解析失败: {}", e)))?;
                // 在 resp_value 被 move 之前提取 token（从原始 Responses API 格式）
                let (it, ot, cc, cr, sr) =
                    extract_responses_tokens(&resp_value);
                log_input_tokens = it;
                log_output_tokens = ot;
                log_cache_creation = cc;
                log_cache_read = cr;
                log_stop_reason = sr;
                let anthropic_resp =
                    translate::responses_response::responses_to_anthropic(resp_value)?;
                let resp_bytes = serde_json::to_vec(&anthropic_resp)
                    .map_err(|e| ProxyError::Internal(e.to_string()))?;
                Body::from(resp_bytes)
            }
        }
        ResponseTranslationMode::AnthropicPassthrough { request_model } => {
            if !status.is_success() {
                // 4xx/5xx 错误响应直接透传，不做 model 替换
                Body::from_stream(upstream_resp.bytes_stream())
            } else if upstream_is_sse {
                // 流式：创建 oneshot channel，逐行扫描替换 model 字段
                let (tx, rx) = tokio::sync::oneshot::channel::<crate::traffic::log::StreamTokenData>();
                streaming_token_rx = Some(rx);
                Body::from_stream(create_anthropic_reverse_model_stream(
                    upstream_resp.bytes_stream(),
                    request_model,
                    tx,
                ))
            } else {
                // 非流式：读完整响应，提取 token，替换 model 后返回
                let resp_bytes = upstream_resp
                    .bytes()
                    .await
                    .map_err(|e| ProxyError::Internal(format!("读取上游响应失败: {}", e)))?;
                let resp_value: Value = serde_json::from_slice(&resp_bytes)
                    .map_err(|e| ProxyError::TranslateError(format!("响应解析失败: {}", e)))?;
                // 在 resp_value 被 move 之前提取 token（从原始 Anthropic 格式）
                let (it, ot, cc, cr, sr) =
                    extract_anthropic_tokens(&resp_value);
                log_input_tokens = it;
                log_output_tokens = ot;
                log_cache_creation = cc;
                log_cache_read = cr;
                log_stop_reason = sr;
                let reversed = reverse_model_in_response(resp_value, &request_model);
                let resp_bytes = serde_json::to_vec(&reversed)
                    .map_err(|e| ProxyError::Internal(e.to_string()))?;
                Body::from(resp_bytes)
            }
        }
        ResponseTranslationMode::Passthrough => {
            // 透传（现有行为，token 留 None）
            Body::from_stream(upstream_resp.bytes_stream())
        }
    };

    // 日志采集
    let entry = LogEntry {
        created_at: request_start_ms,
        provider_name: upstream.provider_name.clone(),
        cli_id: state.cli_id().to_string(),
        method: method.to_string(),
        path: path.clone(),
        status_code: Some(status.as_u16() as i64),
        is_streaming: if upstream_is_sse { 1 } else { 0 },
        request_model: log_request_model,
        upstream_model: log_upstream_model,
        protocol_type: protocol_type_str(&upstream.protocol_type).to_string(),
        input_tokens: log_input_tokens,
        output_tokens: log_output_tokens,
        cache_creation_tokens: log_cache_creation,
        cache_read_tokens: log_cache_read,
        ttfb_ms: Some(ttfb_ms), // TTFB 统一采样（send().await 返回后）
        duration_ms: if upstream_is_sse {
            None // 流式请求 duration 在 EOF 后由后台 task 填充
        } else {
            Some(start_time.elapsed().as_millis() as i64)
        },
        stop_reason: log_stop_reason,
        error_message: None,
    };

    let mut log_row_id: Option<i64> = None;

    if upstream_is_sse {
        // 流式请求：直接 INSERT 获取 rowid（不走 log_worker channel，以便获取 id 用于后续 UPDATE）
        // 技术原因：mpsc log_worker 是 fire-and-forget 模式无法返回 rowid，
        // 而流式请求需要 rowid 以便 stream EOF 后 UPDATE token/duration。
        // 与 STORE-03 描述略有偏差但功能等价，经 Phase 28 验证正确。
        if let Some(app_handle) = state.app_handle() {
            use tauri::{Emitter, Manager};
            if let Some(db) = app_handle.try_state::<crate::traffic::TrafficDb>() {
                match db.insert_request_log(&entry) {
                    Ok(id) => {
                        // emit type="new"（token 为 None 的初始状态）
                        let payload = crate::traffic::log::TrafficLogPayload::from_entry(id, &entry, "new");
                        if let Err(e) = app_handle.emit("traffic-log", &payload) {
                            log::warn!("流式日志 emit new 失败: {}", e);
                        }
                        log_row_id = Some(id);
                    }
                    Err(e) => log::warn!("流式日志 INSERT 失败: {}", e),
                }
            }
        }
    } else {
        // 非流式请求：走 log_worker channel（现有逻辑）
        if let Some(tx) = state.log_sender() {
            if let Err(e) = tx.try_send(entry.clone()) {
                log::warn!("日志 channel 发送失败（可能已满）: {}", e);
            }
        }
    }

    // 流式请求：spawn 后台 task 等待 stream EOF 后 UPDATE token/duration 并 emit update
    if let (Some(rx), Some(row_id)) = (streaming_token_rx, log_row_id) {
        let app_handle = state.app_handle().cloned();
        let start_time_clone = start_time;
        let ttfb = ttfb_ms;
        let entry_for_update = entry.clone();

        tokio::spawn(async move {
            match rx.await {
                Ok(token_data) => {
                    let duration_ms = start_time_clone.elapsed().as_millis() as i64;
                    if let Some(ref handle) = app_handle {
                        use tauri::{Emitter, Manager};
                        if let Some(db) = handle.try_state::<crate::traffic::TrafficDb>() {
                            if let Err(e) = db.update_streaming_log(row_id, &token_data, Some(ttfb), Some(duration_ms)) {
                                log::warn!("流式日志 UPDATE 失败: {}", e);
                            } else {
                                // emit type="update"
                                let mut updated_entry = entry_for_update;
                                updated_entry.input_tokens = token_data.input_tokens;
                                updated_entry.output_tokens = token_data.output_tokens;
                                updated_entry.cache_creation_tokens = token_data.cache_creation_tokens;
                                updated_entry.cache_read_tokens = token_data.cache_read_tokens;
                                updated_entry.stop_reason = token_data.stop_reason;
                                updated_entry.ttfb_ms = Some(ttfb);
                                updated_entry.duration_ms = Some(duration_ms);
                                let payload = crate::traffic::log::TrafficLogPayload::from_entry(row_id, &updated_entry, "update");
                                if let Err(e) = handle.emit("traffic-log", &payload) {
                                    log::warn!("emit traffic-log update 失败: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // oneshot sender 被 drop（stream 异常中断，客户端断开）
                    log::debug!("流式 token 回传 channel 已关闭（stream 可能被客户端中断）");
                }
            }
        });
    }

    builder
        .body(body)
        .map_err(|e| ProxyError::Internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::HeaderMap;
    use axum::Router;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tokio::sync::{oneshot, Mutex as TokioMutex};

    // ── handler.rs 内部辅助（用于路由分支测试） ──

    fn make_upstream_responses_target(base_url: &str) -> super::super::state::UpstreamTarget {
        super::super::state::UpstreamTarget {
            api_key: "sk-test".to_string(),
            base_url: base_url.to_string(),
            protocol_type: crate::provider::ProtocolType::OpenAiResponses,
            upstream_model: None,
            upstream_model_map: None,
            provider_name: "test".to_string(),
        }
    }

    fn make_upstream_openai_target(base_url: &str) -> super::super::state::UpstreamTarget {
        super::super::state::UpstreamTarget {
            api_key: "sk-test".to_string(),
            base_url: base_url.to_string(),
            protocol_type: crate::provider::ProtocolType::OpenAiChatCompletions,
            upstream_model: None,
            upstream_model_map: None,
            provider_name: "test".to_string(),
        }
    }

    async fn assert_non_messages_request_passthrough(protocol_type: ProtocolType) {
        use axum::routing::post;

        let captured_uri: Arc<TokioMutex<Option<String>>> = Arc::new(TokioMutex::new(None));
        let captured_body: Arc<TokioMutex<Option<Value>>> = Arc::new(TokioMutex::new(None));
        let captured_uri_clone = captured_uri.clone();
        let captured_body_clone = captured_body.clone();
        let passthrough_resp = json!({"input_tokens": 42});
        let passthrough_resp_for_route = passthrough_resp.clone();

        let mock_app = Router::new().route(
            "/v1/token_count",
            post(move |req: axum::extract::Request| {
                let captured_uri = captured_uri_clone.clone();
                let captured_body = captured_body_clone.clone();
                let resp = passthrough_resp_for_route.clone();
                async move {
                    let uri = req
                        .uri()
                        .path_and_query()
                        .map(|value| value.as_str().to_string())
                        .unwrap_or_else(|| req.uri().path().to_string());
                    let body_bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024)
                        .await
                        .unwrap();
                    let body_value: Value = serde_json::from_slice(&body_bytes).unwrap();
                    *captured_uri.lock().await = Some(uri);
                    *captured_body.lock().await = Some(body_value);
                    axum::Json(resp)
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let base_url = format!("http://127.0.0.1:{}", upstream_port);
        let upstream = super::super::state::UpstreamTarget {
            api_key: "sk-test".to_string(),
            base_url,
            protocol_type,
            upstream_model: None,
            upstream_model_map: None,
            provider_name: "test".to_string(),
        };

        let service = crate::proxy::ProxyService::new();
        service.start("claude", 0, upstream).await.unwrap();

        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let passthrough_req = json!({
            "text": "count these tokens",
            "metadata": {"source": "test"}
        });

        let resp: Value = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!(
                "http://127.0.0.1:{}/v1/token_count?beta=true",
                proxy_port
            ))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&passthrough_req)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(
            captured_uri.lock().await.as_deref(),
            Some("/v1/token_count?beta=true"),
            "非 /v1/messages 请求不应被重写"
        );
        assert_eq!(
            captured_body.lock().await.as_ref(),
            Some(&passthrough_req),
            "非 /v1/messages 请求体应保持透传"
        );
        assert_eq!(resp, passthrough_resp, "非 /v1/messages 响应应保持透传");

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    // ── Task 1 TDD RED：OpenAiResponses 路由分支行为测试 ──

    /// 验证 OpenAiResponses 请求被路由到 /responses 端点（而非 /chat/completions）
    #[tokio::test]
    async fn test_responses_api_endpoint() {
        use axum::routing::post;

        // 记录请求路径
        let captured_path: Arc<TokioMutex<Option<String>>> = Arc::new(TokioMutex::new(None));
        let captured_path_clone = captured_path.clone();

        // mock 上游返回简单 Responses API 格式响应
        let mock_resp = json!({
            "id": "resp_test",
            "object": "response",
            "output": [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "Hello"}], "status": "completed"}],
            "usage": {"input_tokens": 5, "output_tokens": 3}
        });

        let mock_app = Router::new().route(
            "/v1/responses",
            post(move |req: axum::extract::Request| {
                let captured = captured_path_clone.clone();
                let resp = mock_resp.clone();
                async move {
                    *captured.lock().await = Some(req.uri().path().to_string());
                    axum::Json(resp)
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let service = crate::proxy::ProxyService::new();
        let base_url = format!("http://127.0.0.1:{}", upstream_port);
        service
            .start("claude", 0, make_upstream_responses_target(&base_url))
            .await
            .unwrap();

        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        // 发送 Anthropic 格式请求
        let anthropic_req = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 100,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let _ = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&anthropic_req)
            .send()
            .await
            .unwrap();

        // 验证请求命中 /v1/responses（而非 /v1/chat/completions）
        let path = captured_path.lock().await;
        assert_eq!(
            path.as_deref(),
            Some("/v1/responses"),
            "OpenAiResponses 请求应路由到 /v1/responses 端点"
        );

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    /// 验证 OpenAiResponses 请求体被转换为 Responses API 格式（包含 input 而非 messages）
    #[tokio::test]
    async fn test_responses_api_routing() {
        use axum::routing::post;

        let captured_body: Arc<TokioMutex<Option<Value>>> = Arc::new(TokioMutex::new(None));
        let captured_body_clone = captured_body.clone();

        let mock_resp = json!({
            "id": "resp_test",
            "object": "response",
            "output": [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "Hello"}], "status": "completed"}],
            "usage": {"input_tokens": 5, "output_tokens": 3}
        });

        let mock_app = Router::new().route(
            "/v1/responses",
            post(move |body: axum::extract::Json<Value>| {
                let captured = captured_body_clone.clone();
                let resp = mock_resp.clone();
                async move {
                    *captured.lock().await = Some(body.0);
                    axum::Json(resp)
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let service = crate::proxy::ProxyService::new();
        let base_url = format!("http://127.0.0.1:{}", upstream_port);
        service
            .start("claude", 0, make_upstream_responses_target(&base_url))
            .await
            .unwrap();

        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let anthropic_req = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 100,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let _ = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&anthropic_req)
            .send()
            .await
            .unwrap();

        // 验证上游收到的是 Responses API 格式（有 input 字段，无 messages 字段）
        let body = captured_body.lock().await;
        let received = body.as_ref().expect("mock 上游应已收到请求");
        assert!(
            received.get("input").is_some(),
            "Responses API 请求应包含 input 字段"
        );
        assert!(
            received.get("messages").is_none(),
            "Responses API 请求不应包含 messages 字段"
        );
        assert_eq!(
            received.get("max_output_tokens").and_then(|v| v.as_u64()),
            Some(100),
            "max_tokens 应映射为 max_output_tokens"
        );

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    /// 验证模型映射在 Responses API 请求转换前执行
    #[tokio::test]
    async fn test_responses_api_model_mapping() {
        use axum::routing::post;
        use std::collections::HashMap;

        let captured_body: Arc<TokioMutex<Option<Value>>> = Arc::new(TokioMutex::new(None));
        let captured_body_clone = captured_body.clone();

        let mock_resp = json!({
            "id": "resp_test",
            "object": "response",
            "output": [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "Hello"}], "status": "completed"}],
            "usage": {"input_tokens": 5, "output_tokens": 3}
        });

        let mock_app = Router::new().route(
            "/v1/responses",
            post(move |body: axum::extract::Json<Value>| {
                let captured = captured_body_clone.clone();
                let resp = mock_resp.clone();
                async move {
                    *captured.lock().await = Some(body.0);
                    axum::Json(resp)
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 带模型映射的 UpstreamTarget
        let mut model_map = HashMap::new();
        model_map.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            "o1-mini".to_string(),
        );
        let upstream = super::super::state::UpstreamTarget {
            api_key: "sk-test".to_string(),
            base_url: format!("http://127.0.0.1:{}", upstream_port),
            protocol_type: crate::provider::ProtocolType::OpenAiResponses,
            upstream_model: None,
            upstream_model_map: Some(model_map),
            provider_name: "test".to_string(),
        };

        let service = crate::proxy::ProxyService::new();
        service.start("claude", 0, upstream).await.unwrap();

        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let anthropic_req = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 100,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let _ = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&anthropic_req)
            .send()
            .await
            .unwrap();

        let body = captured_body.lock().await;
        let received = body.as_ref().expect("mock 上游应已收到请求");
        assert_eq!(
            received["model"], "o1-mini",
            "模型名应在转换前被映射为 o1-mini"
        );

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    #[tokio::test]
    async fn test_openai_chat_non_messages_request_passthrough() {
        assert_non_messages_request_passthrough(ProtocolType::OpenAiChatCompletions).await;
    }

    #[tokio::test]
    async fn test_openai_responses_non_messages_request_passthrough() {
        assert_non_messages_request_passthrough(ProtocolType::OpenAiResponses).await;
    }

    #[tokio::test]
    async fn test_openai_non_messages_request_passthrough_with_base_path_prefix() {
        use axum::routing::post;

        let captured_uri: Arc<TokioMutex<Option<String>>> = Arc::new(TokioMutex::new(None));
        let captured_uri_clone = captured_uri.clone();

        let mock_app = Router::new().route(
            "/openai/v1/token_count",
            post(move |req: axum::extract::Request| {
                let captured = captured_uri_clone.clone();
                async move {
                    *captured.lock().await = Some(
                        req.uri()
                            .path_and_query()
                            .map(|value| value.as_str().to_string())
                            .unwrap_or_else(|| req.uri().path().to_string()),
                    );
                    axum::Json(json!({"count": 7}))
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let upstream = super::super::state::UpstreamTarget {
            api_key: "sk-test".to_string(),
            base_url: format!("http://127.0.0.1:{}/openai/v1", upstream_port),
            protocol_type: ProtocolType::OpenAiChatCompletions,
            upstream_model: None,
            upstream_model_map: None,
            provider_name: "test".to_string(),
        };

        let service = crate::proxy::ProxyService::new();
        service.start("claude", 0, upstream).await.unwrap();

        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let resp: Value = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!(
                "http://127.0.0.1:{}/v1/token_count?beta=true",
                proxy_port
            ))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&json!({"text": "count these tokens"}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(
            captured_uri.lock().await.as_deref(),
            Some("/openai/v1/token_count?beta=true"),
            "带路径前缀的 OpenAI base_url 不应拼出重复 /v1"
        );
        assert_eq!(resp["count"], 7);

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    #[tokio::test]
    async fn test_openai_chat_stream_request_with_json_fallback() {
        use axum::routing::post;

        let mock_resp = json!({
            "id": "chatcmpl-fallback",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello fallback"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
        });

        let mock_app = Router::new().route(
            "/v1/chat/completions",
            post(move || {
                let resp = mock_resp.clone();
                async move { axum::Json(resp) }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let service = crate::proxy::ProxyService::new();
        let base_url = format!("http://127.0.0.1:{}", upstream_port);
        service
            .start("claude", 0, make_upstream_openai_target(&base_url))
            .await
            .unwrap();

        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let anthropic_req = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 100,
            "stream": true,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let resp = client
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&anthropic_req)
            .send()
            .await
            .unwrap();

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body: Value = resp.json().await.unwrap();

        assert!(
            content_type.contains("application/json"),
            "上游返回 JSON 时不应误转成 SSE"
        );
        assert_eq!(body["content"][0]["type"], "text");
        assert_eq!(body["content"][0]["text"], "Hello fallback");
        assert_eq!(body["stop_reason"], "end_turn");

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    #[tokio::test]
    async fn test_openai_responses_stream_request_with_json_fallback() {
        use axum::routing::post;

        let mock_resp = json!({
            "id": "resp-fallback",
            "object": "response",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Hello fallback"}],
                "status": "completed"
            }],
            "usage": {"input_tokens": 5, "output_tokens": 3}
        });

        let mock_app = Router::new().route(
            "/v1/responses",
            post(move || {
                let resp = mock_resp.clone();
                async move { axum::Json(resp) }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let service = crate::proxy::ProxyService::new();
        let base_url = format!("http://127.0.0.1:{}", upstream_port);
        service
            .start("claude", 0, make_upstream_responses_target(&base_url))
            .await
            .unwrap();

        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let anthropic_req = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 100,
            "stream": true,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let resp = client
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&anthropic_req)
            .send()
            .await
            .unwrap();

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body: Value = resp.json().await.unwrap();

        assert!(
            content_type.contains("application/json"),
            "上游返回 JSON 时不应误转成 SSE"
        );
        assert_eq!(body["content"][0]["type"], "text");
        assert_eq!(body["content"][0]["text"], "Hello fallback");
        assert_eq!(body["stop_reason"], "end_turn");

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

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
            provider_name: "test".to_string(),
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
            provider_name: "test".to_string(),
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
            provider_name: "test".to_string(),
        };
        let body = json!({"model": "claude-3-5-sonnet-20241022", "messages": []});
        let result = apply_upstream_model_mapping(body, &upstream);
        assert_eq!(result["model"], "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_extract_model_from_json_bytes_returns_actual_mapped_model() {
        let mut model_map = HashMap::new();
        model_map.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            "gpt-4o".to_string(),
        );
        let upstream = UpstreamTarget {
            api_key: "key".to_string(),
            base_url: "http://example.com".to_string(),
            protocol_type: ProtocolType::OpenAiChatCompletions,
            upstream_model: Some("gpt-4o-mini".to_string()),
            upstream_model_map: Some(model_map),
            provider_name: "test".to_string(),
        };
        let body = json!({"model": "claude-3-5-sonnet-20241022", "messages": []});
        let mapped = apply_upstream_model_mapping(body, &upstream);
        let mapped_bytes = serde_json::to_vec(&mapped).unwrap();

        assert_eq!(
            extract_model_from_json_bytes(&mapped_bytes),
            Some("gpt-4o".to_string())
        );
    }

    #[test]
    fn test_extract_model_from_json_bytes_preserves_passthrough_model() {
        let body = br#"{"model":"claude-3-5-sonnet-20241022","messages":[]}"#;

        assert_eq!(
            extract_model_from_json_bytes(body),
            Some("claude-3-5-sonnet-20241022".to_string())
        );
    }

    #[tokio::test]
    async fn test_health_handler_returns_ok() {
        let (status, json) = health_handler().await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json.0["status"], "ok");
    }

    // ──────────────────────────────────────────────────────────────
    // Anthropic 透传分支测试（MMAP-01 / MMAP-02 / MMAP-03）
    // ──────────────────────────────────────────────────────────────

    fn make_upstream_anthropic_target(base_url: &str) -> super::super::state::UpstreamTarget {
        super::super::state::UpstreamTarget {
            api_key: "sk-ant-test".to_string(),
            base_url: base_url.to_string(),
            protocol_type: crate::provider::ProtocolType::Anthropic,
            upstream_model: None,
            upstream_model_map: None,
            provider_name: "test".to_string(),
        }
    }

    /// 测试 1：Anthropic + /v1/messages + upstream_model_map 精确匹配 → 转发请求 model 为映射后值
    #[tokio::test]
    async fn test_anthropic_messages_model_map_exact_match() {
        use axum::routing::post;

        let captured_body: Arc<TokioMutex<Option<Value>>> = Arc::new(TokioMutex::new(None));
        let captured_body_clone = captured_body.clone();

        let mock_app = Router::new().route(
            "/v1/messages",
            post(move |body: axum::extract::Json<Value>| {
                let captured = captured_body_clone.clone();
                async move {
                    *captured.lock().await = Some(body.0.clone());
                    axum::Json(json!({
                        "id": "msg_test",
                        "type": "message",
                        "role": "assistant",
                        "model": "mapped-model-name",
                        "content": [{"type": "text", "text": "Hello"}],
                        "stop_reason": "end_turn",
                        "usage": {"input_tokens": 5, "output_tokens": 3}
                    }))
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut model_map = HashMap::new();
        model_map.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            "mapped-model-name".to_string(),
        );
        let upstream = super::super::state::UpstreamTarget {
            api_key: "sk-ant-test".to_string(),
            base_url: format!("http://127.0.0.1:{}", upstream_port),
            protocol_type: crate::provider::ProtocolType::Anthropic,
            upstream_model: None,
            upstream_model_map: Some(model_map),
            provider_name: "test".to_string(),
        };

        let service = crate::proxy::ProxyService::new();
        service.start("claude", 0, upstream).await.unwrap();
        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let _ = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&json!({
                "model": "claude-3-5-sonnet-20241022",
                "max_tokens": 100,
                "messages": [{"role": "user", "content": "Hello"}]
            }))
            .send()
            .await
            .unwrap();

        let body = captured_body.lock().await;
        let received = body.as_ref().expect("mock 上游应已收到请求");
        assert_eq!(
            received["model"], "mapped-model-name",
            "Anthropic 精确匹配：请求 model 应被映射为 mapped-model-name"
        );

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    /// 测试 2：Anthropic + /v1/messages + 只有 upstream_model（无 map）→ 转发 model 为 upstream_model
    #[tokio::test]
    async fn test_anthropic_messages_upstream_model_fallback() {
        use axum::routing::post;

        let captured_body: Arc<TokioMutex<Option<Value>>> = Arc::new(TokioMutex::new(None));
        let captured_body_clone = captured_body.clone();

        let mock_app = Router::new().route(
            "/v1/messages",
            post(move |body: axum::extract::Json<Value>| {
                let captured = captured_body_clone.clone();
                async move {
                    *captured.lock().await = Some(body.0.clone());
                    axum::Json(json!({
                        "id": "msg_test",
                        "type": "message",
                        "role": "assistant",
                        "model": "default-upstream-model",
                        "content": [{"type": "text", "text": "Hello"}],
                        "stop_reason": "end_turn",
                        "usage": {"input_tokens": 5, "output_tokens": 3}
                    }))
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let upstream = super::super::state::UpstreamTarget {
            api_key: "sk-ant-test".to_string(),
            base_url: format!("http://127.0.0.1:{}", upstream_port),
            protocol_type: crate::provider::ProtocolType::Anthropic,
            upstream_model: Some("default-upstream-model".to_string()),
            upstream_model_map: None,
            provider_name: "test".to_string(),
        };

        let service = crate::proxy::ProxyService::new();
        service.start("claude", 0, upstream).await.unwrap();
        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let _ = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&json!({
                "model": "claude-3-5-sonnet-20241022",
                "max_tokens": 100,
                "messages": [{"role": "user", "content": "Hello"}]
            }))
            .send()
            .await
            .unwrap();

        let body = captured_body.lock().await;
        let received = body.as_ref().expect("mock 上游应已收到请求");
        assert_eq!(
            received["model"], "default-upstream-model",
            "Anthropic upstream_model 兜底：请求 model 应被替换为 default-upstream-model"
        );

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    /// 测试 3：Anthropic + /v1/messages + 无任何映射配置 → 转发 model 保持原值不变
    #[tokio::test]
    async fn test_anthropic_messages_no_mapping_passthrough() {
        use axum::routing::post;

        let captured_body: Arc<TokioMutex<Option<Value>>> = Arc::new(TokioMutex::new(None));
        let captured_body_clone = captured_body.clone();

        let mock_app = Router::new().route(
            "/v1/messages",
            post(move |body: axum::extract::Json<Value>| {
                let captured = captured_body_clone.clone();
                async move {
                    *captured.lock().await = Some(body.0.clone());
                    axum::Json(json!({
                        "id": "msg_test",
                        "type": "message",
                        "role": "assistant",
                        "model": "claude-3-5-sonnet-20241022",
                        "content": [{"type": "text", "text": "Hello"}],
                        "stop_reason": "end_turn",
                        "usage": {"input_tokens": 5, "output_tokens": 3}
                    }))
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let service = crate::proxy::ProxyService::new();
        service
            .start(
                "claude",
                0,
                make_upstream_anthropic_target(&format!("http://127.0.0.1:{}", upstream_port)),
            )
            .await
            .unwrap();
        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let _ = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&json!({
                "model": "claude-3-5-sonnet-20241022",
                "max_tokens": 100,
                "messages": [{"role": "user", "content": "Hello"}]
            }))
            .send()
            .await
            .unwrap();

        let body = captured_body.lock().await;
        let received = body.as_ref().expect("mock 上游应已收到请求");
        assert_eq!(
            received["model"], "claude-3-5-sonnet-20241022",
            "无映射配置时，请求 model 应保持原值"
        );

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    /// 测试 4：Anthropic + /v1/messages + 有映射 → 上游非流式响应 model 字段被替换回原始名
    #[tokio::test]
    async fn test_anthropic_messages_response_model_reverse_mapped() {
        use axum::routing::post;

        let mock_app = Router::new().route(
            "/v1/messages",
            post(move || async move {
                axum::Json(json!({
                    "id": "msg_test",
                    "type": "message",
                    "role": "assistant",
                    "model": "mapped-model-name",  // 上游返回映射后的名字
                    "content": [{"type": "text", "text": "Hello"}],
                    "stop_reason": "end_turn",
                    "usage": {"input_tokens": 5, "output_tokens": 3}
                }))
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut model_map = HashMap::new();
        model_map.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            "mapped-model-name".to_string(),
        );
        let upstream = super::super::state::UpstreamTarget {
            api_key: "sk-ant-test".to_string(),
            base_url: format!("http://127.0.0.1:{}", upstream_port),
            protocol_type: crate::provider::ProtocolType::Anthropic,
            upstream_model: None,
            upstream_model_map: Some(model_map),
            provider_name: "test".to_string(),
        };

        let service = crate::proxy::ProxyService::new();
        service.start("claude", 0, upstream).await.unwrap();
        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let resp: Value = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&json!({
                "model": "claude-3-5-sonnet-20241022",
                "max_tokens": 100,
                "messages": [{"role": "user", "content": "Hello"}]
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(
            resp["model"], "claude-3-5-sonnet-20241022",
            "非流式响应：model 字段应被反向映射回原始请求模型名"
        );

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    /// 测试 5：Anthropic + /v1/messages + 有映射 → 流式 SSE 中 model 字段被替换回原始名
    #[tokio::test]
    async fn test_anthropic_messages_sse_model_reverse_mapped() {
        use axum::response::sse::{Event, Sse};
        use axum::routing::post;
        use futures::stream;

        let mock_app = Router::new().route(
            "/v1/messages",
            post(move || async move {
                // 返回包含 model 字段的 SSE 事件
                let events = vec![
                    Ok::<Event, std::convert::Infallible>(Event::default()
                        .data(r#"{"type":"message_start","message":{"id":"msg_test","type":"message","role":"assistant","model":"mapped-model-name","content":[],"stop_reason":null,"usage":{"input_tokens":5,"output_tokens":0}}}"#)
                    ),
                    Ok(Event::default()
                        .data(r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#)
                    ),
                    Ok(Event::default()
                        .data(r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":3}}"#)
                    ),
                    Ok(Event::default().data("[DONE]")),
                ];
                Sse::new(stream::iter(events))
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = listener.local_addr().unwrap().port();
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            axum::serve(listener, mock_app)
                .with_graceful_shutdown(async {
                    rx.await.ok();
                })
                .await
                .ok();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut model_map = HashMap::new();
        model_map.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            "mapped-model-name".to_string(),
        );
        let upstream = super::super::state::UpstreamTarget {
            api_key: "sk-ant-test".to_string(),
            base_url: format!("http://127.0.0.1:{}", upstream_port),
            protocol_type: crate::provider::ProtocolType::Anthropic,
            upstream_model: None,
            upstream_model_map: Some(model_map),
            provider_name: "test".to_string(),
        };

        let service = crate::proxy::ProxyService::new();
        service.start("claude", 0, upstream).await.unwrap();
        let proxy_port = service
            .status()
            .await
            .servers
            .into_iter()
            .find(|s| s.cli_id == "claude")
            .unwrap()
            .port;

        let resp = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!("http://127.0.0.1:{}/v1/messages", proxy_port))
            .header("x-api-key", "PROXY_MANAGED")
            .header("content-type", "application/json")
            .json(&json!({
                "model": "claude-3-5-sonnet-20241022",
                "max_tokens": 100,
                "stream": true,
                "messages": [{"role": "user", "content": "Hello"}]
            }))
            .send()
            .await
            .unwrap();

        let body_text = resp.text().await.unwrap();

        // 验证 SSE 中 message_start 事件的 model 已被反向映射
        assert!(
            body_text.contains("claude-3-5-sonnet-20241022"),
            "SSE 流式响应中 model 字段应被反向映射回原始请求模型名。收到:\n{}",
            body_text
        );
        assert!(
            !body_text.contains("mapped-model-name"),
            "SSE 流式响应中不应包含上游映射模型名。收到:\n{}",
            body_text
        );

        service.stop("claude").await.unwrap();
        let _ = tx.send(());
    }

    /// 测试 5.1：Anthropic SSE 包装器应在跨 chunk UTF-8 字符下保持正文不损坏
    #[tokio::test]
    async fn test_create_anthropic_reverse_model_stream_preserves_split_utf8() {
        use futures::stream;
        use futures::StreamExt;

        let raw = "data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"你\"}}\n\n";
        let raw_bytes = raw.as_bytes();
        let split_start = raw_bytes
            .windows("你".len())
            .position(|window| window == "你".as_bytes())
            .unwrap();
        let split_at = split_start + 1;

        let chunks = vec![
            Ok::<Bytes, reqwest::Error>(Bytes::copy_from_slice(&raw_bytes[..split_at])),
            Ok::<Bytes, reqwest::Error>(Bytes::copy_from_slice(&raw_bytes[split_at..])),
        ];

        let (token_tx, _token_rx) = tokio::sync::oneshot::channel();
        let output = create_anthropic_reverse_model_stream(
            stream::iter(chunks),
            "claude-3-5-sonnet-20241022".to_string(),
            token_tx,
        )
        .collect::<Vec<_>>()
        .await;

        let mut combined = Vec::new();
        for chunk in output {
            combined.extend_from_slice(&chunk.unwrap());
        }

        assert_eq!(
            String::from_utf8(combined).unwrap(),
            raw,
            "跨 chunk 的 UTF-8 字符应保持原样透传"
        );
    }

    #[tokio::test]
    async fn test_create_anthropic_reverse_model_stream_prefers_message_delta_usage_when_start_is_placeholder() {
        use futures::stream;
        use futures::StreamExt;

        let chunks = vec![
            Ok::<Bytes, reqwest::Error>(Bytes::from_static(
                b"data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_test\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"mapped-model-name\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":0,\"output_tokens\":0}}}\n\n",
            )),
            Ok::<Bytes, reqwest::Error>(Bytes::from_static(
                b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"input_tokens\":792,\"output_tokens\":14,\"cache_read_input_tokens\":125440}}\n\n",
            )),
            Ok::<Bytes, reqwest::Error>(Bytes::from_static(b"data: [DONE]\n\n")),
        ];

        let (token_tx, token_rx) = tokio::sync::oneshot::channel();
        let _: Vec<_> = create_anthropic_reverse_model_stream(
            stream::iter(chunks),
            "claude-3-5-sonnet-20241022".to_string(),
            token_tx,
        )
        .collect()
        .await;

        let token_data = token_rx.await.expect("应收到 token 数据");
        assert_eq!(token_data.input_tokens, Some(792));
        assert_eq!(token_data.output_tokens, Some(14));
        assert_eq!(token_data.cache_read_tokens, Some(125440));
        assert_eq!(token_data.stop_reason.as_deref(), Some("end_turn"));
    }

    /// 测试 5.2：Anthropic SSE 包装器应把上游读取错误产出为 Err
    #[tokio::test]
    async fn test_create_anthropic_reverse_model_stream_propagates_upstream_error() {
        use futures::stream;
        use futures::StreamExt;

        let upstream_error = reqwest::Client::new()
            .get("http://[::1")
            .build()
            .expect_err("非法 URL 应构造出 reqwest::Error");

        let chunks = vec![
            Ok::<Bytes, reqwest::Error>(Bytes::from_static(
                b"data: {\"type\":\"message_start\",\"message\":{\"model\":\"mapped-model-name\"}}\n\n",
            )),
            Err(upstream_error),
        ];

        let (token_tx, _token_rx) = tokio::sync::oneshot::channel();
        let output = create_anthropic_reverse_model_stream(
            stream::iter(chunks),
            "claude-3-5-sonnet-20241022".to_string(),
            token_tx,
        )
        .collect::<Vec<_>>()
        .await;

        assert!(output.len() >= 2, "应至少包含已透传的数据和最终错误");
        let successful_bytes = output[..output.len() - 1]
            .iter()
            .map(|item| item.as_ref().expect("错误前的 chunk 应成功"))
            .fold(Vec::new(), |mut acc, chunk| {
                acc.extend_from_slice(chunk);
                acc
            });
        assert!(
            String::from_utf8(successful_bytes)
                .unwrap()
                .contains("claude-3-5-sonnet-20241022"),
            "错误前的已透传数据应完成 model 反向映射"
        );
        assert!(
            output.last().is_some_and(|item| item.is_err()),
            "上游读取错误不应被静默吞掉"
        );
    }

    /// 测试 6：Anthropic + 非 /v1/messages 路径（如 /v1/token_count）→ 保持纯透传，不做任何映射
    #[tokio::test]
    async fn test_anthropic_non_messages_path_passthrough() {
        assert_non_messages_request_passthrough(crate::provider::ProtocolType::Anthropic).await;
    }

    // ── reverse_model_in_response 单元测试 ──

    #[test]
    fn test_reverse_model_in_response_replaces_model() {
        let body = json!({
            "id": "msg_test",
            "model": "mapped-model-name",
            "content": [{"type": "text", "text": "Hello"}]
        });
        let result = reverse_model_in_response(body, "claude-3-5-sonnet-20241022");
        assert_eq!(result["model"], "claude-3-5-sonnet-20241022");
        // 其他字段保持不变
        assert_eq!(result["id"], "msg_test");
    }

    #[test]
    fn test_reverse_model_in_response_no_model_field() {
        // 响应中不含 model 字段时，不报错，其他字段保持
        let body = json!({
            "id": "msg_test",
            "content": [{"type": "text", "text": "Hello"}]
        });
        let result = reverse_model_in_response(body, "claude-3-5-sonnet-20241022");
        assert!(result.get("model").is_none(), "无 model 字段时不应插入");
        assert_eq!(result["id"], "msg_test");
    }

    // ── reverse_model_in_sse_line 单元测试 ──

    #[test]
    fn test_reverse_model_in_sse_line_replaces_model() {
        // 注意：该函数替换的是顶层 model 字段；message_start 嵌套结构不含顶层 model
        // 只测试含顶层 model 的事件行
        let line2 = r#"data: {"model":"mapped-name","type":"text"}"#;
        let result = reverse_model_in_sse_line(line2, "claude-3-5-sonnet-20241022");
        let json_part = result.strip_prefix("data: ").unwrap();
        let v: Value = serde_json::from_str(json_part).unwrap();
        assert_eq!(v["model"], "claude-3-5-sonnet-20241022");
        // 其他字段保持
        assert_eq!(v["type"], "text");
    }

    #[test]
    fn test_reverse_model_in_sse_line_no_model() {
        // 无 model 字段的行：原样返回
        let line = r#"data: {"type":"content_block_delta","delta":{"text":"Hi"}}"#;
        let result = reverse_model_in_sse_line(line, "claude-3-5-sonnet-20241022");
        // 不含 model 字段，行内容不含模型名
        let json_part = result.strip_prefix("data: ").unwrap();
        let v: Value = serde_json::from_str(json_part).unwrap();
        assert!(v.get("model").is_none());
    }

    #[test]
    fn test_reverse_model_in_sse_line_non_data_line() {
        // event: 行和空行不做修改
        let event_line = "event: message_start";
        let result = reverse_model_in_sse_line(event_line, "claude-3-5-sonnet-20241022");
        assert_eq!(result, event_line);

        let empty_line = "";
        let result2 = reverse_model_in_sse_line(empty_line, "claude-3-5-sonnet-20241022");
        assert_eq!(result2, empty_line);
    }

    #[test]
    fn test_has_effective_upstream_model_mapping_ignores_empty_map() {
        let upstream = super::super::state::UpstreamTarget {
            api_key: "sk-ant-test".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            protocol_type: crate::provider::ProtocolType::Anthropic,
            upstream_model: None,
            upstream_model_map: Some(HashMap::new()),
            provider_name: "test".to_string(),
        };

        assert!(
            !has_effective_upstream_model_mapping(&upstream),
            "空映射表不应触发 AnthropicPassthrough 分支"
        );
    }

    #[test]
    fn test_has_effective_upstream_model_mapping_accepts_non_empty_map() {
        let mut model_map = HashMap::new();
        model_map.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            "mapped-model-name".to_string(),
        );
        let upstream = super::super::state::UpstreamTarget {
            api_key: "sk-ant-test".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            protocol_type: crate::provider::ProtocolType::Anthropic,
            upstream_model: None,
            upstream_model_map: Some(model_map),
            provider_name: "test".to_string(),
        };

        assert!(has_effective_upstream_model_mapping(&upstream));
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

    // ── Task 2：token 提取函数单元测试 ──

    /// 验证 extract_anthropic_tokens 从完整 Anthropic 响应中正确提取所有字段
    #[test]
    fn test_extract_anthropic_tokens_full() {
        let v = json!({
            "id": "msg_test",
            "type": "message",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 42,
                "cache_creation_input_tokens": 30,
                "cache_read_input_tokens": 20
            }
        });
        let (input, output, cc, cr, sr) = extract_anthropic_tokens(&v);
        assert_eq!(input, Some(100));
        assert_eq!(output, Some(42));
        assert_eq!(cc, Some(30));
        assert_eq!(cr, Some(20));
        assert_eq!(sr, Some("end_turn".to_string()));
    }

    /// 验证 extract_anthropic_tokens 对缺少 usage 的 JSON 返回全 None
    #[test]
    fn test_extract_anthropic_tokens_no_usage() {
        let v = json!({"id": "msg_test", "type": "message"});
        let (input, output, cc, cr, sr) = extract_anthropic_tokens(&v);
        assert_eq!(input, None);
        assert_eq!(output, None);
        assert_eq!(cc, None);
        assert_eq!(cr, None);
        assert_eq!(sr, None);
    }

    /// 验证 extract_openai_chat_tokens 从完整 OpenAI Chat Completions 响应中正确提取所有字段
    #[test]
    fn test_extract_openai_chat_tokens_full() {
        let v = json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 25,
                "total_tokens": 75,
                "prompt_tokens_details": {
                    "cached_tokens": 10
                }
            }
        });
        let (input, output, cc, cr, sr) = extract_openai_chat_tokens(&v);
        assert_eq!(input, Some(50));
        assert_eq!(output, Some(25));
        assert_eq!(cc, None); // Chat Completions 无 cache_creation
        assert_eq!(cr, Some(10));
        assert_eq!(sr, Some("stop".to_string()));
    }

    /// 验证 extract_openai_chat_tokens 在缺少 prompt_tokens_details 时 cache_read 返回 None
    #[test]
    fn test_extract_openai_chat_tokens_no_cache() {
        let v = json!({
            "choices": [{
                "finish_reason": "length"
            }],
            "usage": {
                "prompt_tokens": 30,
                "completion_tokens": 15
            }
        });
        let (input, output, cc, cr, sr) = extract_openai_chat_tokens(&v);
        assert_eq!(input, Some(30));
        assert_eq!(output, Some(15));
        assert_eq!(cc, None);
        assert_eq!(cr, None); // 无 prompt_tokens_details
        assert_eq!(sr, Some("length".to_string()));
    }

    /// 验证 extract_responses_tokens 正确提取 input/output，cache 字段为 None
    #[test]
    fn test_extract_responses_tokens() {
        let v = json!({
            "id": "resp_test",
            "object": "response",
            "usage": {
                "input_tokens": 80,
                "output_tokens": 35
            }
        });
        let (input, output, cc, cr, sr) = extract_responses_tokens(&v);
        assert_eq!(input, Some(80));
        assert_eq!(output, Some(35));
        assert_eq!(cc, None); // Phase 27 留 null
        assert_eq!(cr, None); // Phase 27 留 null
        assert_eq!(sr, None); // Responses API 无统一 stop_reason 字段
    }

    /// 验证 protocol_type_str 对三种 ProtocolType 返回正确的小写字符串
    #[test]
    fn test_protocol_type_str() {
        assert_eq!(
            protocol_type_str(&ProtocolType::Anthropic),
            "anthropic"
        );
        assert_eq!(
            protocol_type_str(&ProtocolType::OpenAiChatCompletions),
            "open_ai_chat_completions"
        );
        assert_eq!(
            protocol_type_str(&ProtocolType::OpenAiResponses),
            "open_ai_responses"
        );
    }
}
