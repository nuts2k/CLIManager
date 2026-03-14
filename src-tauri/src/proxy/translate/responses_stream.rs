//! 流式转换：OpenAI Responses API SSE 流 → Anthropic SSE 流
//!
//! 公开函数：
//! - `create_responses_anthropic_sse_stream()` — 将 Responses API SSE 事件流转换为 Anthropic SSE 事件序列

use bytes::Bytes;
use futures::stream::Stream;

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
    _upstream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
    _request_model: String,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send {
    // TDD RED: 未实现
    futures::stream::empty::<Result<Bytes, std::io::Error>>()
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
        let out_stream =
            create_responses_anthropic_sse_stream(upstream, model.to_string());
        let chunks: Vec<_> = out_stream.collect().await;
        let merged: String = chunks
            .into_iter()
            .map(|r: Result<Bytes, _>| String::from_utf8_lossy(r.unwrap().as_ref()).to_string())
            .collect();
        merged
            .split("\n\n")
            .filter_map(|block: &str| {
                let data = block.lines().find_map(|line: &str| line.strip_prefix("data: "))?;
                serde_json::from_str::<Value>(data).ok()
            })
            .collect()
    }

    /// 构造 Responses API SSE 事件块（event: type\ndata: json\n\n）
    fn make_event(event_type: &str, json: &str) -> Result<Bytes, reqwest::Error> {
        Ok(Bytes::from(format!("event: {event_type}\ndata: {json}\n\n")))
    }

    // ── 测试 9：文本流式事件完整序列 ──

    #[tokio::test]
    async fn test_text_stream_sequence() {
        let chunks = vec![
            make_event("response.created", r#"{"type":"response.created","response":{"id":"resp_001","model":"gpt-4o","status":"in_progress"}}"#),
            make_event("response.output_item.added", r#"{"type":"response.output_item.added","output_index":0,"item":{"id":"item_001","type":"message","role":"assistant","status":"in_progress","content":[]}}"#),
            make_event("response.content_part.added", r#"{"type":"response.content_part.added","item_id":"item_001","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}"#),
            make_event("response.output_text.delta", r#"{"type":"response.output_text.delta","item_id":"item_001","output_index":0,"content_index":0,"delta":"Hel"}"#),
            make_event("response.output_text.delta", r#"{"type":"response.output_text.delta","item_id":"item_001","output_index":0,"content_index":0,"delta":"lo"}"#),
            make_event("response.output_text.done", r#"{"type":"response.output_text.done","item_id":"item_001","output_index":0,"content_index":0,"text":"Hello"}"#),
            make_event("response.output_item.done", r#"{"type":"response.output_item.done","output_index":0,"item":{"id":"item_001","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"Hello"}]}}"#),
            make_event("response.completed", r#"{"type":"response.completed","response":{"id":"resp_001","model":"gpt-4o","status":"completed","usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}}"#),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;
        let types: Vec<&str> = events
            .iter()
            .filter_map(|e| e.get("type").and_then(|v| v.as_str()))
            .collect();

        // 必须包含 message_start
        assert!(types.contains(&"message_start"), "应有 message_start，实际: {:?}", types);
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
        assert!(types.contains(&"content_block_stop"), "应有 content_block_stop");

        // message_delta stop_reason=end_turn
        let msg_delta = events
            .iter()
            .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta"))
            .expect("应有 message_delta");
        assert_eq!(
            msg_delta.pointer("/delta/stop_reason").and_then(|v| v.as_str()),
            Some("end_turn"),
            "文本响应应映射为 end_turn"
        );

        // usage 透传
        assert_eq!(
            msg_delta.pointer("/usage/input_tokens").and_then(|v| v.as_u64()),
            Some(10),
            "input_tokens 应透传"
        );
        assert_eq!(
            msg_delta.pointer("/usage/output_tokens").and_then(|v| v.as_u64()),
            Some(5),
            "output_tokens 应透传"
        );

        // message_stop 在最末
        assert!(types.contains(&"message_stop"), "应有 message_stop");
        assert_eq!(*types.last().unwrap(), "message_stop", "message_stop 应是最后事件");
    }

    // ── 测试 10：函数调用流式事件 ──

    #[tokio::test]
    async fn test_function_call_stream() {
        let chunks = vec![
            make_event("response.created", r#"{"type":"response.created","response":{"id":"resp_fn01","model":"gpt-4o","status":"in_progress"}}"#),
            make_event("response.output_item.added", r#"{"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_001","call_id":"call_abc","name":"get_weather","arguments":"","status":"in_progress"}}"#),
            make_event("response.function_call_arguments.delta", r#"{"type":"response.function_call_arguments.delta","item_id":"fc_001","output_index":0,"delta":"{\"loc"}"#),
            make_event("response.function_call_arguments.delta", r#"{"type":"response.function_call_arguments.delta","item_id":"fc_001","output_index":0,"delta":"ation\":\"Tokyo\"}"}"#),
            make_event("response.function_call_arguments.done", r#"{"type":"response.function_call_arguments.done","item_id":"fc_001","output_index":0,"arguments":"{\"location\":\"Tokyo\"}"}"#),
            make_event("response.output_item.done", r#"{"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","id":"fc_001","call_id":"call_abc","name":"get_weather","arguments":"{\"location\":\"Tokyo\"}","status":"completed"}}"#),
            make_event("response.completed", r#"{"type":"response.completed","response":{"id":"resp_fn01","model":"gpt-4o","status":"completed","usage":{"input_tokens":20,"output_tokens":10}}}"#),
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
        assert!(types.contains(&"content_block_stop"), "应有 content_block_stop");

        // message_delta stop_reason=tool_use
        let msg_delta = events
            .iter()
            .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta"))
            .expect("应有 message_delta");
        assert_eq!(
            msg_delta.pointer("/delta/stop_reason").and_then(|v| v.as_str()),
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
            make_event("response.created", r#"{"type":"response.created","response":{"id":"resp_mix","model":"gpt-4o","status":"in_progress"}}"#),
            // 第一个 output item：文本
            make_event("response.output_item.added", r#"{"type":"response.output_item.added","output_index":0,"item":{"id":"item_text","type":"message","role":"assistant","status":"in_progress","content":[]}}"#),
            make_event("response.content_part.added", r#"{"type":"response.content_part.added","item_id":"item_text","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}"#),
            make_event("response.output_text.delta", r#"{"type":"response.output_text.delta","item_id":"item_text","output_index":0,"content_index":0,"delta":"Let me check."}"#),
            make_event("response.output_text.done", r#"{"type":"response.output_text.done","item_id":"item_text","output_index":0,"content_index":0,"text":"Let me check."}"#),
            make_event("response.output_item.done", r#"{"type":"response.output_item.done","output_index":0,"item":{"id":"item_text","type":"message","role":"assistant","status":"completed"}}"#),
            // 第二个 output item：函数调用
            make_event("response.output_item.added", r#"{"type":"response.output_item.added","output_index":1,"item":{"type":"function_call","id":"fc_001","call_id":"call_mix","name":"get_weather","arguments":"","status":"in_progress"}}"#),
            make_event("response.function_call_arguments.delta", r#"{"type":"response.function_call_arguments.delta","item_id":"fc_001","output_index":1,"delta":"{\"city\":\"Tokyo\"}"}"#),
            make_event("response.function_call_arguments.done", r#"{"type":"response.function_call_arguments.done","item_id":"fc_001","output_index":1,"arguments":"{\"city\":\"Tokyo\"}"}"#),
            make_event("response.output_item.done", r#"{"type":"response.output_item.done","output_index":1,"item":{"type":"function_call","call_id":"call_mix","name":"get_weather","status":"completed"}}"#),
            make_event("response.completed", r#"{"type":"response.completed","response":{"id":"resp_mix","model":"gpt-4o","status":"completed","usage":{"input_tokens":20,"output_tokens":15}}}"#),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;
        let types: Vec<&str> = events
            .iter()
            .filter_map(|e| e.get("type").and_then(|v| v.as_str()))
            .collect();

        // 应有 2 个 content_block_start
        let start_count = types.iter().filter(|&&t| t == "content_block_start").count();
        assert_eq!(start_count, 2, "应有 2 个 content_block_start（text + tool_use）");

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
            msg_delta.pointer("/delta/stop_reason").and_then(|v| v.as_str()),
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
            make_event("response.created", r#"{"type":"response.created","response":{"id":"resp_imm","model":"gpt-4o","status":"in_progress"}}"#),
            // output_item.added 同时携带 call_id 和 name
            make_event("response.output_item.added", r#"{"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_imm","call_id":"call_immediate","name":"immediate_tool","arguments":"","status":"in_progress"}}"#),
            // 随后才是 arguments delta
            make_event("response.function_call_arguments.delta", r#"{"type":"response.function_call_arguments.delta","item_id":"fc_imm","output_index":0,"delta":"{\"key\":\"val\"}"}"#),
            make_event("response.function_call_arguments.done", r#"{"type":"response.function_call_arguments.done","item_id":"fc_imm","output_index":0,"arguments":"{\"key\":\"val\"}"}"#),
            make_event("response.output_item.done", r#"{"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","call_id":"call_immediate","name":"immediate_tool","status":"completed"}}"#),
            make_event("response.completed", r#"{"type":"response.completed","response":{"id":"resp_imm","model":"gpt-4o","status":"completed","usage":{"input_tokens":5,"output_tokens":3}}}"#),
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
}
