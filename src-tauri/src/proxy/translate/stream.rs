//! 流式转换：OpenAI SSE 流 → Anthropic SSE 流
//!
//! 实现 STRM-01..04 的全部流式转换逻辑：
//! - STRM-01: 文本 delta 序列（message_start -> content_block_start -> content_block_delta -> content_block_stop -> message_delta -> message_stop）
//! - STRM-02: 工具调用 Deferred Start（id/name 未就绪时缓冲 arguments delta，就绪后先发 content_block_start 再发缓冲内容）
//! - STRM-03: 多工具并发（按 tool_calls.index 独立追踪，互不干扰）
//! - STRM-04: 流结束事件（finish_reason 映射 + 关闭所有打开的 content block）

use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

// ── 内部数据结构 ──

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    id: String,
    #[serde(default)]
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: Delta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<DeltaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct DeltaToolCall {
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<DeltaFunction>,
}

#[derive(Debug, Deserialize)]
struct DeltaFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokensDetails>,
    #[serde(default)]
    cache_read_input_tokens: Option<u32>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PromptTokensDetails {
    #[serde(default)]
    cached_tokens: u32,
}

/// 工具调用流式状态，用于 Deferred Start 机制
///
/// 每个 OpenAI tool_call（按 index 唯一标识）对应一个 ToolBlockState，
/// 追踪该工具在 Anthropic 侧的 content_block index、是否已启动及缓冲的参数片段。
#[derive(Debug, Clone)]
pub struct ToolBlockState {
    /// 该工具在 Anthropic content_block 序列中的编号
    pub anthropic_index: u32,
    /// 工具调用 id（来自 OpenAI delta.tool_calls[*].id）
    pub id: String,
    /// 工具名称（来自 OpenAI delta.tool_calls[*].function.name）
    pub name: String,
    /// 是否已发出 content_block_start 事件
    pub started: bool,
    /// id/name 就绪之前积累的 arguments 片段
    pub pending_args: String,
}

// ── 辅助函数 ──

/// 格式化并返回 Anthropic SSE 事件字节：`event: {type}\ndata: {json}\n\n`
fn format_sse_event(event_type: &str, data: &Value) -> Bytes {
    let json_str = serde_json::to_string(data).unwrap_or_default();
    Bytes::from(format!("event: {event_type}\ndata: {json_str}\n\n"))
}

/// 构造 message_start 事件 payload
fn make_message_start(id: &str, model: &str) -> Value {
    json!({
        "type": "message_start",
        "message": {
            "id": id,
            "type": "message",
            "role": "assistant",
            "model": model,
            "content": [],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": {
                "input_tokens": 0,
                "output_tokens": 0
            }
        }
    })
}

/// 将 OpenAI finish_reason 映射为 Anthropic stop_reason
///
/// 注意：Plan 03 (response.rs) 也提供公开的 `map_finish_reason`，此处为 stream.rs
/// 的内部独立副本，避免在 Wave 2 并行期间引入跨模块依赖。
fn map_finish_reason(reason: &str) -> &'static str {
    match reason {
        "tool_calls" | "function_call" => "tool_use",
        "stop" => "end_turn",
        "length" => "max_tokens",
        "content_filter" => "end_turn",
        _ => "end_turn",
    }
}

/// 从 Usage 中提取 cache_read_input_tokens（兼容 OpenAI prompt_tokens_details 嵌套格式）
fn extract_cache_read_tokens(usage: &Usage) -> Option<u32> {
    // 直接字段优先（部分兼容服务器直接返回 Anthropic 风格字段）
    if let Some(v) = usage.cache_read_input_tokens {
        return Some(v);
    }
    // OpenAI 标准：prompt_tokens_details.cached_tokens
    usage
        .prompt_tokens_details
        .as_ref()
        .map(|d| d.cached_tokens)
        .filter(|&v| v > 0)
}

// ── 核心函数 ──

/// 将 OpenAI SSE 上游字节流转换为 Anthropic SSE 格式的异步流
///
/// # 参数
/// - `upstream`: OpenAI Chat Completions 格式的 SSE 字节流（reqwest body stream）
/// - `model`: 用于 message_start 事件中的 model 字段（调用方传入的 Anthropic 模型名）
///
/// # 返回
/// Anthropic SSE 格式字节流，每个 item 为完整的 SSE 块（`event: {type}\ndata: {json}\n\n`）
///
/// # 流式事件顺序
/// 文本响应：message_start -> content_block_start(text) -> content_block_delta(text_delta)... ->
///            content_block_stop -> message_delta -> message_stop
///
/// 工具调用：message_start -> [content_block_start(tool_use) -> content_block_delta(input_json_delta)...]... ->
///            content_block_stop... -> message_delta -> message_stop
pub fn create_anthropic_sse_stream(
    upstream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
    model: String,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send {
    async_stream::stream! {
        // 跨 chunk SSE 行缓冲（处理 SSE 块被 TCP 分片的情况）
        let mut buffer = String::new();

        // message_start 状态
        let mut message_started = false;
        let mut first_chunk_id = String::new();

        // content block 编号管理
        let mut next_anthropic_index: u32 = 0;

        // 当前文本 block 的 Anthropic index（None = 没有打开的文本 block）
        let mut current_text_index: Option<u32> = None;

        // 工具调用状态表（key: OpenAI tool_call.index, value: ToolBlockState）
        let mut tool_blocks_by_index: HashMap<usize, ToolBlockState> = HashMap::new();

        // 已发 content_block_start 但未发 content_block_stop 的 Anthropic block index 集合
        let mut open_block_indices: HashSet<u32> = HashSet::new();

        // finish_reason 是否已处理（防止 [DONE] 重复发 message_stop）
        let mut finish_handled = false;

        tokio::pin!(upstream);

        'outer: while let Some(chunk) = upstream.next().await {
            match chunk {
                Err(e) => {
                    // 上游流 IO 错误 => 转换为 io::Error 并终止流
                    log::error!("[stream] 上游流读取错误: {e}");
                    yield Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
                    break 'outer;
                }
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    buffer.push_str(&text);

                    // 每次消耗一个完整 SSE 块（以 \n\n 结尾）
                    // 不完整的块留在 buffer 中，等待下一个 chunk 拼接
                    while let Some(pos) = buffer.find("\n\n") {
                        let block = buffer[..pos].to_string();
                        buffer = buffer[pos + 2..].to_string();

                        if block.trim().is_empty() {
                            continue;
                        }

                        // 解析 SSE 块中的 data: 行
                        for line in block.lines() {
                            let Some(data) = line.strip_prefix("data: ") else {
                                // 非 data: 前缀行（event:, id:, 注释等）跳过
                                continue;
                            };

                            // 流结束信号
                            if data.trim() == "[DONE]" {
                                if !finish_handled {
                                    // finish_reason chunk 之前收到 [DONE]（异常情况）补发 message_stop
                                    let event = json!({"type": "message_stop"});
                                    yield Ok(format_sse_event("message_stop", &event));
                                }
                                break 'outer;
                            }

                            // 解析 OpenAI chunk JSON（解析失败则跳过该行，继续处理后续）
                            let chunk_val: OpenAiStreamChunk =
                                match serde_json::from_str(data) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        log::debug!("[stream] 跳过无法解析的 SSE 数据: {e}");
                                        continue;
                                    }
                                };

                            // 记录首个 chunk id（用于 message_start）
                            if first_chunk_id.is_empty() {
                                first_chunk_id.clone_from(&chunk_val.id);
                            }

                            let Some(choice) = chunk_val.choices.first() else {
                                continue;
                            };

                            // ── message_start（仅在首次有效 delta 时发送一次）──
                            if !message_started {
                                message_started = true;
                                let start_data = make_message_start(&first_chunk_id, &model);
                                yield Ok(format_sse_event("message_start", &start_data));
                            }

                            // ── 文本 delta 处理（STRM-01）──
                            if let Some(content) = &choice.delta.content {
                                if !content.is_empty() {
                                    // 首个文本 delta：打开 text content block
                                    if current_text_index.is_none() {
                                        let idx = next_anthropic_index;
                                        next_anthropic_index += 1;
                                        current_text_index = Some(idx);
                                        open_block_indices.insert(idx);

                                        let event = json!({
                                            "type": "content_block_start",
                                            "index": idx,
                                            "content_block": {
                                                "type": "text",
                                                "text": ""
                                            }
                                        });
                                        yield Ok(format_sse_event("content_block_start", &event));
                                    }

                                    let idx = current_text_index.unwrap();
                                    let event = json!({
                                        "type": "content_block_delta",
                                        "index": idx,
                                        "delta": {
                                            "type": "text_delta",
                                            "text": content
                                        }
                                    });
                                    yield Ok(format_sse_event("content_block_delta", &event));
                                }
                            }

                            // ── 工具调用 delta 处理（STRM-02 Deferred Start + STRM-03 多工具）──
                            if let Some(tool_calls) = &choice.delta.tool_calls {
                                // 文本 block 遇到工具 delta 时关闭（文本和工具不混排）
                                if let Some(text_idx) = current_text_index.take() {
                                    open_block_indices.remove(&text_idx);
                                    let event = json!({
                                        "type": "content_block_stop",
                                        "index": text_idx
                                    });
                                    yield Ok(format_sse_event("content_block_stop", &event));
                                }

                                for tool_call in tool_calls {
                                    // ── Deferred Start 核心逻辑 ──
                                    //
                                    // 规则：只有当 id 和 name 同时就绪时，才发出 content_block_start。
                                    // 在此之前收到的 arguments delta 先缓冲到 pending_args。
                                    //
                                    // 技术要点（参考 cc-switch 第 280-347 行）：
                                    // 必须在 mutable borrow 作用域内将所有需要的值提取到局部变量，
                                    // borrow 结束后才能 yield（规避 Rust 借用检查器的 &mut 冲突）。
                                    let (
                                        anthropic_index,
                                        id,
                                        name,
                                        should_start,
                                        pending_after_start,
                                        immediate_delta,
                                    ) = {
                                        let state = tool_blocks_by_index
                                            .entry(tool_call.index)
                                            .or_insert_with(|| {
                                                let idx = next_anthropic_index;
                                                next_anthropic_index += 1;
                                                ToolBlockState {
                                                    anthropic_index: idx,
                                                    id: String::new(),
                                                    name: String::new(),
                                                    started: false,
                                                    pending_args: String::new(),
                                                }
                                            });

                                        // 更新 id / name
                                        if let Some(id) = &tool_call.id {
                                            state.id.clone_from(id);
                                        }
                                        if let Some(func) = &tool_call.function {
                                            if let Some(name) = &func.name {
                                                state.name.clone_from(name);
                                            }
                                        }

                                        // 判断是否应该发出 content_block_start
                                        let should_start = !state.started
                                            && !state.id.is_empty()
                                            && !state.name.is_empty();
                                        if should_start {
                                            state.started = true;
                                        }

                                        // 就绪后一次性刷出缓冲的 pending_args
                                        let pending_after_start = if should_start
                                            && !state.pending_args.is_empty()
                                        {
                                            Some(std::mem::take(&mut state.pending_args))
                                        } else {
                                            None
                                        };

                                        // 本 chunk 的 arguments 片段
                                        let args_delta = tool_call
                                            .function
                                            .as_ref()
                                            .and_then(|f| f.arguments.clone());
                                        let immediate_delta = if let Some(args) = args_delta {
                                            if state.started {
                                                // 已 started：直接作为 immediate_delta 发出
                                                Some(args)
                                            } else {
                                                // 未 started：缓冲，等待 id/name 就绪
                                                state.pending_args.push_str(&args);
                                                None
                                            }
                                        } else {
                                            None
                                        };

                                        (
                                            state.anthropic_index,
                                            state.id.clone(),
                                            state.name.clone(),
                                            should_start,
                                            pending_after_start,
                                            immediate_delta,
                                        )
                                    };
                                    // mutable borrow 在此结束，可以安全 yield

                                    // 步骤 1：content_block_start（id/name 首次同时就绪）
                                    if should_start {
                                        open_block_indices.insert(anthropic_index);
                                        let event = json!({
                                            "type": "content_block_start",
                                            "index": anthropic_index,
                                            "content_block": {
                                                "type": "tool_use",
                                                "id": id,
                                                "name": name
                                            }
                                        });
                                        yield Ok(format_sse_event("content_block_start", &event));
                                    }

                                    // 步骤 2：刷出缓冲的 pending_args（Deferred Start 场景）
                                    if let Some(args) = pending_after_start {
                                        let event = json!({
                                            "type": "content_block_delta",
                                            "index": anthropic_index,
                                            "delta": {
                                                "type": "input_json_delta",
                                                "partial_json": args
                                            }
                                        });
                                        yield Ok(format_sse_event("content_block_delta", &event));
                                    }

                                    // 步骤 3：本 chunk 的即时 delta
                                    if let Some(args) = immediate_delta {
                                        let event = json!({
                                            "type": "content_block_delta",
                                            "index": anthropic_index,
                                            "delta": {
                                                "type": "input_json_delta",
                                                "partial_json": args
                                            }
                                        });
                                        yield Ok(format_sse_event("content_block_delta", &event));
                                    }
                                }
                            }

                            // ── 流结束处理（STRM-04）──
                            if let Some(finish_reason) = &choice.finish_reason {
                                finish_handled = true;

                                // 关闭文本 block（如仍打开）
                                if let Some(text_idx) = current_text_index.take() {
                                    open_block_indices.remove(&text_idx);
                                    let event = json!({
                                        "type": "content_block_stop",
                                        "index": text_idx
                                    });
                                    yield Ok(format_sse_event("content_block_stop", &event));
                                }

                                // 处理尚未启动但有 payload 的工具（id/name 未在流中出现的退化情况）
                                let mut late_starts: Vec<(u32, String, String, String)> = Vec::new();
                                for (tool_idx, state) in tool_blocks_by_index.iter_mut() {
                                    if state.started {
                                        continue;
                                    }
                                    let has_payload = !state.pending_args.is_empty()
                                        || !state.id.is_empty()
                                        || !state.name.is_empty();
                                    if !has_payload {
                                        continue;
                                    }
                                    let fallback_id = if state.id.is_empty() {
                                        format!("tool_call_{tool_idx}")
                                    } else {
                                        state.id.clone()
                                    };
                                    let fallback_name = if state.name.is_empty() {
                                        "unknown_tool".to_string()
                                    } else {
                                        state.name.clone()
                                    };
                                    state.started = true;
                                    let pending = std::mem::take(&mut state.pending_args);
                                    late_starts.push((
                                        state.anthropic_index,
                                        fallback_id,
                                        fallback_name,
                                        pending,
                                    ));
                                }
                                // 按 anthropic_index 升序发出（保持确定性顺序）
                                late_starts.sort_unstable_by_key(|(idx, _, _, _)| *idx);
                                for (idx, fid, fname, pending) in late_starts {
                                    open_block_indices.insert(idx);
                                    let event = json!({
                                        "type": "content_block_start",
                                        "index": idx,
                                        "content_block": {
                                            "type": "tool_use",
                                            "id": fid,
                                            "name": fname
                                        }
                                    });
                                    yield Ok(format_sse_event("content_block_start", &event));
                                    if !pending.is_empty() {
                                        let event = json!({
                                            "type": "content_block_delta",
                                            "index": idx,
                                            "delta": {
                                                "type": "input_json_delta",
                                                "partial_json": pending
                                            }
                                        });
                                        yield Ok(format_sse_event("content_block_delta", &event));
                                    }
                                }

                                // 关闭所有打开的工具 block（按 index 升序保证确定性）
                                if !open_block_indices.is_empty() {
                                    let mut indices: Vec<u32> =
                                        open_block_indices.iter().copied().collect();
                                    indices.sort_unstable();
                                    for idx in indices {
                                        let event = json!({
                                            "type": "content_block_stop",
                                            "index": idx
                                        });
                                        yield Ok(format_sse_event("content_block_stop", &event));
                                    }
                                    open_block_indices.clear();
                                }

                                // message_delta（含 stop_reason + usage）
                                let stop_reason = map_finish_reason(finish_reason);
                                let usage_val: Value = chunk_val.usage.as_ref().map(|u| {
                                    let mut uj = json!({
                                        "input_tokens": u.prompt_tokens,
                                        "output_tokens": u.completion_tokens
                                    });
                                    if let Some(cached) = extract_cache_read_tokens(u) {
                                        uj["cache_read_input_tokens"] = json!(cached);
                                    }
                                    if let Some(created) = u.cache_creation_input_tokens {
                                        uj["cache_creation_input_tokens"] = json!(created);
                                    }
                                    uj
                                }).unwrap_or(Value::Null);
                                let event = json!({
                                    "type": "message_delta",
                                    "delta": {
                                        "stop_reason": stop_reason,
                                        "stop_sequence": null
                                    },
                                    "usage": usage_val
                                });
                                yield Ok(format_sse_event("message_delta", &event));

                                // message_stop
                                let event = json!({"type": "message_stop"});
                                yield Ok(format_sse_event("message_stop", &event));
                            }
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

    use super::create_anthropic_sse_stream;

    // ── 测试辅助函数 ──

    async fn collect_events(
        upstream: impl futures::Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
        model: &str,
    ) -> Vec<Value> {
        let out_stream = create_anthropic_sse_stream(upstream, model.to_string());
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

    fn make_chunk(json: &str) -> Result<Bytes, reqwest::Error> {
        Ok(Bytes::from(format!("data: {json}\n\n")))
    }

    fn done_chunk() -> Result<Bytes, reqwest::Error> {
        Ok(Bytes::from("data: [DONE]\n\n"))
    }

    // ── STRM-01: 文本 delta 完整序列 ──

    #[tokio::test]
    async fn test_text_delta_full_sequence() {
        let chunks = vec![
            make_chunk(r#"{"id":"chatcmpl-1","model":"gpt-4o","choices":[{"delta":{"content":"Hello"}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-1","model":"gpt-4o","choices":[{"delta":{"content":" world"}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-1","model":"gpt-4o","choices":[{"delta":{},"finish_reason":"stop"}]}"#),
            done_chunk(),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;
        let types: Vec<&str> = events
            .iter()
            .filter_map(|e| e.get("type").and_then(|v| v.as_str()))
            .collect();

        // message_start 仅出现一次且在最前
        assert!(types.contains(&"message_start"), "应有 message_start");
        assert_eq!(types.iter().filter(|&&t| t == "message_start").count(), 1, "message_start 仅一次");
        assert_eq!(types[0], "message_start", "message_start 必须是第一个事件");

        // message_start 包含正确的 model 名
        let msg_start = events.iter().find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_start")).unwrap();
        assert_eq!(
            msg_start.pointer("/message/model").and_then(|v| v.as_str()),
            Some("claude-3-5-sonnet"),
            "message_start 应含传入的 model 名"
        );

        // content_block_start(text)
        assert!(
            events.iter().any(|e| e.get("type").and_then(|v| v.as_str()) == Some("content_block_start")
                && e.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("text")),
            "应有 content_block_start(text)"
        );

        // text_delta 包含正确内容
        let text_deltas: Vec<&str> = events
            .iter()
            .filter(|e| {
                e.get("type").and_then(|v| v.as_str()) == Some("content_block_delta")
                    && e.pointer("/delta/type").and_then(|v| v.as_str()) == Some("text_delta")
            })
            .filter_map(|e| e.pointer("/delta/text").and_then(|v| v.as_str()))
            .collect();
        assert!(!text_deltas.is_empty(), "应有 text_delta 事件");
        assert!(text_deltas.contains(&"Hello"), "应含 'Hello'");
        assert!(text_deltas.contains(&" world"), "应含 ' world'");

        // content_block_stop
        assert!(types.contains(&"content_block_stop"), "应有 content_block_stop");

        // message_delta stop_reason=end_turn
        let msg_delta = events.iter().find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta")).expect("应有 message_delta");
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

    #[tokio::test]
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
        assert_eq!(starts[0].pointer("/content_block/id").and_then(|v| v.as_str()), Some("call_0"));
        assert_eq!(starts[0].pointer("/content_block/name").and_then(|v| v.as_str()), Some("my_tool"));

        // 缓冲的参数应在 start 之后的 input_json_delta 中出现
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

        // message_delta stop_reason=tool_use
        let msg_delta = events.iter().find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta")).expect("应有 message_delta");
        assert_eq!(
            msg_delta.pointer("/delta/stop_reason").and_then(|v| v.as_str()),
            Some("tool_use"),
            "finish_reason=tool_calls 应映射为 tool_use"
        );
    }

    // ── STRM-03: 多工具并发 ──

    #[tokio::test]
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

        // 两个工具各自有独立 anthropic_index
        let mut tool_index_by_id: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
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
        assert_ne!(tool_index_by_id.get("call_0"), tool_index_by_id.get("call_1"), "两个工具 index 应不同");

        // 每个工具的 input_json_delta 路由到正确 index
        let deltas: Vec<(u64, String)> = events
            .iter()
            .filter(|e| {
                e.get("type").and_then(|v| v.as_str()) == Some("content_block_delta")
                    && e.pointer("/delta/type").and_then(|v| v.as_str()) == Some("input_json_delta")
            })
            .filter_map(|e| {
                let idx = e.get("index").and_then(|v| v.as_u64())?;
                let json = e.pointer("/delta/partial_json").and_then(|v| v.as_str())?.to_string();
                Some((idx, json))
            })
            .collect();
        assert_eq!(deltas.len(), 2, "应有两个 input_json_delta");
        let call0_idx = *tool_index_by_id.get("call_0").unwrap();
        let call1_idx = *tool_index_by_id.get("call_1").unwrap();
        assert!(deltas.iter().any(|(idx, json)| *idx == call0_idx && json == "{\"a\":1}"), "call_0 的参数应路由到其 index");
        assert!(deltas.iter().any(|(idx, json)| *idx == call1_idx && json == "{\"b\":2}"), "call_1 的参数应路由到其 index");
    }

    // ── STRM-04: 流结束关闭所有 block ──

    #[tokio::test]
    async fn test_stream_end_closes_all_blocks() {
        let chunks = vec![
            make_chunk(r#"{"id":"chatcmpl-4","model":"gpt-4o","choices":[{"delta":{"content":"Hi"}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-4","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_x","type":"function","function":{"name":"tool_x"}}]}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-4","model":"gpt-4o","choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{}"}}]}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-4","model":"gpt-4o","choices":[{"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#),
            done_chunk(),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-5-sonnet").await;
        let types: Vec<&str> = events.iter().filter_map(|e| e.get("type").and_then(|v| v.as_str())).collect();

        // content_block_stop 数量应 >= content_block_start 数量
        let start_count = types.iter().filter(|&&t| t == "content_block_start").count();
        let stop_count = types.iter().filter(|&&t| t == "content_block_stop").count();
        assert!(stop_count >= start_count, "stop({stop_count}) 应 >= start({start_count})");

        // message_delta 在所有 content_block_stop 之后
        let last_stop = types.iter().rposition(|&t| t == "content_block_stop").unwrap();
        let delta_pos = types.iter().position(|&t| t == "message_delta").unwrap();
        assert!(delta_pos > last_stop, "message_delta 应在所有 content_block_stop 之后");

        // message_stop 在 message_delta 之后
        let msg_stop_pos = types.iter().position(|&t| t == "message_stop").unwrap();
        assert!(msg_stop_pos > delta_pos, "message_stop 应在 message_delta 之后");
    }

    // ── 跨 chunk SSE 截断处理 ──

    #[tokio::test]
    async fn test_cross_chunk_sse_truncation() {
        // 把一个 SSE 事件拆成两个 TCP chunk
        let part1 = b"data: {\"id\":\"chatcmpl-5\",\"model\":\"gpt-4o\",\"choices\":[{\"delta\":{\"content\":\"Hi\"".to_vec();
        let part2 = b"}}]}\n\ndata: [DONE]\n\n".to_vec();

        let chunks = vec![
            Ok::<_, reqwest::Error>(Bytes::from(part1)),
            Ok::<_, reqwest::Error>(Bytes::from(part2)),
        ];

        let events = collect_events(stream::iter(chunks), "gpt-4o").await;
        let types: Vec<&str> = events.iter().filter_map(|e| e.get("type").and_then(|v| v.as_str())).collect();

        assert!(types.contains(&"content_block_delta"), "截断后应正确产生 content_block_delta");
        assert!(types.contains(&"message_stop"), "截断后 [DONE] 应触发 message_stop");
    }

    // ── finish_reason=length → max_tokens ──

    #[tokio::test]
    async fn test_finish_reason_length_maps_to_max_tokens() {
        let chunks = vec![
            make_chunk(r#"{"id":"chatcmpl-6","model":"gpt-4o","choices":[{"delta":{"content":"..."}}]}"#),
            make_chunk(r#"{"id":"chatcmpl-6","model":"gpt-4o","choices":[{"delta":{},"finish_reason":"length"}]}"#),
            done_chunk(),
        ];

        let events = collect_events(stream::iter(chunks), "claude-3-haiku").await;
        let msg_delta = events.iter().find(|e| e.get("type").and_then(|v| v.as_str()) == Some("message_delta")).expect("应有 message_delta");
        assert_eq!(
            msg_delta.pointer("/delta/stop_reason").and_then(|v| v.as_str()),
            Some("max_tokens"),
            "finish_reason=length 应映射为 max_tokens"
        );
    }
}
