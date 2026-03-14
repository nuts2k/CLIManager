//! 流式转换：OpenAI SSE 流 → Anthropic SSE 流
//!
//! 公开函数：
//! - `create_anthropic_sse_stream()` — 将 OpenAI SSE 流转换为 Anthropic SSE 流（Wave 2 Plan 04 实现）

use bytes::Bytes;
use futures::Stream;

/// OpenAI SSE 流 → Anthropic SSE 流转换（占位实现，Plan 14-04 实现完整逻辑）
///
/// # 参数
/// - `upstream`: OpenAI SSE 上游字节流
/// - `model`: 用于 message_start 事件中的 model 字段
pub fn create_anthropic_sse_stream<S>(
    upstream: S,
    _model: String,
) -> impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
{
    // 占位：Plan 14-04 将实现完整的 Deferred Start 逻辑
    upstream
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures::stream;
    use futures::StreamExt;
    use serde_json::Value;

    // 在实现就绪之前先引用，让测试失败（RED）
    use super::create_anthropic_sse_stream;

    // ── 辅助函数：把流收集为 SSE 事件列表 ──

    async fn collect_events(
        upstream: impl futures::Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
        model: &str,
    ) -> Vec<Value> {
        let stream = create_anthropic_sse_stream(upstream, model.to_string());
        let chunks: Vec<_> = stream.collect().await;
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

    fn make_chunk(json: &str) -> Result<Bytes, reqwest::Error> {
        Ok(Bytes::from(format!("data: {json}\n\n")))
    }

    fn done_chunk() -> Result<Bytes, reqwest::Error> {
        Ok(Bytes::from("data: [DONE]\n\n"))
    }

    // ── STRM-01: 文本 delta 序列 ──

    /// 文本 delta 应产生完整序列：message_start -> content_block_start(text) ->
    /// content_block_delta(text_delta)... -> content_block_stop -> message_delta -> message_stop
    #[tokio::test]
    #[ignore = "Plan 04 RED 测试：create_anthropic_sse_stream 尚未实现完整转换逻辑"]
    async fn test_text_delta_full_sequence() {
        let chunks = vec![
            make_chunk(r#"{"id":"chatcmpl-1","model":"gpt-4o","choices":[{"delta":{"content":"Hello"}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-1","model":"gpt-4o","choices":[{"delta":{"content":" world"}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-1","model":"gpt-4o","choices":[{"delta":{},"finish_reason":"stop"}]}"#),
            done_chunk(),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;

        // message_start 仅出现一次且在最前面
        let types: Vec<&str> = events
            .iter()
            .filter_map(|e| e.get("type").and_then(|v| v.as_str()))
            .collect();

        assert!(types.contains(&"message_start"), "应有 message_start");
        assert_eq!(
            types.iter().filter(|&&t| t == "message_start").count(),
            1,
            "message_start 仅出现一次"
        );
        assert_eq!(types[0], "message_start", "message_start 必须是第一个事件");

        // message_start 包含 model 字段
        let msg_start = events
            .iter()
            .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_start"))
            .unwrap();
        assert_eq!(
            msg_start.pointer("/message/model").and_then(|v| v.as_str()),
            Some("claude-3-5-sonnet"),
            "message_start 应包含传入的 model 名"
        );

        // content_block_start(type=text)
        assert!(
            events.iter().any(|e| e.get("type").and_then(|v| v.as_str()) == Some("content_block_start")
                && e.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("text")),
            "应有 content_block_start(text)"
        );

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
        assert!(text_deltas.contains(&"Hello"), "应含 'Hello' 文本");
        assert!(text_deltas.contains(&" world"), "应含 ' world' 文本");

        // content_block_stop
        assert!(
            types.contains(&"content_block_stop"),
            "应有 content_block_stop"
        );

        // message_delta 含 stop_reason=end_turn
        let msg_delta = events
            .iter()
            .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta"))
            .expect("应有 message_delta");
        assert_eq!(
            msg_delta.pointer("/delta/stop_reason").and_then(|v| v.as_str()),
            Some("end_turn"),
            "finish_reason=stop 应映射为 end_turn"
        );

        // message_stop 在最末
        assert!(types.contains(&"message_stop"), "应有 message_stop");
        assert_eq!(*types.last().unwrap(), "message_stop", "message_stop 应是最后事件");
    }

    // ── STRM-02: Deferred Start 工具调用 ──

    /// id/name 未就绪时缓冲 arguments，就绪后先发 content_block_start 再发缓冲的 input_json_delta
    #[tokio::test]
    #[ignore = "Plan 04 RED 测试：create_anthropic_sse_stream 尚未实现完整转换逻辑"]
    async fn test_tool_deferred_start() {
        let chunks = vec![
            // 第一个 chunk 只有 arguments，没有 id/name
            make_chunk(r#"{"id":"chatcmpl-2","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"a\":"}}]}}]}"#),
            // 第二个 chunk 带来 id 和 name
            make_chunk(r#"{"id":"chatcmpl-2","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_0","type":"function","function":{"name":"my_tool"}}]}}]}"#),
            // 第三个 chunk 继续 arguments
            make_chunk(r#"{"id":"chatcmpl-2","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"1}"}}]}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-2","model":"gpt-4o","choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#),
            done_chunk(),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;

        // 只有一个 content_block_start(tool_use)
        let starts: Vec<&Value> = events
            .iter()
            .filter(|e| {
                e.get("type").and_then(|v| v.as_str()) == Some("content_block_start")
                    && e.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("tool_use")
            })
            .collect();
        assert_eq!(starts.len(), 1, "只应有一个 tool_use content_block_start");
        assert_eq!(
            starts[0].pointer("/content_block/id").and_then(|v| v.as_str()),
            Some("call_0"),
            "content_block_start 应有正确 id"
        );
        assert_eq!(
            starts[0].pointer("/content_block/name").and_then(|v| v.as_str()),
            Some("my_tool"),
            "content_block_start 应有正确 name"
        );

        // 缓冲的参数（"{\"a\":"）应在 content_block_start 之后以 input_json_delta 发出
        let start_pos = events
            .iter()
            .position(|e| {
                e.get("type").and_then(|v| v.as_str()) == Some("content_block_start")
                    && e.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("tool_use")
            })
            .unwrap();

        let json_deltas: Vec<&str> = events[start_pos + 1..]
            .iter()
            .filter(|e| {
                e.get("type").and_then(|v| v.as_str()) == Some("content_block_delta")
                    && e.pointer("/delta/type").and_then(|v| v.as_str()) == Some("input_json_delta")
            })
            .filter_map(|e| e.pointer("/delta/partial_json").and_then(|v| v.as_str()))
            .collect();

        assert!(json_deltas.contains(&"{\"a\":"), "缓冲的参数应在 start 后发出");
        assert!(json_deltas.contains(&"1}"), "后续参数也应发出");

        // message_delta stop_reason = tool_use
        let msg_delta = events
            .iter()
            .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta"))
            .expect("应有 message_delta");
        assert_eq!(
            msg_delta.pointer("/delta/stop_reason").and_then(|v| v.as_str()),
            Some("tool_use"),
            "finish_reason=tool_calls 应映射为 tool_use"
        );
    }

    // ── STRM-03: 多工具并发 ──

    /// 按 tool_calls.index 独立追踪，不同工具互不干扰
    #[tokio::test]
    #[ignore = "Plan 04 RED 测试：create_anthropic_sse_stream 尚未实现完整转换逻辑"]
    async fn test_multi_tool_concurrent() {
        let chunks = vec![
            make_chunk(r#"{"id":"chatcmpl-3","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_0","type":"function","function":{"name":"first_tool"}}]}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-3","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":1,"id":"call_1","type":"function","function":{"name":"second_tool"}}]}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-3","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":1,"function":{"arguments":"{\"b\":2}"}}]}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-3","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"a\":1}"}}]}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-3","model":"gpt-4o","choices":[{"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":8,"completion_tokens":4}}"#),
            done_chunk(),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;

        // 两个工具各自有独立 index
        let mut tool_index_by_id: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        for e in &events {
            if e.get("type").and_then(|v| v.as_str()) == Some("content_block_start")
                && e.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("tool_use")
            {
                if let (Some(id), Some(idx)) = (
                    e.pointer("/content_block/id").and_then(|v| v.as_str()),
                    e.get("index").and_then(|v| v.as_u64()),
                ) {
                    tool_index_by_id.insert(id.to_string(), idx);
                }
            }
        }
        assert_eq!(tool_index_by_id.len(), 2, "应有两个不同的 tool block");
        assert_ne!(
            tool_index_by_id.get("call_0"),
            tool_index_by_id.get("call_1"),
            "两个工具 index 应不同"
        );

        // 每个工具的 input_json_delta 路由到正确 index
        let deltas: Vec<(u64, String)> = events
            .iter()
            .filter(|e| {
                e.get("type").and_then(|v| v.as_str()) == Some("content_block_delta")
                    && e.pointer("/delta/type").and_then(|v| v.as_str()) == Some("input_json_delta")
            })
            .filter_map(|e| {
                let idx = e.get("index").and_then(|v| v.as_u64())?;
                let json = e
                    .pointer("/delta/partial_json")
                    .and_then(|v| v.as_str())?
                    .to_string();
                Some((idx, json))
            })
            .collect();

        assert_eq!(deltas.len(), 2, "应有两个 input_json_delta");
        let call0_idx = *tool_index_by_id.get("call_0").unwrap();
        let call1_idx = *tool_index_by_id.get("call_1").unwrap();
        assert!(
            deltas.iter().any(|(idx, json)| *idx == call0_idx && json == "{\"a\":1}"),
            "call_0 的参数应路由到其 index"
        );
        assert!(
            deltas.iter().any(|(idx, json)| *idx == call1_idx && json == "{\"b\":2}"),
            "call_1 的参数应路由到其 index"
        );
    }

    // ── STRM-04: 流结束事件 ──

    /// finish_reason 触发正确关闭序列：content_block_stop(所有打开的) -> message_delta -> message_stop
    #[tokio::test]
    async fn test_stream_end_closes_all_blocks() {
        // 先有文本 block，再有工具 block，然后 finish
        let chunks = vec![
            make_chunk(r#"{"id":"chatcmpl-4","model":"gpt-4o","choices":[{"delta":{"content":"Hi"}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-4","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_x","type":"function","function":{"name":"tool_x"}}]}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-4","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{}"}}]}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-4","model":"gpt-4o","choices":[{"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#),
            done_chunk(),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;
        let types: Vec<&str> = events
            .iter()
            .filter_map(|e| e.get("type").and_then(|v| v.as_str()))
            .collect();

        // content_block_stop 数量 >= content_block_start 数量
        let start_count = types.iter().filter(|&&t| t == "content_block_start").count();
        let stop_count = types.iter().filter(|&&t| t == "content_block_stop").count();
        assert!(
            stop_count >= start_count,
            "content_block_stop 数量 ({stop_count}) 应 >= content_block_start ({start_count})"
        );

        // message_delta 在所有 content_block_stop 之后
        let last_stop = types.iter().rposition(|&t| t == "content_block_stop").unwrap();
        let delta_pos = types.iter().position(|&t| t == "message_delta").unwrap();
        assert!(
            delta_pos > last_stop,
            "message_delta 应在所有 content_block_stop 之后"
        );

        // message_stop 在 message_delta 之后
        let msg_stop_pos = types.iter().position(|&t| t == "message_stop").unwrap();
        assert!(
            msg_stop_pos > delta_pos,
            "message_stop 应在 message_delta 之后"
        );
    }

    // ── 跨 chunk SSE 截断 ──

    /// 不完整的 SSE 行应缓冲到下一个 chunk 后再处理
    #[tokio::test]
    async fn test_cross_chunk_sse_truncation() {
        // 把一个 SSE 事件拆成两个 chunk
        let part1 = b"data: {\"id\":\"chatcmpl-5\",\"model\":\"gpt-4o\",\"choices\":[{\"delta\":{\"content\":\"Hi\"".to_vec();
        let part2 = b"}}]}\n\ndata: [DONE]\n\n".to_vec();

        let chunks = vec![
            Ok::<_, reqwest::Error>(Bytes::from(part1)),
            Ok::<_, reqwest::Error>(Bytes::from(part2)),
        ];

        let events = collect_events(stream::iter(chunks), "gpt-4o").await;
        let types: Vec<&str> = events
            .iter()
            .filter_map(|e| e.get("type").and_then(|v| v.as_str()))
            .collect();

        // 截断的 JSON 应在拼接后正确解析
        assert!(
            types.contains(&"content_block_delta"),
            "截断后拼接应正确产生 content_block_delta"
        );
        assert!(
            types.contains(&"message_stop"),
            "截断后 [DONE] 应触发 message_stop"
        );
    }

    // ── finish_reason=length 映射 ──

    #[tokio::test]
    async fn test_finish_reason_length_maps_to_max_tokens() {
        let chunks = vec![
            make_chunk(r#"{"id":"chatcmpl-6","model":"gpt-4o","choices":[{"delta":{"content":"..."}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-6","model":"gpt-4o","choices":[{"delta":{},"finish_reason":"length"}]}"#),
            done_chunk(),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-haiku").await;
        let msg_delta = events
            .iter()
            .find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta"))
            .expect("应有 message_delta");
        assert_eq!(
            msg_delta.pointer("/delta/stop_reason").and_then(|v| v.as_str()),
            Some("max_tokens"),
            "finish_reason=length 应映射为 max_tokens"
        );
    }
}
