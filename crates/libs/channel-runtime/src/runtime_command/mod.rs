//! 通道命令的权限、分派和会话状态转换。
use std::{collections::HashMap, sync::{Arc, Mutex, OnceLock}, path::{Path, PathBuf}, fmt::Write};
use anyhow::{Context, Result};
use channel_types::{self as traits, Channel, SendMessage};
use channel_support::{runtime_state::ChannelRuntimeContext, config::*, routing::*, scope::*, session::{sender_session_store, channel_session_ownership, ChannelSessionOwnership}};
use agent_config::runtime_schema::{Config, NonCliNaturalLanguageApprovalMode};
use agent_approval::{ApprovalResponse, PendingApprovalError};
use agent_observability::runtime_trace;
use model_types::ChatMessage;
use agent_provider::provider;
use crate::manager::{provider::get_or_create_provider, execution_control::{active_channel_task, stop_channel_task, send_channel_busy}};
use crate::command_host::{create_channel_session, owned_channel_sessions};
pub use crate::manager::runtime_command as command;
pub use crate::manager::catalog;
use command::*;
pub mod approval_commands;
pub mod approval_config;
pub mod handler;
pub mod history_commands;
pub mod project_commands;
pub mod session_commands;
pub mod session_lifecycle;
pub mod task_mode;
use approval_config::*;
const APPROVAL_ALL_TOOLS_ONCE_TOKEN: &str = "__all_tools_once__";
fn is_non_cli_tool_excluded(ctx: &ChannelRuntimeContext, name: &str) -> bool {
    snapshot_non_cli_excluded_tools(ctx).iter().any(|tool| tool == name)
}
fn channel_chat_gate(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> Arc<tokio::sync::RwLock<()>> {
    channel_support::scope::channel_chat_gate(&ctx.project_state, msg)
}
async fn resolve_channel_message_snapshot(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> Result<ChannelMessageSnapshot> {
    channel_support::scope::resolve_channel_message_snapshot(&ctx.project_state, ctx.message_snapshot.as_ref(), &ctx.workspace_dir, msg).await
}
async fn clear_sender_session_id(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> Result<()> {
    let snapshot = resolve_channel_message_snapshot(ctx, msg).await?;
    channel_support::session::clear_sender_session_scope(&snapshot.scope);
    Ok(())
}

fn clear_sender_history(ctx: &ChannelRuntimeContext, key: &str) {
    channel_support::history::clear_sender_history(&ctx.conversation_histories, key)
}
