//! 聊天消息、流请求与事件协议。

pub mod chat;
pub use chat::*;

#[cfg(test)]
mod chat_tests;
