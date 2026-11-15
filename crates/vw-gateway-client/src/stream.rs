//! 聊天流兼容门面，领域实现由 gateway-chat-stream 持有。
pub use gateway_chat_stream::*;

#[cfg(test)]
#[path = "stream_tests.rs"]
mod stream_tests;
