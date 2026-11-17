//! 通道运行实现与宿主能力注入。
#[cfg(any(test, target_arch = "wasm32"))]
use super::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_runtime::manager::message_helpers::*;
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/message_helpers_tests.rs"]
mod message_helpers_tests;
