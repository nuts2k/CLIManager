//! 请求转换：Anthropic Messages API 请求 → OpenAI Responses API 请求
//!
//! 公开函数：
//! - `anthropic_to_responses()` — 主转换函数
//!
//! 差异说明（与 Chat Completions 的主要差异）：
//! - messages → input（Responses API 使用 input 字段）
//! - max_tokens → max_output_tokens（字段名不同）
//! - system（字符串或数组）→ instructions（单字符串，数组时多段拼接）
//! - 工具定义无 function 包装层，直接放 name/description/parameters
//! - tool_result → function_call_output 独立 input 项
//! - tool_use → function_call 独立 input 项

use crate::proxy::error::ProxyError;
use serde_json::{json, Value};

/// 将 Anthropic Messages API 请求体转换为 OpenAI Responses API 请求体
///
/// 纯函数，不依赖任何外部状态。
pub fn anthropic_to_responses(body: Value) -> Result<Value, ProxyError> {
    let mut result = json!({});

    // model 字段原样透传（模型映射由 handler 层处理）
    if let Some(model) = body.get("model").and_then(|m| m.as_str()) {
        result["model"] = json!(model);
    }

    // max_tokens → max_output_tokens（Responses API 字段名不同）
    if let Some(v) = body.get("max_tokens") {
        result["max_output_tokens"] = v.clone();
    }

    // system（字符串或数组）→ instructions（字符串）
    if let Some(system) = body.get("system") {
        let instructions = extract_system_text(system);
        if !instructions.is_empty() {
            result["instructions"] = json!(instructions);
        }
    }

    // 参数映射
    if let Some(v) = body.get("temperature") {
        result["temperature"] = v.clone();
    }
    if let Some(v) = body.get("top_p") {
        result["top_p"] = v.clone();
    }
    if let Some(v) = body.get("stop_sequences") {
        result["stop"] = v.clone();
    }
    if let Some(v) = body.get("stream") {
        result["stream"] = v.clone();
    }

    // 工具定义转换（无 function 包装层）
    if let Some(tools) = body.get("tools").and_then(|t| t.as_array()) {
        let responses_tools: Vec<Value> = convert_tool_definitions(tools);
        if !responses_tools.is_empty() {
            result["tools"] = json!(responses_tools);
        }
    }

    if let Some(v) = body.get("tool_choice") {
        result["tool_choice"] = v.clone();
    }

    // messages → input
    if let Some(msgs) = body.get("messages").and_then(|m| m.as_array()) {
        let input = convert_messages_to_input(msgs);
        result["input"] = json!(input);
    }

    Ok(result)
}

/// 将 system 字段（字符串或数组）提取为单个字符串
///
/// - 字符串：直接返回
/// - 数组：提取各 text block 的 text 字段，用换行连接
fn extract_system_text(system: &Value) -> String {
    if let Some(text) = system.as_str() {
        return text.to_string();
    }
    if let Some(arr) = system.as_array() {
        let parts: Vec<&str> = arr
            .iter()
            .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
            .collect();
        return parts.join("\n");
    }
    String::new()
}

/// 将 Anthropic messages 数组转换为 Responses API input 数组
///
/// 关键差异：
/// - tool_use block → 独立的 {type:"function_call"} input 项
/// - tool_result block → 独立的 {type:"function_call_output"} input 项
/// - thinking blocks 静默丢弃
/// - 普通文本消息：role/content 结构，content 简化为字符串
fn convert_messages_to_input(messages: &[Value]) -> Vec<Value> {
    let mut input: Vec<Value> = Vec::new();

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
        let content = match msg.get("content") {
            Some(c) => c,
            None => continue,
        };

        // 字符串内容：直接转为 input 项
        if let Some(text) = content.as_str() {
            input.push(json!({"role": role, "content": text}));
            continue;
        }

        // 数组内容：逐 block 分路处理
        if let Some(blocks) = content.as_array() {
            let mut text_parts: Vec<String> = Vec::new();
            let mut image_parts: Vec<Value> = Vec::new();
            let mut has_complex = false;

            for block in blocks {
                let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

                match block_type {
                    "text" => {
                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                            text_parts.push(text.to_string());
                        }
                    }
                    "image" => {
                        has_complex = true;
                        if let Some(source) = block.get("source") {
                            let media_type = source
                                .get("media_type")
                                .and_then(|m| m.as_str())
                                .unwrap_or("image/png");
                            let data =
                                source.get("data").and_then(|d| d.as_str()).unwrap_or("");
                            image_parts.push(json!({
                                "type": "input_image",
                                "image_url": format!("data:{};base64,{}", media_type, data)
                            }));
                        }
                    }
                    "tool_use" => {
                        // tool_use → 独立 function_call input 项（先推送已有文本）
                        if !text_parts.is_empty() {
                            let combined = text_parts.join("\n");
                            input.push(json!({"role": role, "content": combined}));
                            text_parts.clear();
                        }
                        if !image_parts.is_empty() {
                            for img in image_parts.drain(..) {
                                input.push(img);
                            }
                        }
                        let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let tool_input = block.get("input").cloned().unwrap_or(json!({}));
                        input.push(json!({
                            "type": "function_call",
                            "call_id": id,
                            "name": name,
                            "arguments": serde_json::to_string(&tool_input).unwrap_or_default()
                        }));
                        has_complex = true;
                    }
                    "tool_result" => {
                        // tool_result → 独立 function_call_output input 项
                        let tool_use_id = block
                            .get("tool_use_id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("");
                        let content_val = block.get("content");
                        let output_str = match content_val {
                            Some(Value::String(s)) => s.clone(),
                            Some(v) => serde_json::to_string(v).unwrap_or_default(),
                            None => String::new(),
                        };
                        input.push(json!({
                            "type": "function_call_output",
                            "call_id": tool_use_id,
                            "output": output_str
                        }));
                        has_complex = true;
                    }
                    "thinking" | _ => {
                        // thinking blocks 和未知 block type 静默丢弃
                    }
                }
            }

            // 将剩余 text_parts 和 image_parts 组装为消息
            if !text_parts.is_empty() || !image_parts.is_empty() {
                if image_parts.is_empty() && !has_complex {
                    // 纯文本：合并为字符串
                    let combined = text_parts.join("\n");
                    input.push(json!({"role": role, "content": combined}));
                } else {
                    // 有图片：先推文本（如有），再推图片
                    if !text_parts.is_empty() {
                        let combined = text_parts.join("\n");
                        input.push(json!({"role": role, "content": combined}));
                    }
                    for img in image_parts {
                        input.push(img);
                    }
                }
            }

            continue;
        }

        // 其他情况（不应发生）
        input.push(json!({"role": role, "content": content}));
    }

    input
}

/// 将 Anthropic 工具定义数组转换为 Responses API 格式
///
/// 主要差异：
/// - Chat Completions: {type:"function", function:{name, description, parameters}}
/// - Responses API:    {type:"function", name, description, parameters}（无 function 包装层）
fn convert_tool_definitions(tools: &[Value]) -> Vec<Value> {
    tools
        .iter()
        .filter(|t| {
            // 过滤 BatchTool 类型
            t.get("type").and_then(|v| v.as_str()) != Some("BatchTool")
        })
        .map(|t| {
            let parameters =
                super::request::clean_schema(t.get("input_schema").cloned().unwrap_or(json!({})));
            let mut tool = json!({
                "type": "function",
                "name": t.get("name").and_then(|n| n.as_str()).unwrap_or(""),
                "parameters": parameters,
            });
            // description 透传（可选字段）
            if let Some(desc) = t.get("description") {
                tool["description"] = desc.clone();
            }
            // cache_control 透传
            if let Some(cc) = t.get("cache_control") {
                tool["cache_control"] = cc.clone();
            }
            tool
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// test 1: 基础文本请求转换
    #[test]
    fn test_basic_text_request() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "system": "You are helpful",
            "messages": [{"role": "user", "content": [{"type": "text", "text": "Hello"}]}]
        });
        let result = anthropic_to_responses(body).unwrap();
        assert_eq!(result["model"], "claude-3-5-sonnet");
        assert_eq!(result["max_output_tokens"], 1024);
        assert!(result.get("max_tokens").is_none(), "max_tokens 字段应不存在");
        assert_eq!(result["instructions"], "You are helpful");
        assert_eq!(result["input"][0]["role"], "user");
        assert_eq!(result["input"][0]["content"], "Hello");
    }

    /// test 2: system 数组格式转 instructions（多段拼接）
    #[test]
    fn test_system_array_format() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "system": [
                {"type": "text", "text": "Part1"},
                {"type": "text", "text": "Part2"}
            ],
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_responses(body).unwrap();
        assert_eq!(result["instructions"], "Part1\nPart2");
    }

    /// test 3: max_tokens → max_output_tokens
    #[test]
    fn test_max_tokens_mapping() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_responses(body).unwrap();
        assert_eq!(result["max_output_tokens"], 4096);
        assert!(result.get("max_tokens").is_none(), "max_tokens 字段不应存在");
    }

    /// test 4: 工具定义无 function 包装层
    #[test]
    fn test_tools_no_function_wrapper() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [{
                "name": "get_weather",
                "type": "custom",
                "input_schema": {
                    "type": "object",
                    "properties": {"location": {"type": "string"}}
                }
            }]
        });
        let result = anthropic_to_responses(body).unwrap();
        let tool = &result["tools"][0];
        // 无 function 包装层
        assert!(tool.get("function").is_none(), "不应有 function 包装层");
        assert_eq!(tool["type"], "function");
        assert_eq!(tool["name"], "get_weather");
        assert_eq!(tool["parameters"]["type"], "object");
        assert_eq!(tool["parameters"]["properties"]["location"]["type"], "string");
    }

    /// test 5: tool_result → function_call_output 独立 input 项
    #[test]
    fn test_tool_result_to_function_call_output() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "call_123",
                    "content": "result text"
                }]
            }]
        });
        let result = anthropic_to_responses(body).unwrap();
        let input = result["input"].as_array().unwrap();
        // 找到 function_call_output 项
        let fc_output = input
            .iter()
            .find(|item| item["type"] == "function_call_output")
            .expect("应存在 function_call_output 项");
        assert_eq!(fc_output["call_id"], "call_123");
        assert_eq!(fc_output["output"], "result text");
    }

    /// test 6: assistant 工具调用 → function_call 独立 input 项
    #[test]
    fn test_assistant_tool_use_to_function_call() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": "call_abc",
                    "name": "get_weather",
                    "input": {"location": "Tokyo"}
                }]
            }]
        });
        let result = anthropic_to_responses(body).unwrap();
        let input = result["input"].as_array().unwrap();
        let fc = input
            .iter()
            .find(|item| item["type"] == "function_call")
            .expect("应存在 function_call 项");
        assert_eq!(fc["call_id"], "call_abc");
        assert_eq!(fc["name"], "get_weather");
        // arguments 为 JSON 字符串
        let args: Value = serde_json::from_str(fc["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(args["location"], "Tokyo");
    }

    /// test 7: 多轮对话完整转换
    #[test]
    fn test_multi_turn_conversation() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "What's the weather?"},
                {"role": "assistant", "content": [{
                    "type": "tool_use",
                    "id": "call_001",
                    "name": "get_weather",
                    "input": {"city": "Beijing"}
                }]},
                {"role": "user", "content": [{
                    "type": "tool_result",
                    "tool_use_id": "call_001",
                    "content": "Sunny, 20°C"
                }]},
                {"role": "assistant", "content": "It is sunny and 20°C in Beijing."}
            ]
        });
        let result = anthropic_to_responses(body).unwrap();
        let input = result["input"].as_array().unwrap();

        // 验证顺序：user msg → function_call → function_call_output → assistant msg
        assert_eq!(input.len(), 4);
        assert_eq!(input[0]["role"], "user");
        assert_eq!(input[0]["content"], "What's the weather?");
        assert_eq!(input[1]["type"], "function_call");
        assert_eq!(input[1]["call_id"], "call_001");
        assert_eq!(input[2]["type"], "function_call_output");
        assert_eq!(input[2]["call_id"], "call_001");
        assert_eq!(input[3]["role"], "assistant");
        assert_eq!(input[3]["content"], "It is sunny and 20°C in Beijing.");
    }

    /// test 8: stream 标志透传
    #[test]
    fn test_stream_passthrough() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "stream": true,
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_responses(body).unwrap();
        assert_eq!(result["stream"], true);
    }

    /// test 9: temperature/top_p/stop_sequences 映射
    #[test]
    fn test_standard_params() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "temperature": 0.7,
            "top_p": 0.9,
            "stop_sequences": ["END"],
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_responses(body).unwrap();
        assert_eq!(result["temperature"], 0.7);
        assert_eq!(result["top_p"], 0.9);
        assert_eq!(result["stop"][0], "END");
    }

    /// test 10: thinking blocks 静默丢弃
    #[test]
    fn test_thinking_blocks_dropped() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "Let me think..."},
                    {"type": "text", "text": "Final answer"}
                ]
            }]
        });
        let result = anthropic_to_responses(body).unwrap();
        let input = result["input"].as_array().unwrap();
        assert_eq!(input.len(), 1, "thinking block 已丢弃，只有一项");
        assert_eq!(input[0]["content"], "Final answer");
    }

    /// test 11: 图片内容转换
    #[test]
    fn test_image_content_to_input_image() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": "abc123"
                    }
                }]
            }]
        });
        let result = anthropic_to_responses(body).unwrap();
        let input = result["input"].as_array().unwrap();
        let img_item = input
            .iter()
            .find(|item| item["type"] == "input_image")
            .expect("应存在 input_image 项");
        assert_eq!(img_item["image_url"], "data:image/png;base64,abc123");
    }

    /// test 12: JSON Schema 清理（clean_schema 调用验证）
    #[test]
    fn test_clean_schema_applied() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [{
                "name": "url_fetcher",
                "description": "Fetch URL",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "format": "uri", "default": "https://example.com"}
                    }
                }
            }]
        });
        let result = anthropic_to_responses(body).unwrap();
        let params = &result["tools"][0]["parameters"];
        assert!(
            params["properties"]["url"].get("format").is_none(),
            "format 应被 clean_schema 移除"
        );
        assert!(
            params["properties"]["url"].get("default").is_none(),
            "default 应被 clean_schema 移除"
        );
    }
}
