---
status: fixed
trigger: "streaming-token-null: 实时日志 token 列全部显示 \"--\"（即 null），流式请求的 token 数据未正确提取和回写。"
created: 2026-03-18T00:00:00Z
updated: 2026-03-18T00:20:00Z
---

## Current Focus

hypothesis: 两个独立 Bug 均已确认并修复，所有 447 个测试通过
test: cargo test 全量运行
expecting: 修复后流式请求 token 字段正确回写
next_action: 人工验证（发送真实请求，确认流量页 token 列有值）

## Symptoms

expected: 流式请求完成后，实时日志表格中 Token 列应显示 input/output token 数，Duration 列应显示总耗时，缓存数据（cache_read_tokens）应在有缓存时显示
actual: Token 列全部显示 "--"（null），只有 TTFB 有值，duration_ms 和所有 token 字段为 null
errors: 无明显错误日志
reproduction: 启动应用，通过 Claude Code 发送任何请求（走 Anthropic 直通模式），观察流量页面实时日志
started: Phase 28 实现 SSE token 提取以来一直存在

## Eliminated

- hypothesis: 后台 task 未启动或 oneshot channel 未连接
  evidence: handler.rs 代码结构正确——streaming_token_rx 被正确设置，后台 task 正常 spawn
  timestamp: 2026-03-18T00:05:00Z

- hypothesis: Anthropic 直通无 token 提取路径
  evidence: create_anthropic_reverse_model_stream 确实存在 token 提取逻辑，但只看 message_delta；Passthrough 路径（无模型映射时）完全没有 token 提取且不触发后台 task
  timestamp: 2026-03-18T00:08:00Z

## Evidence

- timestamp: 2026-03-18T00:03:00Z
  checked: handler.rs line 508-535
  found: Anthropic 直通模式分两条路径：(1) 有模型映射 → AnthropicPassthrough，走 create_anthropic_reverse_model_stream；(2) 无模型映射 → Passthrough，完全透传，token 不被提取，streaming_token_rx 不被设置
  implication: Claude Code 走 Anthropic 直通且通常无模型映射，故进入 Passthrough 分支，一条 token 数据都不会被提取

- timestamp: 2026-03-18T00:04:00Z
  checked: handler.rs line 747-750
  found: Passthrough 分支体内直接 Body::from_stream，不创建 oneshot，streaming_token_rx 保持 None
  implication: 后台 task 不会启动，DB UPDATE 永远不会执行，token 字段永远 null

- timestamp: 2026-03-18T00:05:00Z
  checked: handler.rs line 200-216 create_anthropic_reverse_model_stream
  found: 只监听 message_delta 事件提取 token；Anthropic SSE 格式中 input_tokens/cache 字段在 message_start 事件中（message.usage），而非 message_delta.usage（后者只含 output_tokens）
  implication: 即使走 AnthropicPassthrough 路径，input_tokens 和 cache 字段也永远是 None

- timestamp: 2026-03-18T00:07:00Z
  checked: translate/stream.rs line 540-546 (OpenAI Chat stream)
  found: stream.rs 里 OpenAI Chat 流式路径正确收集 usage chunk（依赖上游返回 include_usage），用 collected_token_data 暂存后在流结束时回传
  implication: 同样的"需要 include_usage"问题存在于 OpenAI 上游路径，但本次最高优先 Bug 是 Passthrough 路径完全没有 token 提取

- timestamp: 2026-03-18T00:09:00Z
  checked: translate/request.rs anthropic_to_openai()
  found: stream=true 时没有添加 stream_options: {include_usage: true}，OpenAI 兼容上游不会在最终 chunk 返回 usage
  implication: Bug 2：OpenAI 上游流式 token 也是 null

## Resolution

root_cause: |
  Bug 1（最高优先，影响 Anthropic 直通模式）：
  - Passthrough 分支（ProtocolType::Anthropic 且无模型映射）完全不提取 token，streaming_token_rx 为 None，后台 task 永不启动
  - 需要给 Passthrough 也创建 oneshot，在流中扫描 message_start（input_tokens/cache）和 message_delta（output_tokens）事件

  Bug 1 附属：create_anthropic_reverse_model_stream 只看 message_delta，漏掉 message_start 里的 input_tokens 和 cache fields

  Bug 2（影响 OpenAI 上游流式）：
  - anthropic_to_openai() 在 stream=true 时未注入 stream_options: {include_usage: true}
  - OpenAI 兼容 API 不会在最终 chunk 返回 usage，导致 stream.rs 的 collected_token_data 始终为 None

fix: |
  Bug 1A（handler.rs）：将 ProtocolType::Anthropic + /v1/messages 路径统一走 AnthropicPassthrough 模式
    - 原来无模型映射时走 Passthrough（完全不创建 oneshot，token 永远 null）
    - 现在无论有无映射都走 AnthropicPassthrough，保证 oneshot 被创建，后台 task 会执行 DB UPDATE
    - 无映射时 model 名提取后原样透传 body bytes（不重新序列化），行为与原 Passthrough 完全一致
  Bug 1B（handler.rs）：修复 create_anthropic_reverse_model_stream 的 token 提取逻辑
    - 原来只监听 message_delta 提取全部字段（input_tokens 字段在 message_delta 的 usage 里不存在）
    - 现在分两阶段：message_start 提取 message.usage.{input_tokens,cache_creation_input_tokens,cache_read_input_tokens}，message_delta 提取 usage.output_tokens + delta.stop_reason，最终合并为一条 StreamTokenData
  Bug 2（translate/request.rs）：anthropic_to_openai() 中 stream=true 时追加 stream_options: {include_usage: true}
    - OpenAI 兼容 API 流式请求需要此参数才会在最终 chunk 返回 usage 数据
    - stream=false 时不注入（经新增测试 test_stream_false_no_stream_options 验证）

verification: cargo test 全量运行：447 passed, 0 failed
files_changed:
  - src-tauri/src/proxy/handler.rs
  - src-tauri/src/proxy/translate/request.rs
