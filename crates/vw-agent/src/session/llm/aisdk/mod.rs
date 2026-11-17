//! AI SDK 后端适配模块。
//!
//! 该目录负责把 VibeWindow 的 OpenAI 风格消息、工具调用和流式响应转换为 `aisdk`
//! 运行时能够理解的请求与事件。子模块按职责拆分：请求执行、流式解析、错误映射和
//! 通用转换工具分别维护，避免把传输与消息转换混在一个文件中。

pub(super) mod convert;
pub(crate) mod host;
#[cfg(test)]
mod error;
#[cfg(test)]
mod reasoning;
mod request;
#[cfg(test)]
mod stream;

/// 执行 AI SDK 流式请求的公开入口。
#[cfg(any(test, target_arch = "wasm32"))]
pub use request::do_stream_request_aisdk;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use llm_runtime::aisdk::driver::validate_catalog_request;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../../libs/llm-runtime/src/aisdk/host_tests/native_search_acceptance_tests.rs"]
mod native_search_acceptance_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../../libs/llm-runtime/src/aisdk/host_tests/model_catalog_tests.rs"]
mod model_catalog_tests;
