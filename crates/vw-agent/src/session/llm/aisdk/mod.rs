//! AI SDK 后端适配模块。
//!
//! 该目录负责把 VibeWindow 的 OpenAI 风格消息、工具调用和流式响应转换为 `aisdk`
//! 运行时能够理解的请求与事件。子模块按职责拆分：请求执行、流式解析、错误映射和
//! 通用转换工具分别维护，避免把传输与消息转换混在一个文件中。

mod convert;
mod driver;
mod error;
mod request;
#[cfg(not(target_arch = "wasm32"))]
mod reasoning;
mod stream;
mod util;

/// 执行 AI SDK 流式请求的公开入口。
pub use request::do_stream_request_aisdk;
