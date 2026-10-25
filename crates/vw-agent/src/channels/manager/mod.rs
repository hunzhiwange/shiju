//! 通道运行时管理模块的聚合入口。
//!
//! 该模块把配置、监听、消息处理、模型路由、运行时命令和回复发送等子职责
//! 拆到独立文件中，并在这里重新导出给通道启动流程使用。

use super::*;

mod channels_config;
mod command;
mod dispatch;
mod errors;
mod health;
mod listener;
mod message;
mod message_execution;
mod message_helpers;
mod message_response;
mod message_result;
mod provider;
mod reload;
mod start;
mod telegram;
mod typing;

pub(crate) use channels_config::*;
pub use command::*;
pub(crate) use dispatch::*;
pub(crate) use errors::*;
pub use health::*;
pub(crate) use listener::*;
pub(crate) use message::*;
pub(crate) use message_execution::*;
pub(crate) use message_helpers::*;
pub(crate) use message_response::*;
pub(crate) use message_result::*;
pub(crate) use provider::*;
pub use start::*;
pub(crate) use telegram::*;
pub(crate) use typing::*;

#[cfg(test)]
mod tests;
