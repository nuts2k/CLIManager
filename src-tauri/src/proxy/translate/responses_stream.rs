//! 流式转换：OpenAI Responses API SSE 流 → Anthropic SSE 流
//!
//! 公开函数：
//! - `create_responses_anthropic_sse_stream()` — 将 Responses API SSE 事件流转换为 Anthropic SSE 事件序列

use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use serde_json::{json, Value};

// ── 辅助函数 ──

/// 格式化并返回 Anthropic SSE 事件字节：`event: {type}\ndata: {json}\n\n`
fn format_sse_event(event_type: &str, data: &Value) -> Bytes {
    let json_str = serde_json::to_string(data).unwrap_or_default();
    Bytes::from(format!("event: {event_type}\ndata: {json_str}\n\n"))
}

/// 返回完整 SSE 块分隔符 `\n\n` 的起始位置。
fn find_sse_block_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == b"\n\n")
}

/// 解析 Responses API SSE 块，返回 (event_type, data_json) 对
///
/// Responses API SSE 格式：`event: type\ndata: json\n\n`
fn parse_responses_sse_block(block: &str) -> Option<(String, Value)> {
    let mut event_type = None;
    let mut data_str = None;

    for line in block.lines() {
        if let Some(ev) = line.strip_prefix("event: ") {
            event_type = Some(ev.trim().to_string());
        } else if let Some(d) = line.strip_prefix("data: ") {
            data_str = Some(d);
        }
    }

    let event_type = event_type?;
    let data: Value = serde_json::from_str(data_str?).ok()?;
    Some((event_type, data))
}

// ── 状态结构 ──

/// 追踪单个 function_call output item 的流式状态
#[derive(Debug)]
struct FunctionCallState {
    /// 在 Anthropic 事件序列中的 content_block index
    anthropic_index: u32,
    /// 来自 output_item.added 的 call_id（保留用于调试/扩展）
    #[allow(dead_code)]
    call_id: String,
    /// 函数名称（保留用于调试/扩展）
    #[allow(dead_code)]
    name: String,
    /// 是否已发出 content_block_start（保留用于调试/扩展）
    #[allow(dead_code)]
    started: bool,
}

/// 将 OpenAI Responses API SSE 上游字节流转换为 Anthropic SSE 格式的异步流
///
/// # 参数
/// - `upstream`: OpenAI Responses API 格式的 SSE 字节流（reqwest body stream）
/// - `request_model`: 用于 message_start 事件中的 model 字段（调用方传入的 Anthropic 模型名）
///
/// # Responses API SSE 事件格式
/// Responses API 使用 `event: type\ndata: json\n\n` 格式，事件名在 event 字段。
///
/// # 文本响应流式事件序列
/// response.created → response.output_item.added(message) → response.content_part.added →
/// response.output_text.delta* → response.output_text.done → response.output_item.done →
/// response.completed
///
/// # 函数调用流式事件序列
/// response.output_item.added(function_call, call_id+name) →
/// response.function_call_arguments.delta* → response.function_call_arguments.done →
/// response.output_item.done → response.completed
pub fn create_responses_anthropic_sse_stream(
    upstream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
    request_model: String,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send {
    async_stream::stream! {
        // 跨 chunk SSE 字节缓冲
        let mut buffer = Vec::new();

        // message_start 状态
        let mut message_started = false;

        // content block 编号管理
        let mut next_anthropic_index: u32 = 0;

        // 当前文本 block 的 Anthropic index（None = 没有打开的文本 block）
        let mut current_text_index: Option<u32> = None;

        // 函数调用状态表（key: output_index, value: FunctionCallState）
        let mut function_calls: std::collections::HashMap<u64, FunctionCallState> = std::collections::HashMap::new();

        // 是否有 function_call 输出（用于推断 stop_reason）
        let mut has_function_call = false;

        tokio::pin!(upstream);

        'outer: while let Some(chunk) = upstream.next().await {
            match chunk {
                Err(e) => {
                    log::error!("[responses_stream] 上游流读取错误: {e}");
                    yield Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
                    break 'outer;
                }
                Ok(bytes) => {
                    buffer.extend_from_slice(&bytes);

                    // 消耗所有完整 SSE 块（以 \n\n 结尾）
                    while let Some(pos) = find_sse_block_end(&buffer) {
                        let block_bytes = buffer[..pos].to_vec();
                        buffer.drain(..pos + 2);
                        let block = String::from_utf8_lossy(&block_bytes).into_owned();

                        if block.trim().is_empty() {
                            continue;
                        }

                        let Some((event_type, data)) = parse_responses_sse_block(&block) else {
                            continue;
                        };

                        match event_type.as_str() {
                            // ── response.created → message_start ──
                            "response.created" => {
                                if !message_started {
                                    message_started = true;

                                    // 提取 response id，前缀替换 resp_ → msg_
                                    let raw_id = data
                                        .pointer("/response/id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    let msg_id = if raw_id.starts_with("resp_") {
                                        format!("msg_{}", &raw_id["resp_".len()..])
                                    } else {
                                        raw_id.to_string()
                                    };

                                    let start_event = json!({
                                        "type": "message_start",
                                        "message": {
                                            "id": msg_id,
                                            "type": "message",
                                            "role": "assistant",
                                            "model": request_model,
                                            "content": [],
                                            "stop_reason": null,
                                            "stop_sequence": null,
                                            "usage": {
                                                "input_tokens": 0,
                                                "output_tokens": 0
                                            }
                                        }
                                    });
                                    yield Ok(format_sse_event("message_start", &start_event));
                                }
                            }

                            // ── response.output_item.added ──
                            // message 类型：等待 content_part.added 发 content_block_start
                            // function_call 类型：立即发 content_block_start（无需 Deferred Start）
                            "response.output_item.added" => {
                                let output_index = data
                                    .get("output_index")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let item = &data["item"];
                                let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

                                if item_type == "function_call" {
                                    has_function_call = true;

                                    let call_id = item
                                        .get("call_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let name = item
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    let idx = next_anthropic_index;
                                    next_anthropic_index += 1;

                                    // 立即发 content_block_start（无 Deferred Start）
                                    let cbs_event = json!({
                                        "type": "content_block_start",
                                        "index": idx,
                                        "content_block": {
                                            "type": "tool_use",
                                            "id": call_id,
                                            "name": name
                                        }
                                    });
                                    yield Ok(format_sse_event("content_block_start", &cbs_event));

                                    function_calls.insert(
                                        output_index,
                                        FunctionCallState {
                                            anthropic_index: idx,
                                            call_id,
                                            name,
                                            started: true,
                                        },
                                    );
                                }
                                // message 类型不在此处处理，等待 content_part.added
                            }

                            // ── response.content_part.added → content_block_start(text) ──
                            "response.content_part.added" => {
                                let part_type = data
                                    .pointer("/part/type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");

                                if part_type == "output_text" && current_text_index.is_none() {
                                    let idx = next_anthropic_index;
                                    next_anthropic_index += 1;
                                    current_text_index = Some(idx);

                                    let cbs_event = json!({
                                        "type": "content_block_start",
                                        "index": idx,
                                        "content_block": {
                                            "type": "text",
                                            "text": ""
                                        }
                                    });
                                    yield Ok(format_sse_event("content_block_start", &cbs_event));
                                }
                            }

                            // ── response.output_text.delta → content_block_delta(text_delta) ──
                            "response.output_text.delta" => {
                                if let Some(idx) = current_text_index {
                                    let delta = data.get("delta").and_then(|v| v.as_str()).unwrap_or("");
                                    let cbd_event = json!({
                                        "type": "content_block_delta",
                                        "index": idx,
                                        "delta": {
                                            "type": "text_delta",
                                            "text": delta
                                        }
                                    });
                                    yield Ok(format_sse_event("content_block_delta", &cbd_event));
                                }
                            }

                            // ── response.output_text.done → content_block_stop ──
                            "response.output_text.done" => {
                                if let Some(idx) = current_text_index.take() {
                                    let cbs_stop = json!({
                                        "type": "content_block_stop",
                                        "index": idx
                                    });
                                    yield Ok(format_sse_event("content_block_stop", &cbs_stop));
                                }
                            }

                            // ── response.function_call_arguments.delta → content_block_delta(input_json_delta) ──
                            "response.function_call_arguments.delta" => {
                                let output_index = data
                                    .get("output_index")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                if let Some(state) = function_calls.get(&output_index) {
                                    let idx = state.anthropic_index;
                                    let delta = data.get("delta").and_then(|v| v.as_str()).unwrap_or("");
                                    let cbd_event = json!({
                                        "type": "content_block_delta",
                                        "index": idx,
                                        "delta": {
                                            "type": "input_json_delta",
                                            "partial_json": delta
                                        }
                                    });
                                    yield Ok(format_sse_event("content_block_delta", &cbd_event));
                                }
                            }

                            // ── response.function_call_arguments.done → content_block_stop ──
                            "response.function_call_arguments.done" => {
                                let output_index = data
                                    .get("output_index")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                if let Some(state) = function_calls.get(&output_index) {
                                    let idx = state.anthropic_index;
                                    let cbs_stop = json!({
                                        "type": "content_block_stop",
                                        "index": idx
                                    });
                                    yield Ok(format_sse_event("content_block_stop", &cbs_stop));
                                }
                            }

                            // ── response.completed → message_delta + message_stop ──
                            "response.completed" => {
                                {
                                    // 关闭尚未关闭的文本 block（异常情况保底）
                                    if let Some(idx) = current_text_index.take() {
                                        let cbs_stop = json!({
                                            "type": "content_block_stop",
                                            "index": idx
                                        });
                                        yield Ok(format_sse_event("content_block_stop", &cbs_stop));
                                    }

                                    // 推断 stop_reason
                                    let stop_reason = if has_function_call { "tool_use" } else { "end_turn" };

                                    // 提取 usage（Responses API 命名与 Anthropic 相同）
                                    let input_tokens = data
                                        .pointer("/response/usage/input_tokens")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0);
                                    let output_tokens = data
                                        .pointer("/response/usage/output_tokens")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0);
                                    let usage_val = json!({
                                        "input_tokens": input_tokens,
                                        "output_tokens": output_tokens
                                    });

                                    // message_delta
                                    let msg_delta = json!({
                                        "type": "message_delta",
                                        "delta": {
                                            "stop_reason": stop_reason,
                                            "stop_sequence": null
                                        },
                                        "usage": usage_val
                                    });
                                    yield Ok(format_sse_event("message_delta", &msg_delta));

                                    // message_stop
                                    let msg_stop = json!({"type": "message_stop"});
                                    yield Ok(format_sse_event("message_stop", &msg_stop));

                                    break 'outer;
                                }
                            }

                            // 其他事件（output_item.done 等）跳过
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures::stream;
    use futures::StreamExt;
    use serde_json::Value;

    use super::create_responses_anthropic_sse_stream;

    // ── 测试辅助函数 ──

    /// 收集流式事件为 JSON 列表
    async fn collect_events(
        upstream: impl futures::Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
        model: &str,
    ) -> Vec<Value> {
        let out_stream = create_responses_anthropic_sse_stream(upstream, model.to_string());
        let chunks: Vec<_> = out_stream.collect().await;
        let merged: String = chunks
            .into_iter()
            .map(|r: Result<Bytes, _>| String::from_utf8_lossy(r.unwrap().as_ref()).to_string())
            .collect();
        merged
            .split("\n\n")
            .filter_map(|block: &str| {
                let data = block
                    .lines()
                    .find_map(|line: &str| line.strip_prefix("data: "))?;
                serde_json::from_str::<Value>(data).ok()
            })
            .collect()
    }

    /// 构造 Responses API SSE 事件块（event: type\ndata: json\n\n）
    fn make_event(event_type: &str, json: &str) -> Result<Bytes, reqwest::Error> {
        Ok(Bytes::from(format!(
            "event: {event_type}\ndata: {json}\n\n"
        )))
    }

    // ── 测试 9：文本流式事件完整序列 ──

    #[tokio::test]
    async fn test_text_stream_sequence() {
        let chunks = vec![
            make_event(
                "response.created",
                r#"{"type":"response.created","response":{"id":"resp_001","model":"gpt-4o","status":"in_progress"}}"#,
            ),
            make_event(
                "response.output_item.added",
                r#"{"type":"response.output_item.added","output_index":0,"item":{"id":"item_001","type":"message","role":"assistant","status":"in_progress","content":[]}}"#,
            ),
            make_event(
                "response.content_part.added",
                r#"{"type":"response.content_part.added","item_id":"item_001","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}"#,
            ),
            make_event(
                "response.output_text.delta",
                r#"{"type":"response.output_text.delta","item_id":"item_001","output_index":0,"content_index":0,"delta":"Hel"}"#,
            ),
            make_event(
                "response.output_text.delta",
                r#"{"type":"response.output_text.delta","item_id":"item_001","output_index":0,"content_index":0,"delta":"lo"}"#,
            ),
            make_event(
                "response.output_text.done",
                r#"{"type":"response.output_text.done","item_id":"item_001","output_index":0,"content_index":0,"text":"Hello"}"#,
            ),
            make_event(
                "response.output_item.done",
                r#"{"type":"response.output_item.done","output_index":0,"item":{"id":"item_001","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"Hello"}]}}"#,
            ),
            make_event(
                "response.completed",
                r#"{"type":"response.completed","response":{"id":"resp_001","model":"gpt-4o","status":"completed","usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}}"#,
            ),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;
        let types: Vec<&str> = events
            .iter()
            .filter_map(|e| e.get("type").and_then(|v| v.as_str()))
            .collect();

        // 必须包含 message_start
        assert!(
            types.contains(&"message_start"),
            "应有 message_start，实际: {:?}",
            types
        );
        assert_eq!(
            types.iter().filter(|&&t| t == "message_start").count(),
            1,
            "message_start 仅一次"
        );
        assert_eq!(types[0], "message_start", "message_start 必须是第一个事件");

        // content_block_start(text, index:0)
        let cbs = events.iter().find(|e| {
            e.get("type").and_then(|v| v.as_str()) == Some("content_block_start")
                && e.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("text")
        });
        assert!(cbs.is_some(), "应有 content_block_start(text)");
        assert_eq!(cbs.unwrap()["index"], 0, "第一个 content block index 为 0");

        // text_delta 事件
        let text_deltas: Vec<&str> = events
            .iter()
            .filter(|e| {
                e.get("type").and_then(|v| v.as_str()) == Some("content_block_delta")
                    && e.pointer("/delta/type").and_then(|v| v.as_str()) == Some("text_delta")
            })
            .filter_map(|e| e.pointer("/delta/text").and_then(|v| v.as_str()))
            .collect();
        assert!(!text_deltas.is_empty(), "应有 text_delta 事件");
        assert!(text_deltas.contains(&"Hel"), "应含 'Hel'");
        assert!(text_deltas.contains(&"lo"), "应含 'lo'");

        // content_block_stop
        assert!(
            types.contains(&"content_block_stop"),
            "应有 content_block_stop"
        );

        // message_delta stop_reason=end_turn
        let msg_delta = events
            .iter()
            .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta"))
            .expect("应有 message_delta");
        assert_eq!(
            msg_delta
                .pointer("/delta/stop_reason")
                .and_then(|v| v.as_str()),
            Some("end_turn"),
            "文本响应应映射为 end_turn"
        );

        // usage 透传
        assert_eq!(
            msg_delta
                .pointer("/usage/input_tokens")
                .and_then(|v| v.as_u64()),
            Some(10),
            "input_tokens 应透传"
        );
        assert_eq!(
            msg_delta
                .pointer("/usage/output_tokens")
                .and_then(|v| v.as_u64()),
            Some(5),
            "output_tokens 应透传"
        );

        // message_stop 在最末
        assert!(types.contains(&"message_stop"), "应有 message_stop");
        assert_eq!(
            *types.last().unwrap(),
            "message_stop",
            "message_stop 应是最后事件"
        );
    }

    // ── 测试 10：函数调用流式事件 ──

    #[tokio::test]
    async fn test_function_call_stream() {
        let chunks = vec![
            make_event(
                "response.created",
                r#"{"type":"response.created","response":{"id":"resp_fn01","model":"gpt-4o","status":"in_progress"}}"#,
            ),
            make_event(
                "response.output_item.added",
                r#"{"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_001","call_id":"call_abc","name":"get_weather","arguments":"","status":"in_progress"}}"#,
            ),
            make_event(
                "response.function_call_arguments.delta",
                r#"{"type":"response.function_call_arguments.delta","item_id":"fc_001","output_index":0,"delta":"{\"loc"}"#,
            ),
            make_event(
                "response.function_call_arguments.delta",
                r#"{"type":"response.function_call_arguments.delta","item_id":"fc_001","output_index":0,"delta":"ation\":\"Tokyo\"}"}"#,
            ),
            make_event(
                "response.function_call_arguments.done",
                r#"{"type":"response.function_call_arguments.done","item_id":"fc_001","output_index":0,"arguments":"{\"location\":\"Tokyo\"}"}"#,
            ),
            make_event(
                "response.output_item.done",
                r#"{"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","id":"fc_001","call_id":"call_abc","name":"get_weather","arguments":"{\"location\":\"Tokyo\"}","status":"completed"}}"#,
            ),
            make_event(
                "response.completed",
                r#"{"type":"response.completed","response":{"id":"resp_fn01","model":"gpt-4o","status":"completed","usage":{"input_tokens":20,"output_tokens":10}}}"#,
            ),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;
        let types: Vec<&str> = events
            .iter()
            .filter_map(|e| e.get("type").and_then(|v| v.as_str()))
            .collect();

        // content_block_start(tool_use, id:"call_abc", name:"get_weather", index:0)
        let cbs = events.iter().find(|e| {
            e.get("type").and_then(|v| v.as_str()) == Some("content_block_start")
                && e.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("tool_use")
        });
        assert!(cbs.is_some(), "应有 content_block_start(tool_use)");
        let cbs = cbs.unwrap();
        assert_eq!(cbs["index"], 0, "第一个 content block index 为 0");
        assert_eq!(
            cbs.pointer("/content_block/id").and_then(|v| v.as_str()),
            Some("call_abc"),
            "tool_use id 应为 call_abc（来自 call_id）"
        );
        assert_eq!(
            cbs.pointer("/content_block/name").and_then(|v| v.as_str()),
            Some("get_weather")
        );

        // input_json_delta 事件
        let json_deltas: Vec<&str> = events
            .iter()
            .filter(|e| {
                e.get("type").and_then(|v| v.as_str()) == Some("content_block_delta")
                    && e.pointer("/delta/type").and_then(|v| v.as_str()) == Some("input_json_delta")
            })
            .filter_map(|e| e.pointer("/delta/partial_json").and_then(|v| v.as_str()))
            .collect();
        assert!(!json_deltas.is_empty(), "应有 input_json_delta 事件");

        // content_block_stop
        assert!(
            types.contains(&"content_block_stop"),
            "应有 content_block_stop"
        );

        // message_delta stop_reason=tool_use
        let msg_delta = events
            .iter()
            .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta"))
            .expect("应有 message_delta");
        assert_eq!(
            msg_delta
                .pointer("/delta/stop_reason")
                .and_then(|v| v.as_str()),
            Some("tool_use"),
            "函数调用应映射为 tool_use"
        );

        // message_stop
        assert!(types.contains(&"message_stop"), "应有 message_stop");
        assert_eq!(*types.last().unwrap(), "message_stop");
    }

    // ── 测试 11：文本 + 函数调用混合流 ──

    #[tokio::test]
    async fn test_mixed_stream() {
        let chunks = vec![
            make_event(
                "response.created",
                r#"{"type":"response.created","response":{"id":"resp_mix","model":"gpt-4o","status":"in_progress"}}"#,
            ),
            // 第一个 output item：文本
            make_event(
                "response.output_item.added",
                r#"{"type":"response.output_item.added","output_index":0,"item":{"id":"item_text","type":"message","role":"assistant","status":"in_progress","content":[]}}"#,
            ),
            make_event(
                "response.content_part.added",
                r#"{"type":"response.content_part.added","item_id":"item_text","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}"#,
            ),
            make_event(
                "response.output_text.delta",
                r#"{"type":"response.output_text.delta","item_id":"item_text","output_index":0,"content_index":0,"delta":"Let me check."}"#,
            ),
            make_event(
                "response.output_text.done",
                r#"{"type":"response.output_text.done","item_id":"item_text","output_index":0,"content_index":0,"text":"Let me check."}"#,
            ),
            make_event(
                "response.output_item.done",
                r#"{"type":"response.output_item.done","output_index":0,"item":{"id":"item_text","type":"message","role":"assistant","status":"completed"}}"#,
            ),
            // 第二个 output item：函数调用
            make_event(
                "response.output_item.added",
                r#"{"type":"response.output_item.added","output_index":1,"item":{"type":"function_call","id":"fc_001","call_id":"call_mix","name":"get_weather","arguments":"","status":"in_progress"}}"#,
            ),
            make_event(
                "response.function_call_arguments.delta",
                r#"{"type":"response.function_call_arguments.delta","item_id":"fc_001","output_index":1,"delta":"{\"city\":\"Tokyo\"}"}"#,
            ),
            make_event(
                "response.function_call_arguments.done",
                r#"{"type":"response.function_call_arguments.done","item_id":"fc_001","output_index":1,"arguments":"{\"city\":\"Tokyo\"}"}"#,
            ),
            make_event(
                "response.output_item.done",
                r#"{"type":"response.output_item.done","output_index":1,"item":{"type":"function_call","call_id":"call_mix","name":"get_weather","status":"completed"}}"#,
            ),
            make_event(
                "response.completed",
                r#"{"type":"response.completed","response":{"id":"resp_mix","model":"gpt-4o","status":"completed","usage":{"input_tokens":20,"output_tokens":15}}}"#,
            ),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;
        let types: Vec<&str> = events
            .iter()
            .filter_map(|e| e.get("type").and_then(|v| v.as_str()))
            .collect();

        // 应有 2 个 content_block_start
        let start_count = types
            .iter()
            .filter(|&&t| t == "content_block_start")
            .count();
        assert_eq!(
            start_count, 2,
            "应有 2 个 content_block_start（text + tool_use）"
        );

        // 第一个是 text block (index:0)
        let text_cbs = events.iter().find(|e| {
            e.get("type").and_then(|v| v.as_str()) == Some("content_block_start")
                && e.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("text")
        });
        assert!(text_cbs.is_some(), "应有 text content_block_start");
        assert_eq!(text_cbs.unwrap()["index"], 0, "text block index 应为 0");

        // 第二个是 tool_use block (index:1)
        let tool_cbs = events.iter().find(|e| {
            e.get("type").and_then(|v| v.as_str()) == Some("content_block_start")
                && e.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("tool_use")
        });
        assert!(tool_cbs.is_some(), "应有 tool_use content_block_start");
        assert_eq!(tool_cbs.unwrap()["index"], 1, "tool_use block index 应为 1");

        // message_delta stop_reason=tool_use（有函数调用）
        let msg_delta = events
            .iter()
            .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta"))
            .expect("应有 message_delta");
        assert_eq!(
            msg_delta
                .pointer("/delta/stop_reason")
                .and_then(|v| v.as_str()),
            Some("tool_use"),
            "混合流含函数调用应映射为 tool_use"
        );

        // message_stop 在最末
        assert_eq!(*types.last().unwrap(), "message_stop");
    }

    // ── 测试 12：无 Deferred Start 机制 ──
    //
    // Responses API 在 output_item.added 时即携带 call_id 和 name，
    // 因此 content_block_start 应立即发出，不需要等待缓冲。

    #[tokio::test]
    async fn test_stream_no_deferred_start() {
        let chunks = vec![
            make_event(
                "response.created",
                r#"{"type":"response.created","response":{"id":"resp_imm","model":"gpt-4o","status":"in_progress"}}"#,
            ),
            // output_item.added 同时携带 call_id 和 name
            make_event(
                "response.output_item.added",
                r#"{"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_imm","call_id":"call_immediate","name":"immediate_tool","arguments":"","status":"in_progress"}}"#,
            ),
            // 随后才是 arguments delta
            make_event(
                "response.function_call_arguments.delta",
                r#"{"type":"response.function_call_arguments.delta","item_id":"fc_imm","output_index":0,"delta":"{\"key\":\"val\"}"}"#,
            ),
            make_event(
                "response.function_call_arguments.done",
                r#"{"type":"response.function_call_arguments.done","item_id":"fc_imm","output_index":0,"arguments":"{\"key\":\"val\"}"}"#,
            ),
            make_event(
                "response.output_item.done",
                r#"{"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","call_id":"call_immediate","name":"immediate_tool","status":"completed"}}"#,
            ),
            make_event(
                "response.completed",
                r#"{"type":"response.completed","response":{"id":"resp_imm","model":"gpt-4o","status":"completed","usage":{"input_tokens":5,"output_tokens":3}}}"#,
            ),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;

        // content_block_start 在第一个 arguments delta 之前出现
        let cbs_pos = events
            .iter()
            .position(|e| {
                e.get("type").and_then(|v| v.as_str()) == Some("content_block_start")
                    && e.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("tool_use")
            })
            .expect("应有 content_block_start(tool_use)");

        let first_delta_pos = events.iter().position(|e| {
            e.get("type").and_then(|v| v.as_str()) == Some("content_block_delta")
                && e.pointer("/delta/type").and_then(|v| v.as_str()) == Some("input_json_delta")
        });

        if let Some(delta_pos) = first_delta_pos {
            assert!(
                cbs_pos < delta_pos,
                "content_block_start(index:{cbs_pos}) 应在 content_block_delta(index:{delta_pos}) 之前发出"
            );
        }

        // content_block_start 中 id 来自 call_id
        let cbs = &events[cbs_pos];
        assert_eq!(
            cbs.pointer("/content_block/id").and_then(|v| v.as_str()),
            Some("call_immediate"),
            "tool_use id 应来自 call_id 字段"
        );
        assert_eq!(
            cbs.pointer("/content_block/name").and_then(|v| v.as_str()),
            Some("immediate_tool")
        );
    }

    #[tokio::test]
    async fn test_cross_chunk_utf8_text_delta_preserved() {
        let utf8_event = "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"item_id\":\"item_utf8\",\"output_index\":0,\"content_index\":0,\"delta\":\"你\"}\n\n";
        let utf8_bytes = utf8_event.as_bytes();
        let split_start = utf8_bytes
            .windows("你".len())
            .position(|window| window == "你".as_bytes())
            .unwrap();
        let split_at = split_start + 1;

        let chunks = vec![
            make_event(
                "response.created",
                r#"{"type":"response.created","response":{"id":"resp_utf8","model":"gpt-4o","status":"in_progress"}}"#,
            ),
            make_event(
                "response.output_item.added",
                r#"{"type":"response.output_item.added","output_index":0,"item":{"id":"item_utf8","type":"message","role":"assistant","status":"in_progress","content":[]}}"#,
            ),
            make_event(
                "response.content_part.added",
                r#"{"type":"response.content_part.added","item_id":"item_utf8","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}"#,
            ),
            Ok::<Bytes, reqwest::Error>(Bytes::copy_from_slice(&utf8_bytes[..split_at])),
            Ok::<Bytes, reqwest::Error>(Bytes::copy_from_slice(&utf8_bytes[split_at..])),
            make_event(
                "response.output_text.done",
                r#"{"type":"response.output_text.done","item_id":"item_utf8","output_index":0,"content_index":0,"text":"你"}"#,
            ),
            make_event(
                "response.output_item.done",
                r#"{"type":"response.output_item.done","output_index":0,"item":{"id":"item_utf8","type":"message","role":"assistant","status":"completed"}}"#,
            ),
            make_event(
                "response.completed",
                r#"{"type":"response.completed","response":{"id":"resp_utf8","model":"gpt-4o","status":"completed","usage":{"input_tokens":3,"output_tokens":1}}}"#,
            ),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-haiku").await;
        let text_deltas: Vec<&str> = events
            .iter()
            .filter(|e| {
                e.get("type").and_then(|v| v.as_str()) == Some("content_block_delta")
                    && e.pointer("/delta/type").and_then(|v| v.as_str()) == Some("text_delta")
            })
            .filter_map(|e| e.pointer("/delta/text").and_then(|v| v.as_str()))
            .collect();

        assert_eq!(text_deltas, vec!["你"]);
    }
}
