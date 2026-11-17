//! 通道任务模式处理模块。
use super::super::*;
pub(crate) async fn handle_task_mode_message_if_needed(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    sender_key: &str,
    target_channel: Option<&Arc<dyn Channel>>,
) -> bool { channel_runtime::runtime_command::task_mode::handle_task_mode_message_if_needed(super::super::manager::command_host(), ctx, msg, sender_key, target_channel).await }
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_runtime::runtime_command::task_mode::{set_sender_task_mode};
