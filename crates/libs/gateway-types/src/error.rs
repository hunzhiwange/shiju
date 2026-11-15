//! Gateway HTTP API 的统一错误类型。
//!
//! 本模块把内部错误映射成稳定的 HTTP 状态码和 JSON 错误体，避免各个处理器
//! 重复拼装响应，也避免把底层错误结构直接暴露给客户端。

pub use gateway_error::ApiError;

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
