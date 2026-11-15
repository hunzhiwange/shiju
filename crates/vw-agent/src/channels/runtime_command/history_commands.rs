//! 真实消息锚点驱动的历史操作；调用方持有聊天写锁和租户/项目上下文。
use super::super::*;
pub(crate) async fn handle_history_command(
    ctx: &ChannelRuntimeContext,
    command: ChannelRuntimeCommand,
) -> Result<String> { channel_runtime::runtime_command::history_commands::handle_history_command(super::super::manager::command_host(), ctx, command).await }
pub(crate) use channel_runtime::runtime_command::history_commands::{invalidate_history, cached_history};
