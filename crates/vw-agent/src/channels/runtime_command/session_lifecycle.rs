//! 当前聊天和项目的会话选择、恢复与归档；所有权校验先于绑定替换。
use super::super::*;
pub(crate) async fn handle_session_lifecycle_command(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    command: ChannelRuntimeCommand,
) -> String { channel_runtime::runtime_command::session_lifecycle::handle_session_lifecycle_command(super::super::manager::command_host(), ctx, msg, command).await }
pub(crate) use channel_runtime::runtime_command::session_lifecycle::{invalidate_archive};
pub(crate) use channel_runtime::manager::{persist_current_route, is_session_lifecycle_command};
