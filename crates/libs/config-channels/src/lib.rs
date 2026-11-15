//! 渠道实例组合与执行绑定配置。

pub mod chat;
pub use chat::*;

#[cfg(test)]
#[path = "chat_tests.rs"]
mod channel_project_binding_tests;

#[cfg(test)]
mod wecom_tests;
