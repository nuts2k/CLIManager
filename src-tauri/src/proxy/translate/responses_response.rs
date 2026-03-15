//! 响应转换：OpenAI Responses API 非流式响应 → Anthropic Messages API 响应
//!
//! 公开函数：
//! - `responses_to_anthropic()` — 将 Responses API 非流式响应 body 转换为 Anthropic Messages 响应格式

use crate::proxy::error::ProxyError;
use serde_json::{json, Value};

/// 将 OpenAI Responses API 非流式响应 body 转换为 Anthropic Messages API 响应格式
///
/// 处理以下情况：
/// - output_text 类型内容 → text content block
/// - function_call 类型内容 → tool_use content block（call_id → id）
/// - usage 字段直接透传（命名相同）
/// - stop_reason 从 output 内容推断
/// - id 前缀替换 resp_ → msg_
pub fn responses_to_anthropic(body: Value) -> Result<Value, ProxyError> {
    let output = body
        .get("output")
        .and_then(|o| o.as_array())
        .ok_or_else(|| ProxyError::TranslateError("响应中缺少 output 字段".to_string()))?;

    let mut content: Vec<Value> = Vec::new();
    let mut has_function_call = false;
    let mut incomplete_status = false;

    for item in output {
        let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match item_type {
            "message" => {
                // 提取 content 数组，处理 output_text 类型
                if let Some(parts) = item.get("content").and_then(|c| c.as_array()) {
                    for part in parts {
                        let part_type = part.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if part_type == "output_text" {
                            let text = part.get("text").and_then(|t| t.as_str()).unwrap_or("");
                            content.push(json!({"type": "text", "text": text}));
                        }
                    }
                }

                // 检查 status 是否为 incomplete（max_tokens）
                if item.get("status").and_then(|s| s.as_str()) == Some("incomplete") {
                    incomplete_status = true;
                }
            }
            "function_call" => {
                has_function_call = true;

                // 使用 call_id（不是 id）映射到 tool_use.id
                let call_id = item.get("call_id").and_then(|c| c.as_str()).unwrap_or("");
                let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args_str = item
                    .get("arguments")
                    .and_then(|a| a.as_str())
                    .unwrap_or("{}");

                // arguments 反序列化失败时包装为 {"raw": "原字符串"}
                let input: Value =
                    serde_json::from_str(args_str).unwrap_or_else(|_| json!({"raw": args_str}));

                content.push(json!({
                    "type": "tool_use",
                    "id": call_id,
                    "name": name,
                    "input": input
                }));
            }
            _ => {}
        }
    }

    // stop_reason 推断逻辑
    let stop_reason = if has_function_call {
        "tool_use"
    } else if incomplete_status {
        "max_tokens"
    } else {
        "end_turn"
    };

    // usage 直接透传（Responses API 命名与 Anthropic 相同）
    let usage = body.get("usage").cloned().unwrap_or(json!({}));
    let input_tokens = usage
        .get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output_tokens = usage
        .get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let usage_json = json!({
        "input_tokens": input_tokens,
        "output_tokens": output_tokens
    });

    // id 前缀替换 resp_ → msg_
    let raw_id = body.get("id").and_then(|i| i.as_str()).unwrap_or("");
    let response_id = if raw_id.starts_with("resp_") {
        format!("msg_{}", &raw_id["resp_".len()..])
    } else if raw_id.starts_with("msg_") {
        raw_id.to_string()
    } else if raw_id.is_empty() {
        String::new()
    } else {
        format!("msg_{}", raw_id)
    };

    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();

    let result = json!({
        "id": response_id,
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": content,
        "stop_reason": stop_reason,
        "stop_sequence": null,
        "usage": usage_json
    });

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── 测试 1：output_text → text content block ──

    #[test]
    fn test_text_response() {
        let input = json!({
            "id": "resp_abc123",
            "object": "response",
            "model": "gpt-4o",
            "output": [
                {
                    "id": "msg_xyz",
                    "type": "message",
                    "role": "assistant",
                    "status": "completed",
                    "content": [
                        { "type": "output_text", "text": "Hello!", "annotations": [] }
                    ]
                }
            ],
            "usage": { "input_tokens": 10, "output_tokens": 5, "total_tokens": 15 }
        });

        let result = responses_to_anthropic(input).unwrap();

        assert_eq!(result["type"], "message");
        assert_eq!(result["role"], "assistant");
        let content = result["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Hello!");
        assert_eq!(result["stop_reason"], "end_turn");
        assert_eq!(result["usage"]["input_tokens"], 10);
        assert_eq!(result["usage"]["output_tokens"], 5);
    }

    // ── 测试 2：function_call → tool_use content block ──

    #[test]
    fn test_function_call_response() {
        let input = json!({
            "id": "resp_def456",
            "object": "response",
            "model": "gpt-4o",
            "output": [
                {
                    "type": "function_call",
                    "call_id": "call_abc",
                    "name": "get_weather",
                    "arguments": "{\"location\": \"Tokyo\"}"
                }
            ],
            "usage": { "input_tokens": 15, "output_tokens": 10 }
        });

        let result = responses_to_anthropic(input).unwrap();

        let content = result["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "tool_use");
        // 注意：使用 call_id（不是 id）映射到 tool_use.id
        assert_eq!(content[0]["id"], "call_abc");
        assert_eq!(content[0]["name"], "get_weather");
        assert_eq!(content[0]["input"]["location"], "Tokyo");
    }

    // ── 测试 3：文本 + 工具调用混合响应 ──

    #[test]
    fn test_mixed_text_and_function_call() {
        let input = json!({
            "id": "resp_mix789",
            "object": "response",
            "model": "gpt-4o",
            "output": [
                {
                    "id": "msg_text",
                    "type": "message",
                    "role": "assistant",
                    "status": "completed",
                    "content": [
                        { "type": "output_text", "text": "Let me check the weather." }
                    ]
                },
                {
                    "type": "function_call",
                    "call_id": "call_xyz",
                    "name": "get_weather",
                    "arguments": "{\"city\": \"Tokyo\"}"
                }
            ],
            "usage": { "input_tokens": 20, "output_tokens": 15 }
        });

        let result = responses_to_anthropic(input).unwrap();

        let content = result["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        // 第一个是 text block
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Let me check the weather.");
        // 第二个是 tool_use block
        assert_eq!(content[1]["type"], "tool_use");
        assert_eq!(content[1]["name"], "get_weather");
        // 有 function_call → stop_reason 为 tool_use
        assert_eq!(result["stop_reason"], "tool_use");
    }

    // ── 测试 4：usage 字段直接透传 ──

    #[test]
    fn test_usage_passthrough() {
        let input = json!({
            "id": "resp_usage",
            "object": "response",
            "model": "gpt-4o",
            "output": [
                {
                    "id": "msg_u",
                    "type": "message",
                    "role": "assistant",
                    "status": "completed",
                    "content": [{ "type": "output_text", "text": "ok" }]
                }
            ],
            "usage": { "input_tokens": 100, "output_tokens": 50, "total_tokens": 150 }
        });

        let result = responses_to_anthropic(input).unwrap();
        assert_eq!(result["usage"]["input_tokens"], 100);
        assert_eq!(result["usage"]["output_tokens"], 50);
        // total_tokens 可省略，不做强制要求
    }

    // ── 测试 5：stop_reason 推断 ──

    #[test]
    fn test_stop_reason_inference() {
        // 有 function_call → tool_use
        let with_function_call = json!({
            "id": "resp_fn",
            "output": [
                {
                    "type": "function_call",
                    "call_id": "c1",
                    "name": "fn1",
                    "arguments": "{}"
                }
            ],
            "usage": { "input_tokens": 5, "output_tokens": 3 }
        });
        let result = responses_to_anthropic(with_function_call).unwrap();
        assert_eq!(result["stop_reason"], "tool_use");

        // 无 function_call + status:completed → end_turn
        let completed = json!({
            "id": "resp_done",
            "output": [
                {
                    "id": "msg_d",
                    "type": "message",
                    "role": "assistant",
                    "status": "completed",
                    "content": [{ "type": "output_text", "text": "done" }]
                }
            ],
            "usage": { "input_tokens": 5, "output_tokens": 3 }
        });
        let result = responses_to_anthropic(completed).unwrap();
        assert_eq!(result["stop_reason"], "end_turn");

        // status:incomplete → max_tokens
        let incomplete = json!({
            "id": "resp_inc",
            "output": [
                {
                    "id": "msg_i",
                    "type": "message",
                    "role": "assistant",
                    "status": "incomplete",
                    "content": [{ "type": "output_text", "text": "truncated..." }]
                }
            ],
            "usage": { "input_tokens": 5, "output_tokens": 100 }
        });
        let result = responses_to_anthropic(incomplete).unwrap();
        assert_eq!(result["stop_reason"], "max_tokens");
    }

    // ── 测试 6：id 前缀替换 resp_ → msg_ ──

    #[test]
    fn test_id_prefix_mapping() {
        let input = json!({
            "id": "resp_abc123",
            "object": "response",
            "model": "gpt-4o",
            "output": [
                {
                    "id": "msg_xyz",
                    "type": "message",
                    "role": "assistant",
                    "status": "completed",
                    "content": [{ "type": "output_text", "text": "Hi" }]
                }
            ],
            "usage": { "input_tokens": 1, "output_tokens": 1 }
        });

        let result = responses_to_anthropic(input).unwrap();
        // resp_ → msg_
        assert_eq!(result["id"], "msg_abc123");
    }

    // ── 测试 7：错误响应不经此函数处理（handler 层判断） ──

    // 注意：此测试验证函数对合法 Responses API 响应的处理，
    // 4xx/5xx 错误在 handler 层判断，不传入此函数。

    // ── 测试 8：arguments 反序列化失败时包装为 {"raw": "原字符串"} ──

    #[test]
    fn test_arguments_deserialize_fallback() {
        let input = json!({
            "id": "resp_bad",
            "object": "response",
            "model": "gpt-4o",
            "output": [
                {
                    "type": "function_call",
                    "call_id": "call_bad",
                    "name": "broken_tool",
                    "arguments": "invalid json {"
                }
            ],
            "usage": { "input_tokens": 5, "output_tokens": 5 }
        });

        let result = responses_to_anthropic(input).unwrap();
        let content = result["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "tool_use");
        // 反序列化失败，包装为 {"raw": ...}
        assert!(content[0]["input"].get("raw").is_some());
        assert_eq!(content[0]["input"]["raw"], "invalid json {");
    }
}
