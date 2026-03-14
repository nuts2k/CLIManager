//! 协议转换模块：Anthropic Messages API <-> OpenAI Chat Completions / Responses API
//!
//! 六个子模块：
//! - request: Anthropic 请求 → OpenAI Chat Completions 请求（anthropic_to_openai）
//! - response: OpenAI Chat Completions 非流式响应 → Anthropic 响应（openai_to_anthropic）
//! - stream: OpenAI Chat Completions SSE 流 → Anthropic SSE 流（create_anthropic_sse_stream）
//! - responses_request: Anthropic 请求 → OpenAI Responses API 请求（anthropic_to_responses）
//! - responses_response: OpenAI Responses API 非流式响应 → Anthropic 响应（responses_to_anthropic）
//! - responses_stream: OpenAI Responses API SSE 流 → Anthropic SSE 流（create_responses_anthropic_sse_stream）

pub mod request;
pub mod response;
pub mod responses_request;
pub mod responses_response;
pub mod responses_stream;
pub mod stream;
