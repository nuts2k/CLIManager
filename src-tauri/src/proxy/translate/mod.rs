//! 协议转换模块：Anthropic Messages API <-> OpenAI Chat Completions API
//!
//! 三个子模块：
//! - request: Anthropic 请求 → OpenAI 请求（anthropic_to_openai）
//! - response: OpenAI 非流式响应 → Anthropic 响应（openai_to_anthropic）
//! - stream: OpenAI SSE 流 → Anthropic SSE 流（create_anthropic_sse_stream）

pub mod request;
pub mod response;
pub mod stream;
