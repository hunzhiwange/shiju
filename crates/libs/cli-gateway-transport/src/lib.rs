//! Gateway 连接、会话上下文和流事件传输边界。

pub mod gateway;
pub mod stream_adapter;
pub mod interaction;
mod session_store;

#[cfg(test)]
mod stream_adapter_tests;
