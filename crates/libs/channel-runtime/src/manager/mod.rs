//! 通道运行协作模块。
use std::{sync::Arc, time::Duration};
use anyhow::Result;
use channel_types::{self as traits, Channel, SendMessage};
use channel_support::session_events::ChannelProcessorResponse;
use tokio_util::sync::CancellationToken;
use crate::ConfiguredChannel;
const CHANNEL_HEALTH_HEARTBEAT_SECS: u64 = 30;
const CHANNEL_TYPING_REFRESH_INTERVAL_SECS: u64 = 4;
pub mod listener;
pub mod typing;
pub mod message_result;

use anyhow::Context;
use model_types::Provider;
use channel_support::{runtime_state::ChannelRuntimeContext, config::runtime_defaults_snapshot};
pub mod provider;
pub mod errors;

use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};
use model_types::ChatMessage;
use agent_config::runtime_schema::Config;
use agent_config::runtime_schema::apply_env_overrides;
use agent_memory::MemoryCategory;
use agent_observability::runtime_trace;
use agent_util::truncate_with_ellipsis;
use channel_support::{config::*, prompt::*, routing::*, scope::*, session::sender_session_store};
use crate::ChannelCommands;
use errors::*;
use execution_control::*;
use dispatch::*;
use message_helpers::*;
use message_execution::*;
use message_response::*;
use message_result::*;
use provider::*;
use runtime_command::*;
use typing::*;
use channels_config::*;
use telegram::*;
const CHANNEL_PARALLELISM_PER_CHANNEL: usize = 4;
const CHANNEL_MIN_IN_FLIGHT_MESSAGES: usize = 8;
const CHANNEL_MAX_IN_FLIGHT_MESSAGES: usize = 64;
const AUTOSAVE_MIN_MESSAGE_CHARS: usize = 20;
const CHANNEL_HOOK_MAX_OUTBOUND_CHARS: usize = 20_000;
const APPROVAL_ALL_TOOLS_ONCE_TOKEN: &str = "__all_tools_once__";
pub mod runtime_command;
pub mod catalog;
pub mod dispatch;
pub mod execution_control;
pub mod execution_binding;
pub mod message;
pub mod message_execution;
pub mod message_helpers;
pub mod message_response;
pub mod channels_config;
pub mod command;
pub mod health;
pub mod telegram;
pub mod start;
pub mod reload;
fn channel_chat_gate(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> Arc<tokio::sync::RwLock<()>> {
    channel_support::scope::channel_chat_gate(&ctx.project_state, msg)
}
async fn resolve_channel_message_snapshot(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> Result<ChannelMessageSnapshot> {
    channel_support::scope::resolve_channel_message_snapshot(&ctx.project_state, ctx.message_snapshot.as_ref(), &ctx.workspace_dir, msg).await
}
fn append_sender_turn(ctx: &ChannelRuntimeContext, key: &str, turn: ChatMessage) {
    channel_support::history::append_sender_turn(&ctx.conversation_histories, key, turn)
}
fn rollback_orphan_user_turn(ctx: &ChannelRuntimeContext, key: &str, content: &str) -> bool {
    channel_support::history::rollback_orphan_user_turn(&ctx.conversation_histories, key, content)
}
fn interruption_scope_key(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> String {
    channel_support::history::conversation_history_key(msg, channel_chat_scope(ctx, msg).tenant)
}
pub fn is_session_lifecycle_command(command: &ChannelRuntimeCommand) -> bool {
    matches!(command, ChannelRuntimeCommand::NewSession | ChannelRuntimeCommand::ClearSession | ChannelRuntimeCommand::ListSessions | ChannelRuntimeCommand::ResumeSession(_) | ChannelRuntimeCommand::ContinueSession | ChannelRuntimeCommand::RenameSession(_) | ChannelRuntimeCommand::ArchiveSession(_) | ChannelRuntimeCommand::ForkSession(_) | ChannelRuntimeCommand::CompactSession(_) | ChannelRuntimeCommand::ResetSession(_))
}
#[cfg(not(target_arch="wasm32"))]
use channel_telegram::TelegramChannel;
#[cfg(not(target_arch="wasm32"))]
use channel_discord::DiscordChannel;
#[cfg(not(target_arch="wasm32"))]
use channel_mattermost::MattermostChannel;
#[cfg(not(target_arch="wasm32"))]
use channel_signal::SignalChannel;
#[cfg(not(target_arch="wasm32"))]
use channel_whatsapp::WhatsAppChannel;
#[cfg(not(target_arch="wasm32"))]
use channel_wati::WatiChannel;
#[cfg(not(target_arch="wasm32"))]
use channel_nextcloud_talk::NextcloudTalkChannel;
#[cfg(not(target_arch="wasm32"))]
use channel_nostr::NostrChannel;
#[cfg(not(target_arch="wasm32"))]
use channel_dingtalk::DingTalkChannel;
#[cfg(not(target_arch="wasm32"))]
use channel_qq::QQChannel;
#[cfg(feature="channel-lark")]
use channel_lark::LarkChannel;
#[cfg(all(feature="whatsapp-web", not(target_arch="wasm32")))]
use channel_whatsapp_web::WhatsAppWebChannel;

use channel_support::format::{should_expose_internal_tool_details, extract_tool_context_summary};
use channel_support::session_events::to_session_history;
use crate::ChannelHealthState;

use crate::format::sanitize_channel_response;

use channel_support::session::{channel_session_ownership, save_channel_session_route};
pub async fn persist_current_route(
    ctx: &ChannelRuntimeContext,
    route: &ChannelRouteSelection,
) -> Result<()> {
    let Some(snapshot) = &ctx.message_snapshot else { return Ok(()) };
    let id = sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&snapshot.scope)
        .cloned();
    if let Some(id) = id {
        match channel_session_ownership(snapshot, &id).await {
            Ok(_) => save_channel_session_route(snapshot, &id, route).await?,
            Err(err)
                if err.chain().any(|cause| {
                    matches!(
                        cause.downcast_ref::<agent_storage::Error>(),
                        Some(agent_storage::Error::NotFound(_))
                    )
                }) => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod channels_config_tests;
