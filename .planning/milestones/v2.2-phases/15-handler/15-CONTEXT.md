# Phase 15: Handler 集成与协议路由 - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

将 Phase 14 实现的三个转换模块（request.rs、response.rs、stream.rs）接入 proxy_handler，实现按 ProtocolType 自动路由：OpenAiChatCompletions 走转换路径，Anthropic 透传零回归。同时实现 MODL-03 模型名映射（handler 层在调用转换函数前完成映射替换）。

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
- 协议路由分支在 handler.rs 中的插入位置和代码结构设计
- UpstreamTarget 是否扩展以携带 upstream_model/upstream_model_map，或在 handler 中另行获取映射数据
- 流式/非流式响应的检测方式（请求 body stream 字段 vs 响应 content-type）
- E2E 验证方式（mock server 集成测试 vs 手动真实 Provider 测试）
- handler.rs 内部函数拆分方式
- 错误处理细节（转换函数内部错误已由 TranslateError/400 处理，handler 层如何包装）

### Locked Decisions (from Phase 14)
- 映射优先级：精确匹配 → upstream_model → 保留原模型名
- 4xx/5xx 错误响应直接透传，不经转换处理（RESP-05）
- 转换失败返回 400 BAD_REQUEST（ProxyError::TranslateError）
- 端点重写使用 build_proxy_endpoint_url()（已实现）
- 所有转换函数是纯函数，handler 层负责调用和组装

</decisions>

<specifics>
## Specific Ideas

- cc-switch 的 handler 实现可参考但不受其局限（CLAUDE.md 规定）
- handler.rs 当前仅 133 行，结构简洁，集成时应保持清晰的路由分支
- model 字段在 Phase 14 中原样透传，Phase 15 handler 层需在调用 anthropic_to_openai() 之前执行模型名映射

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `translate::request::anthropic_to_openai(Value) -> Result<Value, TranslateError>`: 请求转换
- `translate::request::build_proxy_endpoint_url(&str, &str) -> String`: 端点重写
- `translate::response::openai_to_anthropic(Value) -> Result<Value, TranslateError>`: 非流式响应转换
- `translate::response::map_finish_reason(&str) -> &'static str`: finish_reason 映射（stream.rs 中也有副本）
- `translate::stream::create_anthropic_sse_stream(impl Stream, &str) -> impl Stream`: 流式 SSE 转换
- `ProxyError::TranslateError(String)`: 转换错误，返回 400

### Established Patterns
- handler.rs 步骤 A-J 线性流程: 获取 upstream → 提取请求信息 → 读取 body → 拼接 URL → 构建请求 → 发送 → 构建响应 → 流式透传
- ProtocolType match 分支: 已在步骤 G（凭据注入）中使用，可复用模式
- body_bytes 已作为 Bytes 类型存在，可直接 serde_json::from_slice 解析

### Integration Points
- handler.rs 步骤 D（URL 拼接）: 替换为 build_proxy_endpoint_url()
- handler.rs 步骤 H 之前: 插入请求转换 + 模型映射
- handler.rs 步骤 J（响应体）: 根据流式/非流式选择 openai_to_anthropic() 或 create_anthropic_sse_stream()
- state.rs UpstreamTarget: 可能需要扩展以携带模型映射数据
- Provider 切换时 update_upstream() 调用点: 需传入映射数据

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 15-handler*
*Context gathered: 2026-03-14*
