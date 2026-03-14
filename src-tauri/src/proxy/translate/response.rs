//! 响应转换：OpenAI Chat Completions 非流式响应 → Anthropic Messages API 响应
//!
//! 公开函数：
//! - `openai_to_anthropic()` — 将 OpenAI 非流式响应 body 转换为 Anthropic Messages 响应格式
//! - `map_finish_reason()` — 将 finish_reason 字符串映射为 Anthropic stop_reason（供 stream.rs 复用）

use crate::proxy::error::ProxyError;
use serde_json::{json, Value};

/// 将 OpenAI finish_reason 映射为 Anthropic stop_reason
///
/// 映射规则：
/// - "stop" | "content_filter" | "" | 未知 => "end_turn"
/// - "length" => "max_tokens"
/// - "tool_calls" | "function_call" => "tool_use"
pub fn map_finish_reason(reason: &str) -> &'static str {
    match reason {
        "stop" => "end_turn",
        "length" => "max_tokens",
        "tool_calls" | "function_call" => "tool_use",
        "content_filter" => "end_turn",
        _ => "end_turn",
    }
}

/// 将 OpenAI Chat Completions 非流式响应 body 转换为 Anthropic Messages API 响应格式
///
/// 处理以下情况：
/// - 基础文本响应（RESP-01）
/// - 工具调用响应（RESP-02）
/// - finish_reason 映射（RESP-03）
/// - usage 字段重命名（RESP-04）
///
/// 注意：4xx/5xx 错误响应不经此函数处理（RESP-05 是 handler 层逻辑）
pub fn openai_to_anthropic(body: Value) -> Result<Value, ProxyError> {
    let choices = body
        .get("choices")
        .and_then(|c| c.as_array())
        .ok_or_else(|| ProxyError::TranslateError("响应中缺少 choices 字段".to_string()))?;

    let choice = choices
        .first()
        .ok_or_else(|| ProxyError::TranslateError("choices 数组为空".to_string()))?;

    let message = choice
        .get("message")
        .ok_or_else(|| ProxyError::TranslateError("choice 中缺少 message 字段".to_string()))?;

    let mut content: Vec<Value> = Vec::new();

    // 文本内容（RESP-01）
    if let Some(msg_content) = message.get("content") {
        match msg_content {
            Value::String(text) => {
                // 空字符串 => 空 content 数组
                if !text.is_empty() {
                    content.push(json!({"type": "text", "text": text}));
                }
            }
            Value::Null => {
                // content 为 null，检查消息级别的 refusal 字段
                if let Some(refusal) = message.get("refusal").and_then(|r| r.as_str()) {
                    if !refusal.is_empty() {
                        content.push(json!({"type": "text", "text": refusal}));
                    }
                }
            }
            _ => {}
        }
    }

    // 工具调用（RESP-02）
    if let Some(tool_calls) = message.get("tool_calls").and_then(|t| t.as_array()) {
        for tc in tool_calls {
            let id = tc.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let empty_obj = json!({});
            let func = tc.get("function").unwrap_or(&empty_obj);
            let name = func.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args_str = func
                .get("arguments")
                .and_then(|a| a.as_str())
                .unwrap_or("{}");

            // arguments 反序列化失败时包装为 {"raw": "原字符串"}
            let input: Value = serde_json::from_str(args_str)
                .unwrap_or_else(|_| json!({"raw": args_str}));

            content.push(json!({
                "type": "tool_use",
                "id": id,
                "name": name,
                "input": input
            }));
        }
    }

    // finish_reason 映射（RESP-03）
    let stop_reason = choice
        .get("finish_reason")
        .and_then(|r| r.as_str())
        .map(map_finish_reason);

    // usage 字段重命名（RESP-04）
    let usage = body.get("usage").cloned().unwrap_or(json!({}));
    let input_tokens = usage
        .get("prompt_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output_tokens = usage
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let mut usage_json = json!({
        "input_tokens": input_tokens,
        "output_tokens": output_tokens
    });

    // cache token 映射：prompt_tokens_details.cached_tokens => cache_read_input_tokens
    if let Some(cached) = usage
        .pointer("/prompt_tokens_details/cached_tokens")
        .and_then(|v| v.as_u64())
    {
        usage_json["cache_read_input_tokens"] = json!(cached);
    }
    // 兼容直接返回 cache 字段的服务商
    if let Some(v) = usage.get("cache_read_input_tokens") {
        usage_json["cache_read_input_tokens"] = v.clone();
    }
    if let Some(v) = usage.get("cache_creation_input_tokens") {
        usage_json["cache_creation_input_tokens"] = v.clone();
    }

    // id 透传：若原 id 不含 "msg_" 前缀则添加
    let raw_id = body.get("id").and_then(|i| i.as_str()).unwrap_or("");
    let response_id = if raw_id.starts_with("msg_") {
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

    // --- map_finish_reason 测试 ---

    #[test]
    fn test_map_finish_reason_stop() {
        assert_eq!(map_finish_reason("stop"), "end_turn");
    }

    #[test]
    fn test_map_finish_reason_length() {
        assert_eq!(map_finish_reason("length"), "max_tokens");
    }

    #[test]
    fn test_map_finish_reason_tool_calls() {
        assert_eq!(map_finish_reason("tool_calls"), "tool_use");
    }

    #[test]
    fn test_map_finish_reason_function_call() {
        assert_eq!(map_finish_reason("function_call"), "tool_use");
    }

    #[test]
    fn test_map_finish_reason_content_filter() {
        assert_eq!(map_finish_reason("content_filter"), "end_turn");
    }

    #[test]
    fn test_map_finish_reason_empty_string() {
        assert_eq!(map_finish_reason(""), "end_turn");
    }

    #[test]
    fn test_map_finish_reason_unknown() {
        assert_eq!(map_finish_reason("unknown_reason"), "end_turn");
        assert_eq!(map_finish_reason("some_future_reason"), "end_turn");
    }

    // --- openai_to_anthropic 测试 ---

    /// RESP-01：基础文本响应
    #[test]
    fn test_basic_text_response() {
        let input = json!({
            "id": "chatcmpl-abc123",
            "object": "chat.completion",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "你好！"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 3, "total_tokens": 13}
        });

        let result = openai_to_anthropic(input).unwrap();

        // id 前缀
        assert_eq!(result["id"], "msg_chatcmpl-abc123");
        assert_eq!(result["type"], "message");
        assert_eq!(result["role"], "assistant");
        assert_eq!(result["model"], "gpt-4o");

        // content block
        assert_eq!(result["content"].as_array().unwrap().len(), 1);
        assert_eq!(result["content"][0]["type"], "text");
        assert_eq!(result["content"][0]["text"], "你好！");

        // stop_reason
        assert_eq!(result["stop_reason"], "end_turn");
        assert_eq!(result["stop_sequence"], Value::Null);

        // usage
        assert_eq!(result["usage"]["input_tokens"], 10);
        assert_eq!(result["usage"]["output_tokens"], 3);
    }

    /// RESP-01：content 为 null + refusal 字段
    #[test]
    fn test_content_null_with_refusal() {
        let input = json!({
            "id": "chatcmpl-refusal",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "refusal": "我无法回答这个问题"
                },
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 8}
        });

        let result = openai_to_anthropic(input).unwrap();
        assert_eq!(result["content"].as_array().unwrap().len(), 1);
        assert_eq!(result["content"][0]["type"], "text");
        assert_eq!(result["content"][0]["text"], "我无法回答这个问题");
    }

    /// RESP-01：content 为空字符串 => 空 content 数组
    #[test]
    fn test_empty_string_content_gives_empty_array() {
        let input = json!({
            "id": "chatcmpl-empty",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": ""},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 0}
        });

        let result = openai_to_anthropic(input).unwrap();
        assert_eq!(result["content"].as_array().unwrap().len(), 0);
    }

    /// RESP-01：model 字段透传
    #[test]
    fn test_model_passthrough() {
        let input = json!({
            "id": "chatcmpl-model",
            "model": "gpt-4-turbo",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "test"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        });

        let result = openai_to_anthropic(input).unwrap();
        assert_eq!(result["model"], "gpt-4-turbo");
    }

    /// RESP-02：工具调用响应 — arguments 正确反序列化为 JSON 对象
    #[test]
    fn test_tool_call_response() {
        let input = json!({
            "id": "chatcmpl-tool",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"Tokyo\", \"unit\": \"celsius\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 20, "completion_tokens": 10}
        });

        let result = openai_to_anthropic(input).unwrap();
        let content = result["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "tool_use");
        assert_eq!(content[0]["id"], "call_abc");
        assert_eq!(content[0]["name"], "get_weather");
        // input 是 JSON 对象，不是字符串
        assert_eq!(content[0]["input"]["location"], "Tokyo");
        assert_eq!(content[0]["input"]["unit"], "celsius");
        assert_eq!(result["stop_reason"], "tool_use");
    }

    /// RESP-02：arguments 反序列化失败 => 包装为 {"raw": "原字符串"}
    #[test]
    fn test_tool_call_invalid_arguments_wrapped() {
        let input = json!({
            "id": "chatcmpl-badjson",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_bad",
                        "type": "function",
                        "function": {
                            "name": "broken_tool",
                            "arguments": "not valid json {"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 5}
        });

        let result = openai_to_anthropic(input).unwrap();
        let content = result["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "tool_use");
        // 反序列化失败，包装为 {"raw": ...}
        assert!(content[0]["input"].get("raw").is_some());
        assert_eq!(content[0]["input"]["raw"], "not valid json {");
    }

    /// RESP-02：多个工具调用
    #[test]
    fn test_multiple_tool_calls() {
        let input = json!({
            "id": "chatcmpl-multi",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {
                            "id": "call_1",
                            "type": "function",
                            "function": {"name": "func_a", "arguments": "{\"x\": 1}"}
                        },
                        {
                            "id": "call_2",
                            "type": "function",
                            "function": {"name": "func_b", "arguments": "{\"y\": 2}"}
                        }
                    ]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 20}
        });

        let result = openai_to_anthropic(input).unwrap();
        let content = result["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["id"], "call_1");
        assert_eq!(content[1]["id"], "call_2");
    }

    /// RESP-02：tool_calls 和 content 同时存在 => text block + tool_use blocks
    #[test]
    fn test_mixed_content_and_tool_calls() {
        let input = json!({
            "id": "chatcmpl-mixed",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "我来帮你查询天气。",
                    "tool_calls": [{
                        "id": "call_weather",
                        "type": "function",
                        "function": {"name": "get_weather", "arguments": "{\"city\": \"北京\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 15, "completion_tokens": 25}
        });

        let result = openai_to_anthropic(input).unwrap();
        let content = result["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        // 第一个是 text block
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "我来帮你查询天气。");
        // 第二个是 tool_use block
        assert_eq!(content[1]["type"], "tool_use");
        assert_eq!(content[1]["name"], "get_weather");
    }

    /// RESP-03：finish_reason 穷举映射
    #[test]
    fn test_finish_reason_mapping_stop() {
        let input = json!({
            "id": "chatcmpl-stop",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        });
        assert_eq!(openai_to_anthropic(input).unwrap()["stop_reason"], "end_turn");
    }

    #[test]
    fn test_finish_reason_mapping_length() {
        let input = json!({
            "id": "chatcmpl-len",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "..."}, "finish_reason": "length"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 100}
        });
        assert_eq!(openai_to_anthropic(input).unwrap()["stop_reason"], "max_tokens");
    }

    #[test]
    fn test_finish_reason_mapping_content_filter() {
        let input = json!({
            "id": "chatcmpl-filter",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "blocked"}, "finish_reason": "content_filter"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        });
        assert_eq!(openai_to_anthropic(input).unwrap()["stop_reason"], "end_turn");
    }

    #[test]
    fn test_finish_reason_mapping_unknown_defaults_to_end_turn() {
        let input = json!({
            "id": "chatcmpl-unk",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "some_future_value"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        });
        assert_eq!(openai_to_anthropic(input).unwrap()["stop_reason"], "end_turn");
    }

    /// RESP-04：usage 字段重命名 + cache token 映射
    #[test]
    fn test_usage_rename() {
        let input = json!({
            "id": "chatcmpl-usage",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150}
        });

        let result = openai_to_anthropic(input).unwrap();
        assert_eq!(result["usage"]["input_tokens"], 100);
        assert_eq!(result["usage"]["output_tokens"], 50);
        // 原字段不应出现
        assert!(result["usage"].get("prompt_tokens").is_none());
        assert!(result["usage"].get("completion_tokens").is_none());
    }

    #[test]
    fn test_usage_cache_tokens_from_prompt_tokens_details() {
        let input = json!({
            "id": "chatcmpl-cache",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "prompt_tokens_details": {"cached_tokens": 80}
            }
        });

        let result = openai_to_anthropic(input).unwrap();
        assert_eq!(result["usage"]["cache_read_input_tokens"], 80);
    }

    #[test]
    fn test_usage_direct_cache_fields() {
        let input = json!({
            "id": "chatcmpl-directcache",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "cache_read_input_tokens": 60,
                "cache_creation_input_tokens": 20
            }
        });

        let result = openai_to_anthropic(input).unwrap();
        assert_eq!(result["usage"]["cache_read_input_tokens"], 60);
        assert_eq!(result["usage"]["cache_creation_input_tokens"], 20);
    }

    #[test]
    fn test_usage_null_gives_defaults() {
        // usage 为 null 时使用默认值 0
        let input = json!({
            "id": "chatcmpl-nousage",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}]
        });

        let result = openai_to_anthropic(input).unwrap();
        assert_eq!(result["usage"]["input_tokens"], 0);
        assert_eq!(result["usage"]["output_tokens"], 0);
    }

    /// id 前缀处理
    #[test]
    fn test_id_with_msg_prefix_unchanged() {
        let input = json!({
            "id": "msg_already_prefixed",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        });

        let result = openai_to_anthropic(input).unwrap();
        // 已有 msg_ 前缀，不重复添加
        assert_eq!(result["id"], "msg_already_prefixed");
    }

    #[test]
    fn test_id_without_msg_prefix_gets_prefixed() {
        let input = json!({
            "id": "chatcmpl-xyz",
            "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        });

        let result = openai_to_anthropic(input).unwrap();
        assert_eq!(result["id"], "msg_chatcmpl-xyz");
    }

    /// 错误情况：缺少 choices 字段
    #[test]
    fn test_missing_choices_returns_error() {
        let input = json!({
            "id": "chatcmpl-err",
            "model": "gpt-4o",
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        });

        assert!(openai_to_anthropic(input).is_err());
    }

    /// 错误情况：choices 数组为空
    #[test]
    fn test_empty_choices_returns_error() {
        let input = json!({
            "id": "chatcmpl-err",
            "model": "gpt-4o",
            "choices": [],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        });

        assert!(openai_to_anthropic(input).is_err());
    }

    /// 整体输出格式验证
    #[test]
    fn test_output_format_has_required_fields() {
        let input = json!({
            "id": "chatcmpl-format",
            "model": "gpt-4o-mini",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 1}
        });

        let result = openai_to_anthropic(input).unwrap();

        // 必须包含所有 Anthropic 响应字段
        assert!(result.get("id").is_some());
        assert_eq!(result["type"], "message");
        assert_eq!(result["role"], "assistant");
        assert!(result.get("model").is_some());
        assert!(result.get("content").is_some());
        assert!(result.get("stop_reason").is_some());
        assert_eq!(result["stop_sequence"], Value::Null);
        assert!(result.get("usage").is_some());
    }
}
