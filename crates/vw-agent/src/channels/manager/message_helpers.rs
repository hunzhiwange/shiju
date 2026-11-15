//! 通道运行实现与宿主能力注入。
use super::*;
pub(crate) use channel_runtime::manager::message_helpers::*;
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/message_helpers_tests.rs"]
mod message_helpers_tests;
