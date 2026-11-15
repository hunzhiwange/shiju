//! 运行时命令的执行入口。
use super::super::*;
pub(crate) async fn handle_runtime_command_if_needed(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
) -> bool { channel_runtime::runtime_command::handler::handle_runtime_command_if_needed(super::super::manager::command_host(), ctx, msg, target_channel).await }
