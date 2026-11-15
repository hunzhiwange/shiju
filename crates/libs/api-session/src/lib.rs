//! 会话管理协议与网关请求响应。

pub mod session;
pub use session::*;

#[cfg(test)]
mod session_tests;
