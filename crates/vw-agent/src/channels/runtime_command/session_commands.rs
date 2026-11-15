//! 运行时会话命令处理模块。
use super::super::*;
pub(crate) async fn handle_set_provider(
    ctx: &ChannelRuntimeContext,
    sender_key: &str,
    current: &mut ChannelRouteSelection,
    raw_provider: String,
) -> String { let _ = super::super::manager::command_host(); channel_runtime::runtime_command::session_commands::handle_set_provider(ctx, sender_key, current, raw_provider).await }
pub(crate) fn handle_show_model(current: &ChannelRouteSelection, workspace_dir: &Path) -> String { channel_runtime::runtime_command::session_commands::handle_show_model(current, workspace_dir) }
pub(crate) async fn handle_set_model(
    ctx: &ChannelRuntimeContext,
    sender_key: &str,
    current: &mut ChannelRouteSelection,
    raw_model: String,
) -> String { let _ = super::super::manager::command_host(); channel_runtime::runtime_command::session_commands::handle_set_model(ctx, sender_key, current, raw_model).await }
pub(crate) async fn handle_clear_session(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    sender_key: &str,
) -> String { channel_runtime::runtime_command::session_commands::handle_clear_session(ctx, msg, sender_key).await }
pub(crate) async fn handle_new_session(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    sender_key: &str,
) -> String { channel_runtime::runtime_command::session_commands::handle_new_session(super::super::manager::command_host(), ctx, msg, sender_key).await }
pub(crate) async fn handle_task_mode(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    sender_key: &str,
) -> String { channel_runtime::runtime_command::session_commands::handle_task_mode(ctx, msg, sender_key).await }
pub(crate) async fn handle_models(current: &ChannelRouteSelection) -> String { let _ = super::super::manager::command_host(); channel_runtime::runtime_command::session_commands::handle_models(current).await }
pub(crate) fn effort_levels(model: &model_types::Model) -> Vec<String> { channel_runtime::runtime_command::session_commands::effort_levels(model) }
pub(crate) async fn handle_effort(
    ctx: &ChannelRuntimeContext,
    key: &str,
    current: &mut ChannelRouteSelection,
    level: &str,
) -> String { let _ = super::super::manager::command_host(); channel_runtime::runtime_command::session_commands::handle_effort(ctx, key, current, level).await }
pub(crate) async fn handle_status(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    current: &ChannelRouteSelection,
) -> String { channel_runtime::runtime_command::session_commands::handle_status(super::super::manager::command_host(), ctx, msg, current).await }
#[cfg(test)]
pub(crate) use channel_runtime::runtime_command::session_commands::with_test_provider_init_error;
