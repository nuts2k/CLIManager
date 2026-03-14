# Project Research Summary

**Project:** CLIManager v2.2 — Anthropic → OpenAI 协议转换里程碑
**Domain:** HTTP 代理层协议转换（Anthropic Messages API ↔ OpenAI Chat Completions API）
**Researched:** 2026-03-14
**Confidence:** HIGH

## Executive Summary

CLIManager v2.2 的目标是在现有 axum 0.8 代理层上实现 Anthropic Messages API → OpenAI Chat Completions API 的单向协议转换，使 Claude Code CLI 能够无缝使用 OpenAI 兼容的上游 Provider（OpenRouter 等）。研究表明：现有代码库（v2.0/v2.1 已验证的 Tauri 2 + axum 0.8 + reqwest 0.12 + serde_json 技术栈）已具备完成本里程碑所需的全部基础能力，**Cargo.toml 仅需新增 `bytes = "1"` 和 `futures = "0.3"` 两行显式依赖**（两者均已作为传递依赖锁定在 Cargo.lock 中，无额外下载开销）。cc-switch 参考实现（transform.rs 775行 + streaming.rs 744行）完整验证了 `serde_json::Value` 手写转换方案的可行性，且该方案对未知字段的兼容性优于强类型 struct 方式。

协议转换的核心复杂性集中在流式 SSE 转换层。OpenAI 和 Anthropic 的流式模型在架构层面根本不同：OpenAI 仅有无状态 `data:` 行，Anthropic 是严格状态机（`message_start → content_block_start → content_block_delta → content_block_stop → message_delta → message_stop`）。流式工具调用存在"id/name 晚于 arguments 到达"的分帧问题，需要 Deferred Start + pending_args 缓冲机制。多工具并发调用需要按 OpenAI `index` 独立维护状态（`HashMap<usize, ToolBlockState>`）。这些都是必须在 Phase 1 解决的 Critical Pitfall，不能推迟。

非流式路径相对简单：读取完整 OpenAI 响应体 → JSON 字段映射 → 序列化为 Anthropic 格式。请求转换需要正确处理 system prompt 的字符串/数组两种格式、工具定义的 `input_schema → parameters` 重构、tool_result 消息从 Anthropic `user` role 拆分为 OpenAI `tool` role 独立消息。handler 层现有的 body 流式透传架构需要在转换路径下改为条件分支：透传路径（Anthropic Provider）保持不变；转换路径根据 `stream` 字段分叉为非流式缓冲转换和流式 SSE 转换两条互斥路径。

## Key Findings

### Recommended Stack

v2.2 不引入任何新的核心 crate。全部协议转换能力由现有技术栈扩展实现：`serde_json::Value` 做动态 JSON 字段映射（比 typed struct 更能兼容未知字段），`reqwest::bytes_stream()` + `axum::Body::from_stream()` 做流式管道（v2.0 已有），`bytes::Bytes` + `futures::StreamExt` 做流组合子（作为传递依赖已锁定，仅需显式声明）。明确不引入：任何 OpenAI/Anthropic SDK crate（面向调用方设计，不适合桥接场景）、`regex` crate（SSE 行解析用标准库字符串方法足够）、`base64` crate（图片 data URI 直接字符串拼接即可）。

**核心技术（新增显式声明，仅 2 项）：**
- `bytes = "1"`：SSE 流转换中 `Bytes` item 类型，axum 与 reqwest 流管道的共同类型 — 已锁 1.11.1，tokio-rs 生态标配
- `futures = "0.3"`：`StreamExt`、`stream::once` 等流组合子，SSE 转换层逐 chunk 映射所需 — 已锁 0.3.32，dev-dependencies 已声明

**现有技术无需任何变化：**
- `serde_json`：已有，`Value` 动态访问足以应对全部字段转换
- `reqwest 0.12`：已有，`.body(bytes)`、`.bytes_stream()` v2.0 已用
- `axum 0.8`：`Body::from_stream()` v2.0 proxy 中已用
- `tokio`：已有异步运行时

### Expected Features

P1 必做（缺少则 Claude Code 无法使用 OpenAI Provider，全部列为 v2.2 Launch）：

**必须包含（v2.2 Launch）：**
- 协议路由：检测 `ProtocolType == OpenAiCompatible` 进入转换路径，其他 Provider 透传不受影响
- 请求转换：system prompt（string 格式 + array with cache_control 格式两种）→ messages[0]{role: system}
- 请求转换：content block 数组（text/tool_use/tool_result/image）完整转换，thinking block 静默丢弃
- 请求转换：工具定义（input_schema → parameters，clean_schema 递归清理，BatchTool 过滤）
- 端点重写：/v1/messages → /v1/chat/completions（仅 OpenAI Provider 生效）
- 非流式响应转换：choices → content blocks，finish_reason → stop_reason 穷举映射，usage 字段重命名
- 非流式工具调用：tool_calls.function.arguments（JSON 字符串）解析为 JSON 对象填入 tool_use.input
- 流式 SSE 转换：完整状态机（message_start + content_block_start/delta/stop + message_delta + message_stop）
- 流式工具调用：pending buffer Deferred Start + HashMap<usize, ToolBlockState> 多工具并发
- 错误响应格式转换：4xx/5xx 时 OpenAI 错误体 → Anthropic 错误格式，保留原 status code

**增强兼容性（v2.2.x，按需触发）：**
- 图片/多模态转换：base64 → data URL 格式（触发：用户传图片给 OpenAI Provider）
- cache_control 透传：向支持 prompt caching 的兼容 Provider 转发（触发：用户使用特定服务）
- 旧版 function_call 格式兼容（触发：遇到返回旧格式的 Provider）

**推迟到 v3.0+：**
- 反向转换（OpenAI → Anthropic）— Out of Scope，PROJECT.md 明确标注
- OpenAI Responses API 格式支持 — cc-switch 有参考实现，但结构复杂，不在 v2.2 范围
- 模型名称映射配置 — 用户自行填写正确模型名即可

### Architecture Approach

采用"最小侵入"架构策略：在现有 `proxy_handler` 的请求管道（步骤 C 和 D 之间）插入条件转换分支，在响应处理（步骤 I 和 J 之间）插入响应转换分支。所有转换逻辑抽取到新建的 `proxy/translate/` 子模块（4个文件），与现有代理逻辑完全解耦，可独立单元测试。`proxy/state.rs` 和 `provider.rs` 无需任何修改。转换函数设计为纯函数（输入 JSON 值，输出 JSON 值）。不引入 trait 抽象（v2.2 单向转换不需要 Provider adapter 层；v3.0 多协议支持时再考虑）。

**主要组件：**
1. `proxy/handler.rs`（修改）— 请求管道编排，插入转换分支：`needs_translation = protocol_type == OpenAiCompatible && path == "/v1/messages"`；根据 Content-Type 分叉流式/非流式/透传三条路径
2. `proxy/translate/request.rs`（新建）— `anthropic_to_openai(Value) -> Result<Value>` 纯函数；覆盖全部 content block 类型
3. `proxy/translate/response.rs`（新建）— `openai_to_anthropic(Value) -> Result<Value>` 纯函数；含 finish_reason 穷举映射和 usage 重命名
4. `proxy/translate/stream.rs`（新建）— `create_anthropic_sse_stream(Stream) -> Stream` 流适配器；含状态机 + ToolBlockState HashMap
5. `proxy/error.rs`（修改）— 新增 `TransformError(String)` 变体，映射到 422 状态码

**关键架构决策：**
- 转换决策用双重判断（`protocol_type + path`），避免误转换 `/v1/models` 等其他端点
- 透传路径（Anthropic Provider）代码路径完全不变，零回归风险
- 非流式转换：`resp.bytes().await` 全量读取（非流式响应通常 < 10KB，风险可控）
- 流式转换：`resp.bytes_stream()` 接入 SSE 状态机，逐 chunk 转换，不缓冲完整响应
- 错误响应（4xx/5xx）：读取并转换为 Anthropic 错误格式，以原始 status code 返回

### Critical Pitfalls

研究识别出 15 个 Pitfall，以下是最高优先级的 10 个（全部属于 Phase 1 必须解决）：

1. **SSE 事件类型体系根本不同，不能逐行转发** — 必须实现完整状态机转换器；OpenAI 无 `event:` 行，Anthropic 要求 message_start 到 message_stop 完整命名事件序列
2. **工具流式分帧：id/name 可能晚于 arguments 到达** — 实现 Deferred Start：每个 tool_call.index 独立维护状态，等 id + name 都就绪才发 `content_block_start`，中间 arguments 缓冲在 pending_args
3. **多并发工具调用 index 映射错误** — `HashMap<usize, ToolBlockState>` 按 OpenAI index 独立维护状态；Anthropic content block index 用独立递增计数器，与 OpenAI index 完全解耦
4. **tool_result 消息结构错位** — Anthropic `user` role 消息中的 `tool_result` block 必须转为独立的 OpenAI `role: "tool"` 消息；混合内容（tool_result + text）需要拆分为多条消息
5. **thinking/redacted_thinking blocks 透传导致上游 400** — content block 迭代时遇到 `thinking` 类型直接 drop，同时过滤顶层 `thinking` 配置字段（thinking budget）
6. **system prompt 只处理字符串格式，忽略数组格式** — 必须同时处理字符串和数组两种格式；数组格式在 Claude Code 大型项目中是高频路径（cache_control 场景）
7. **错误响应格式未转换** — 4xx/5xx 响应必须从 OpenAI 格式（`{"error": {...}}`）转换为 Anthropic 格式（`{"type": "error", "error": {...}}`）
8. **tool_use input 字段类型不匹配** — 非流式响应中 `function.arguments`（JSON 字符串）必须用 `serde_json::from_str` 解析为 JSON 对象再填入 `tool_use.input`
9. **cache_control 透传触发上游 400** — 转换到 OpenAI 格式时必须丢弃全部 `cache_control` 字段（text block、tool 定义、system prompt 中均可能存在）
10. **finish_reason 映射遗漏边缘值** — 穷举 match（stop→end_turn, length→max_tokens, tool_calls→tool_use, content_filter→end_turn），未知值 fallback 为 end_turn + 警告日志

## Implications for Roadmap

基于研究，建议 2 阶段结构（与 PITFALLS.md 的 Phase to address 映射完全对齐）：

### Phase 1: 协议转换核心（translate 子模块）

**Rationale:** 协议转换的全部业务逻辑集中在 `proxy/translate/` 的三个模块（request/response/stream），这三个模块为纯函数/流适配器，不依赖 axum 上下文，可以独立实现和单元测试。将转换逻辑与集成逻辑分离，确保核心业务逻辑有高质量单元测试覆盖，再进入集成阶段。ARCHITECTURE.md 明确指出 request.rs、response.rs、stream.rs 三个模块可以并行开发，无相互依赖。

**Delivers:**
- `proxy/translate/request.rs`：`anthropic_to_openai()` 纯函数，覆盖 system/messages（含全部 content block 类型）/tools 全部字段转换，含 thinking block drop 和 clean_schema
- `proxy/translate/response.rs`：`openai_to_anthropic()` 纯函数，覆盖 text/tool_calls/finish_reason 穷举映射/usage 重命名 + 嵌套 cache token 映射
- `proxy/translate/stream.rs`：`create_anthropic_sse_stream()` 流适配器，完整状态机 + ToolBlockState HashMap + Deferred Start
- 针对以上函数的单元测试套件（目标：覆盖全部 content block 类型、工具调用流式分帧、多工具并发、错误格式、usage 映射等边缘情况）

**Addresses:** 全部 P1 功能特性的转换逻辑（协议路由和端点重写在 Phase 2 集成到 handler）

**Avoids（必须在本 Phase 解决的 Pitfall）:**
- SSE 事件类型体系根本不同
- 工具流式分帧（Deferred Start + pending_args）
- 多并发工具调用 index 映射（HashMap）
- tool_result 消息结构错位
- tool_use input 字段类型不匹配
- thinking blocks 透传
- system prompt 数组格式
- finish_reason 映射遗漏
- 错误响应格式未转换
- cache_control 透传
- token 计数字段不一致（含嵌套 cached_tokens）
- JSON schema 清理不完整（clean_schema 递归函数）

### Phase 2: handler 集成与端到端验证

**Rationale:** Phase 1 完成后，translate 模块经过单元测试覆盖，具备高置信度。Phase 2 的工作是将转换模块集成到 `proxy_handler`，处理 handler 架构层面的分支（透传 vs 非流式转换 vs 流式转换），并通过端到端测试验证完整请求-响应链路。ARCHITECTURE.md 的 Build Order 明确：步骤 4（error.rs + mod.rs 扩展）和步骤 5（handler 集成）依赖步骤 1-3，步骤 6（e2e 测试）依赖步骤 5。

**Delivers:**
- `proxy/error.rs` 扩展：`TransformError(String)` 变体（422 状态码）
- `proxy/mod.rs` 扩展：`pub mod translate` 子模块声明
- `proxy/handler.rs` 集成：needs_translation 条件分支 + 路径重写 + 流式/非流式/透传三路分叉
- `src-tauri/Cargo.toml` 新增：`bytes = "1"` 和 `futures = "0.3"` 两行显式声明
- 端到端集成测试：mock OpenAI 兼容上游，验证请求格式转换正确、响应格式转换正确、流式 SSE 事件序列正确、透传路径不受影响

**Implements:** handler 条件转换分支架构（Pattern 1 in ARCHITECTURE.md）

**Avoids（本 Phase 需验证的 Pitfall）:**
- 现有代理 body 流式透传与缓冲冲突（handler 架构分支，需显式设计）
- URL 路径重写（/v1/messages → /v1/chat/completions，含 /v1/v1 去重场景）
- 非流式大 body 缓冲（200MB 限制，大响应场景压力测试）
- "Looks Done But Isn't" checklist 8 项全部验证

### Phase Ordering Rationale

- **纯函数先行**：translate 模块的纯函数特性使其可独立实现和测试，无需启动代理服务器，单元测试密度高（参考 v2.0 的 221 个 lib tests 基准），在集成前发现绝大多数 bug
- **复杂性前置**：流式 SSE 转换是全项目最高复杂度的功能（Critical Pitfall 1、2、3 均在此），必须在 Phase 1 完全解决，不能推迟到集成阶段才发现问题
- **依赖约束**：Phase 2 的 handler 集成直接调用 Phase 1 的转换函数，依赖关系单向；Phase 1 内部三个模块相互独立，可并行开发
- **零回归保证**：Anthropic 透传路径（现有主路径）在两个阶段都完全不修改，Phase 2 只在 handler 内添加条件分支，不改变现有代码路径

### Research Flags

需要实现前精读参考代码的阶段：
- **Phase 1（stream.rs）**：流式 SSE 状态机是研究中置信度最高但实现复杂度最高的部分。建议实现前精读 cc-switch `streaming.rs` 第 280-347 行（工具调用 Deferred Start 逻辑）和 SSE buffer 跨 chunk 累积逻辑，理解后再自行实现（不照搬 typed struct 设计，改用 `serde_json::Value` 方式）。

可跳过 research-phase 直接实现的阶段：
- **Phase 1（request.rs + response.rs）**：字段映射逻辑已有 cc-switch transform.rs 775行完整参考，FEATURES.md 协议对照速查表枚举全部边界情况，直接按图索骥
- **Phase 2（handler 集成）**：ARCHITECTURE.md Pattern 1 已给出完整骨架代码，集成点列表清晰，按步骤执行即可

## Confidence Assessment

| 区域 | 置信度 | 说明 |
|------|--------|------|
| Stack | HIGH | Cargo.lock 直接验证 bytes 1.11.1、futures 0.3.32 已锁定；cc-switch 同款依赖交叉验证；无需引入任何新 crate |
| Features | HIGH | Anthropic/OpenAI 官方文档直接对比 + cc-switch 完整参考实现双重验证；协议格式对照速查表枚举全部边界情况 |
| Architecture | HIGH | 基于现有代码库直接分析（handler.rs、state.rs、provider.rs）；cc-switch 参考实现验证 translate 子模块设计；ARCHITECTURE.md 给出完整骨架代码 |
| Pitfalls | HIGH | cc-switch 代码审查发现实际 bug 模式 + 官方 API 规范对比；15 个 Pitfall 均有防御策略和具体代码示例 |

**Overall confidence:** HIGH

### Gaps to Address

- **SSE buffer 跨 chunk 分割**：PITFALLS.md 的 Performance Traps 指出 `\n\n` 分隔符被 chunk 截断是 SSE 解析的隐患。buffer 必须跨 chunk 累积到找到完整 `\n\n` 才解析。实现 stream.rs 时需明确处理此场景，并在测试中覆盖"chunk 在 `\n` 中间截断"的情况。

- **图片 base64 转换时机**：FEATURES.md 将图片转换列为 P2（v2.2.x），但 PITFALLS.md 的 Integration Gotchas 将其列为需要处理的转换点。建议在 Phase 1 的 request.rs 中实现 image block 转换函数（`image_url` data URI 字符串拼接），但不作为 v2.2 的门控条件——遇到 image block 时先丢弃并记录警告，后续按需启用。

- **`/v1/v1` 路径去重**：cc-switch `build_url` 中有针对用户配置的 `base_url` 已含 `/v1` 的去重逻辑。Phase 2 的端到端测试中需覆盖 `base_url = "https://api.openai.com/v1"` 的场景，确认路径重写结果正确。

## Sources

### Primary（HIGH confidence）
- `CLIManager/src-tauri/Cargo.lock` — bytes 1.11.1、futures 0.3.32 传递依赖锁定确认
- `CLIManager/src-tauri/Cargo.toml` — 现有依赖清单，无重复引入
- `CLIManager/src-tauri/src/proxy/handler.rs` — 现有请求管道步骤（A-J）分析
- `CLIManager/src-tauri/src/proxy/state.rs` — `UpstreamTarget.protocol_type` 字段已存在确认
- `CLIManager/src-tauri/src/provider.rs` — `ProtocolType` 枚举（Anthropic / OpenAiCompatible）确认
- `cc-switch/src-tauri/src/proxy/providers/transform.rs`（775行）— `anthropic_to_openai/openai_to_anthropic` 完整参考实现，含测试
- `cc-switch/src-tauri/src/proxy/providers/streaming.rs`（744行）— `create_anthropic_sse_stream` 完整参考实现，含多工具 + Deferred Start
- [Anthropic Messages API 官方文档](https://docs.anthropic.com/en/api/messages) — 请求/响应/SSE 格式规范
- [Anthropic Streaming Messages 官方文档](https://docs.anthropic.com/en/api/messages-streaming) — SSE 事件序列规范
- [OpenAI Chat Completions API 官方文档](https://platform.openai.com/docs/api-reference/chat/create) — 请求/响应/流式格式规范

### Secondary（HIGH confidence，交叉验证）
- `cc-switch/src-tauri/Cargo.toml` — bytes/futures 依赖版本选型交叉验证
- `cc-switch/src-tauri/src/proxy/providers/claude.rs` — `needs_transform` 判断逻辑 + URL 重写参考
- `cc-switch/src-tauri/src/proxy/thinking_rectifier.rs` — thinking block 处理场景深度分析

---
*Research completed: 2026-03-14*
*Ready for roadmap: yes*
