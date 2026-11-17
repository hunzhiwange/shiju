//! 通道运行时管理模块的聚合入口。
//!
//! 该模块把配置、监听、消息处理、模型路由、运行时命令和回复发送等子职责
//! 拆到独立文件中，并在这里重新导出给通道启动流程使用。

use super::*;

mod channels_config;
mod command;
mod dispatch;
mod errors;
mod execution_binding;
mod execution_control;
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

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channels_config::*;
pub use command::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use dispatch::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use errors::*;
pub(crate) use execution_control::*;
pub use health::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use listener::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use message_execution::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use message_helpers::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use message_response::*;
pub(crate) use message_result::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use provider::*;
pub use start::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use telegram::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use typing::*;

#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/tests.rs"]
mod tests;

mod runtime_host;
pub(crate) fn command_host() -> &'static dyn channel_runtime::command_host::CommandHost { crate::provider::provider::ensure_host(); &runtime_host::HOST }

#[cfg(test)]
pub(crate) fn contract_host() -> &'static dyn channel_runtime::host::ChannelHost { &runtime_host::HOST }
