//! 请求转换：Anthropic Messages API 请求 → OpenAI Chat Completions API 请求
//!
//! 公开函数：
//! - `anthropic_to_openai()` — 主转换函数
//! - `build_proxy_endpoint_url()` — 转换请求的端点 URL 重写
//! - `build_upstream_url()` — 统一的上游 URL 组装
//! - `clean_schema()` — 递归清理 JSON Schema 不兼容字段

use crate::proxy::error::ProxyError;
use serde_json::{json, Value};

/// 将 Anthropic Messages API 请求体转换为 OpenAI Chat Completions API 请求体
///
/// 纯函数，不依赖任何外部状态。
pub fn anthropic_to_openai(body: Value) -> Result<Value, ProxyError> {
    let mut result = json!({});

    // model 字段原样透传（模型映射由 handler 层处理，Phase 15）
    if let Some(model) = body.get("model").and_then(|m| m.as_str()) {
        result["model"] = json!(model);
    }

    let mut messages: Vec<Value> = Vec::new();

    // 处理 system prompt（字符串或数组两种格式）
    if let Some(system) = body.get("system") {
        if let Some(text) = system.as_str() {
            // 单字符串格式 → 一条 system message
            messages.push(json!({"role": "system", "content": text}));
        } else if let Some(arr) = system.as_array() {
            // 数组格式 → 多条 system message，保留 cache_control
            for block in arr {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    let mut sys_msg = json!({"role": "system", "content": text});
                    if let Some(cc) = block.get("cache_control") {
                        sys_msg["cache_control"] = cc.clone();
                    }
                    messages.push(sys_msg);
                }
            }
        }
    }

    // 转换 messages 数组
    if let Some(msgs) = body.get("messages").and_then(|m| m.as_array()) {
        for msg in msgs {
            let converted = convert_message_to_openai(msg);
            messages.extend(converted);
        }
    }

    result["messages"] = json!(messages);

    // 参数映射
    if let Some(v) = body.get("max_tokens") {
        result["max_tokens"] = v.clone();
    }
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
        // 流式请求需要 stream_options.include_usage = true 才能在最终 chunk 收到 usage 数据
        if v.as_bool() == Some(true) {
            result["stream_options"] = json!({"include_usage": true});
        }
    }

    // 工具定义转换（过滤 BatchTool）
    if let Some(tools) = body.get("tools").and_then(|t| t.as_array()) {
        let openai_tools: Vec<Value> = tools
            .iter()
            .filter(|t| {
                // 过滤 BatchTool 类型
                t.get("type").and_then(|v| v.as_str()) != Some("BatchTool")
            })
            .map(|t| {
                let parameters = clean_schema(t.get("input_schema").cloned().unwrap_or(json!({})));
                let mut tool = json!({
                    "type": "function",
                    "function": {
                        "name": t.get("name").and_then(|n| n.as_str()).unwrap_or(""),
                        "description": t.get("description"),
                        "parameters": parameters,
                    }
                });
                // cache_control 透传
                if let Some(cc) = t.get("cache_control") {
                    tool["cache_control"] = cc.clone();
                }
                tool
            })
            .collect();

        if !openai_tools.is_empty() {
            result["tools"] = json!(openai_tools);
        }
    }

    if let Some(v) = body.get("tool_choice") {
        result["tool_choice"] = v.clone();
    }

    Ok(result)
}

/// 将单条 Anthropic message 转换为一或多条 OpenAI messages
fn convert_message_to_openai(msg: &Value) -> Vec<Value> {
    let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
    let content = match msg.get("content") {
        Some(c) => c,
        None => return vec![json!({"role": role, "content": null})],
    };

    // 字符串内容 → 直接转换
    if let Some(text) = content.as_str() {
        return vec![json!({"role": role, "content": text})];
    }

    // 数组内容（多模态 / 工具调用）
    if let Some(blocks) = content.as_array() {
        let mut result_msgs: Vec<Value> = Vec::new();
        let mut text_parts: Vec<String> = Vec::new();
        let mut content_parts: Vec<Value> = Vec::new();
        let mut tool_calls: Vec<Value> = Vec::new();
        let mut has_cache_control = false;

        for block in blocks {
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match block_type {
                "text" => {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        let cc = block.get("cache_control");
                        if cc.is_some() {
                            has_cache_control = true;
                            let mut part = json!({"type": "text", "text": text});
                            part["cache_control"] = cc.unwrap().clone();
                            content_parts.push(part);
                        } else {
                            text_parts.push(text.to_string());
                        }
                    }
                }
                "image" => {
                    if let Some(source) = block.get("source") {
                        let media_type = source
                            .get("media_type")
                            .and_then(|m| m.as_str())
                            .unwrap_or("image/png");
                        let data = source.get("data").and_then(|d| d.as_str()).unwrap_or("");
                        content_parts.push(json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", media_type, data)
                            }
                        }));
                    }
                }
                "tool_use" => {
                    let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let input = block.get("input").cloned().unwrap_or(json!({}));
                    tool_calls.push(json!({
                        "id": id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": serde_json::to_string(&input).unwrap_or_default(),
                        }
                    }));
                }
                "tool_result" => {
                    // tool_result → 独立 tool role 消息
                    let tool_use_id = block
                        .get("tool_use_id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("");
                    let content_val = block.get("content");
                    let content_str = match content_val {
                        Some(Value::String(s)) => s.clone(),
                        Some(v) => serde_json::to_string(v).unwrap_or_default(),
                        None => String::new(),
                    };
                    result_msgs.push(json!({
                        "role": "tool",
                        "tool_call_id": tool_use_id,
                        "content": content_str,
                    }));
                }
                "thinking" | _ => {
                    // thinking blocks 和未知 block type 静默丢弃
                }
            }
        }

        // 将 text_parts 合并后加入 content_parts（用换行连接多段文本）
        if !text_parts.is_empty() {
            if has_cache_control {
                // 有 cache_control 时保持数组格式，text_parts 独立加入
                for t in &text_parts {
                    content_parts.insert(0, json!({"type": "text", "text": t}));
                }
            } else {
                // 无 cache_control：多段 text 合并为字符串
                let combined = text_parts.join("\n");
                content_parts.insert(0, json!({"type": "text", "text": combined}));
            }
        }

        // 组装主消息
        if !content_parts.is_empty() || !tool_calls.is_empty() || !result_msgs.is_empty() {
            if !content_parts.is_empty() || !tool_calls.is_empty() {
                let mut msg_obj = json!({"role": role});

                if content_parts.is_empty() {
                    msg_obj["content"] = Value::Null;
                } else if content_parts.len() == 1 && !has_cache_control {
                    // 单条纯文本，展开为字符串
                    if let Some(text) = content_parts[0].get("text") {
                        msg_obj["content"] = text.clone();
                    } else {
                        msg_obj["content"] = json!(content_parts);
                    }
                } else {
                    msg_obj["content"] = json!(content_parts);
                }

                if !tool_calls.is_empty() {
                    msg_obj["tool_calls"] = json!(tool_calls);
                }

                result_msgs.insert(0, msg_obj);
            }

            return result_msgs;
        }
    }

    // 其他情况原样透传
    vec![json!({"role": role, "content": content})]
}

/// 统一组装上游 URL。
///
/// 规则：
/// - `target_path` 以 `/v1` 开头时：视为完整 API 路径，避免与已有 `/v1` 重复
/// - `target_path` 不以 `/v1` 开头时：视为 `/v1` 之下的端点后缀
/// - `base_url` 含 `/v1` 时，保留其前缀并替换其后的路径
/// - 自动处理尾部斜杠，并保留 query
pub fn build_upstream_url(base_url: &str, target_path: &str, query: &str) -> String {
    let base = base_url.trim_end_matches('/');

    // 查找 /v1 在 URL 中的位置（忽略协议部分）
    // 跳过 scheme（https://）再搜索
    let scheme_end = base.find("://").map(|i| i + 3).unwrap_or(0);
    let path_part = &base[scheme_end..];

    if let Some(v1_pos) = path_part.find("/v1") {
        if target_path.starts_with("/v1") {
            let prefix = &base[..scheme_end + v1_pos];
            format!("{}{}{}", prefix, target_path, query)
        } else {
            let absolute_v1_end = scheme_end + v1_pos + 3; // 指向 /v1 结束后
            let prefix = &base[..absolute_v1_end];
            format!("{}{}{}", prefix, target_path, query)
        }
    } else if target_path.starts_with("/v1") {
        format!("{}{}{}", base, target_path, query)
    } else {
        format!("{}/v1{}{}", base, target_path, query)
    }
}

/// 重写转换请求的端点 URL：将 /v1/messages 替换为目标 OpenAI 端点。
pub fn build_proxy_endpoint_url(base_url: &str, endpoint_suffix: &str) -> String {
    build_upstream_url(base_url, endpoint_suffix, "")
}

/// 递归清理 JSON Schema，移除 OpenAI 不兼容的字段
///
/// 移除字段：`format`、`default`
pub fn clean_schema(mut schema: Value) -> Value {
    if let Some(obj) = schema.as_object_mut() {
        // 移除顶层不兼容字段
        obj.remove("format");
        obj.remove("default");

        // 递归清理 properties 中的每个子 schema
        if let Some(props) = obj.get_mut("properties") {
            if let Some(props_obj) = props.as_object_mut() {
                for val in props_obj.values_mut() {
                    *val = clean_schema(val.clone());
                }
            }
        }

        // 递归清理 items
        if let Some(items) = obj.get_mut("items") {
            *items = clean_schema(items.clone());
        }
    }
    schema
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== build_proxy_endpoint_url 测试 ==========

    #[test]
    fn test_build_url_no_v1_in_base() {
        // base_url 不含 /v1 → 追加 /v1/chat/completions
        let result = build_proxy_endpoint_url("https://api.openai.com", "/chat/completions");
        assert_eq!(result, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn test_build_url_with_v1_in_base() {
        // base_url 已含 /v1 → 不重复
        let result = build_proxy_endpoint_url("https://openrouter.ai/api/v1", "/chat/completions");
        assert_eq!(result, "https://openrouter.ai/api/v1/chat/completions");
    }

    #[test]
    fn test_build_url_with_trailing_slash() {
        // 尾部斜杠应被去除
        let result = build_proxy_endpoint_url("https://openrouter.ai/api/v1/", "/chat/completions");
        assert_eq!(result, "https://openrouter.ai/api/v1/chat/completions");
    }

    #[test]
    fn test_build_url_replaces_path_after_v1() {
        // /v1 之后的旧路径被 endpoint_suffix 替换
        let result =
            build_proxy_endpoint_url("https://example.com/v1/responses", "/chat/completions");
        assert_eq!(result, "https://example.com/v1/chat/completions");
    }

    #[test]
    fn test_build_upstream_url_passthrough_reuses_existing_v1_prefix() {
        let result = build_upstream_url(
            "https://gateway.example.com/openai/v1/chat/completions",
            "/v1/token_count",
            "?beta=true",
        );
        assert_eq!(
            result,
            "https://gateway.example.com/openai/v1/token_count?beta=true"
        );
    }

    #[test]
    fn test_build_upstream_url_passthrough_appends_v1_when_missing() {
        let result = build_upstream_url(
            "https://gateway.example.com/openai",
            "/v1/models",
            "?limit=1",
        );
        assert_eq!(
            result,
            "https://gateway.example.com/openai/v1/models?limit=1"
        );
    }

    // ========== clean_schema 测试 ==========

    #[test]
    fn test_clean_schema_removes_top_level_format() {
        let schema = json!({"type": "string", "format": "uri"});
        let cleaned = clean_schema(schema);
        assert!(cleaned.get("format").is_none(), "format 应被移除");
        assert_eq!(cleaned["type"], "string");
    }

    #[test]
    fn test_clean_schema_recursive_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "format": "uri"},
                "name": {"type": "string"}
            }
        });
        let cleaned = clean_schema(schema);
        assert!(
            cleaned["properties"]["url"].get("format").is_none(),
            "嵌套 format 应被移除"
        );
        assert_eq!(cleaned["properties"]["name"]["type"], "string");
    }

    #[test]
    fn test_clean_schema_recursive_items() {
        let schema = json!({
            "type": "array",
            "items": {"type": "string", "format": "date-time"}
        });
        let cleaned = clean_schema(schema);
        assert!(
            cleaned["items"].get("format").is_none(),
            "items 内 format 应被移除"
        );
    }

    #[test]
    fn test_clean_schema_no_format_unchanged() {
        let schema = json!({"type": "object", "properties": {"x": {"type": "integer"}}});
        let cleaned = clean_schema(schema.clone());
        assert_eq!(cleaned["properties"]["x"]["type"], "integer");
    }

    #[test]
    fn test_clean_schema_removes_default() {
        let schema = json!({"type": "string", "default": "hello"});
        let cleaned = clean_schema(schema);
        assert!(cleaned.get("default").is_none(), "default 应被移除");
    }

    // ========== anthropic_to_openai 测试 ==========

    #[test]
    fn test_system_string_becomes_first_message() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "system": "You are helpful.",
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert_eq!(result["messages"][0]["role"], "system");
        assert_eq!(result["messages"][0]["content"], "You are helpful.");
        assert_eq!(result["messages"][1]["role"], "user");
    }

    #[test]
    fn test_system_array_becomes_multiple_system_messages() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "system": [
                {"type": "text", "text": "Part one"},
                {"type": "text", "text": "Part two"}
            ],
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert_eq!(result["messages"][0]["role"], "system");
        assert_eq!(result["messages"][0]["content"], "Part one");
        assert_eq!(result["messages"][1]["role"], "system");
        assert_eq!(result["messages"][1]["content"], "Part two");
    }

    #[test]
    fn test_text_content_block() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert_eq!(result["messages"][0]["content"], "Hello");
    }

    #[test]
    fn test_multiple_text_blocks_merged_with_newline() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "Line one"},
                    {"type": "text", "text": "Line two"}
                ]
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        let content = result["messages"][0]["content"].as_str().unwrap();
        assert!(content.contains("Line one"), "应包含 Line one");
        assert!(content.contains("Line two"), "应包含 Line two");
    }

    #[test]
    fn test_tool_use_block_becomes_tool_calls() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "tool_use", "id": "call_abc", "name": "get_weather", "input": {"city": "Tokyo"}}
                ]
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        let msg = &result["messages"][0];
        assert_eq!(msg["role"], "assistant");
        assert!(msg.get("tool_calls").is_some());
        assert_eq!(msg["tool_calls"][0]["id"], "call_abc");
        assert_eq!(msg["tool_calls"][0]["type"], "function");
        assert_eq!(msg["tool_calls"][0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_tool_result_becomes_tool_role_message() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "tool_result", "tool_use_id": "call_abc", "content": "Sunny, 25°C"}
                ]
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        let msg = &result["messages"][0];
        assert_eq!(msg["role"], "tool");
        assert_eq!(msg["tool_call_id"], "call_abc");
        assert_eq!(msg["content"], "Sunny, 25°C");
    }

    #[test]
    fn test_tool_result_content_array_serialized_to_string() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "call_abc",
                        "content": [{"type": "text", "text": "result data"}]
                    }
                ]
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        let msg = &result["messages"][0];
        // content 数组应被序列化为字符串
        let content_str = msg["content"].as_str().expect("content 应为字符串");
        assert!(content_str.contains("result data"));
    }

    #[test]
    fn test_image_block_becomes_image_url() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": "abc123"
                    }
                }]
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        let part = &result["messages"][0]["content"][0];
        assert_eq!(part["type"], "image_url");
        assert_eq!(part["image_url"]["url"], "data:image/jpeg;base64,abc123");
    }

    #[test]
    fn test_tools_converted_with_input_schema() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [{
                "name": "search",
                "description": "Search the web",
                "input_schema": {
                    "type": "object",
                    "properties": {"query": {"type": "string"}},
                    "required": ["query"]
                }
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert_eq!(result["tools"][0]["type"], "function");
        assert_eq!(result["tools"][0]["function"]["name"], "search");
        assert_eq!(
            result["tools"][0]["function"]["parameters"]["type"],
            "object"
        );
    }

    #[test]
    fn test_batch_tool_filtered_out() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [
                {
                    "name": "normal_tool",
                    "description": "A normal tool",
                    "input_schema": {"type": "object", "properties": {}}
                },
                {
                    "type": "BatchTool",
                    "name": "batch_tool",
                    "description": "Should be filtered",
                    "input_schema": {"type": "object", "properties": {}}
                }
            ]
        });
        let result = anthropic_to_openai(body).unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1, "BatchTool 应被过滤");
        assert_eq!(tools[0]["function"]["name"], "normal_tool");
    }

    #[test]
    fn test_clean_schema_applied_to_tool_parameters() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [{
                "name": "url_fetcher",
                "description": "Fetch URL",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "format": "uri"}
                    }
                }
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        let params = &result["tools"][0]["function"]["parameters"];
        assert!(
            params["properties"]["url"].get("format").is_none(),
            "format 应被 clean_schema 移除"
        );
    }

    #[test]
    fn test_cache_control_preserved_in_system() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "system": [
                {"type": "text", "text": "System", "cache_control": {"type": "ephemeral"}}
            ],
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert_eq!(result["messages"][0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_cache_control_preserved_in_text_block() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "Hello", "cache_control": {"type": "ephemeral"}}
                ]
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        // 带 cache_control 的 text block 保持数组格式
        let content = &result["messages"][0]["content"];
        assert!(content.is_array(), "带 cache_control 应保持数组格式");
        assert_eq!(content[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_cache_control_preserved_in_tool() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [{
                "name": "search",
                "description": "Search",
                "input_schema": {"type": "object"},
                "cache_control": {"type": "ephemeral"}
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert_eq!(result["tools"][0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_stop_sequences_mapped_to_stop() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "stop_sequences": ["<stop>", "</end>"],
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert_eq!(result["stop"][0], "<stop>");
        assert_eq!(result["stop"][1], "</end>");
    }

    #[test]
    fn test_params_passthrough() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 2048,
            "temperature": 0.7,
            "top_p": 0.9,
            "stream": true,
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert_eq!(result["max_tokens"], 2048);
        assert_eq!(result["temperature"], 0.7);
        assert_eq!(result["top_p"], 0.9);
        assert_eq!(result["stream"], true);
        // 流式请求应自动注入 stream_options.include_usage = true
        assert_eq!(result["stream_options"]["include_usage"], true);
    }

    #[test]
    fn test_stream_false_no_stream_options() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "stream": false,
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert_eq!(result["stream"], false);
        // 非流式请求不应注入 stream_options
        assert!(result.get("stream_options").is_none(), "stream=false 时不应有 stream_options");
    }

    #[test]
    fn test_thinking_blocks_silently_discarded() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "Let me think..."},
                    {"type": "text", "text": "Answer"}
                ]
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        let msg = &result["messages"][0];
        // thinking block 丢弃，只剩 text
        let content = msg["content"].as_str().unwrap_or("");
        assert_eq!(content, "Answer");
    }

    #[test]
    fn test_unknown_content_block_silently_discarded() {
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "unknown_future_type", "data": "xyz"},
                    {"type": "text", "text": "Real content"}
                ]
            }]
        });
        let result = anthropic_to_openai(body).unwrap();
        let content = result["messages"][0]["content"].as_str().unwrap_or("");
        assert_eq!(content, "Real content");
    }

    #[test]
    fn test_model_passthrough() {
        let body = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert_eq!(result["model"], "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_all_only_batch_tools_no_tools_field() {
        // 如果全部工具都是 BatchTool，结果中不应有 tools 字段
        let body = json!({
            "model": "claude-3-opus",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
            "tools": [
                {
                    "type": "BatchTool",
                    "name": "batch",
                    "input_schema": {"type": "object"}
                }
            ]
        });
        let result = anthropic_to_openai(body).unwrap();
        assert!(
            result.get("tools").is_none(),
            "全部为 BatchTool 时不应有 tools 字段"
        );
    }
}
