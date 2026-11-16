//! LLM 会话日志记录模块
//!
//! 提供 LLM 会话专用的日志记录器，用于记录 LLM 交互过程中的各种事件和信息。
//! 日志记录器会自动添加 `service: llm` 标签，便于在日志流中识别和过滤 LLM 相关的日志条目。

pub use llm_runtime::logging::LOGGER;
#[cfg(test)]
#[path = "logging_tests.rs"]
mod logging_tests;
