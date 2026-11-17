//! 通道运行实现与宿主能力注入。
use super::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_runtime::manager::message_response::*;
pub(crate) async fn handle_llm_result(
    ctx: &ChannelRuntimeContext,
    msg: traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
    route: &ChannelRouteSelection,
    llm_result: LlmExecutionResult,
    history_key: &str,
    history: &[ChatMessage],
    history_len_before_tools: usize,
    started_at: Instant,
    draft_message_id: Option<&str>,
    user_turn_content: &str,
    turn_canary_token: Option<&str>,
    reaction_done_emoji: &str,
    canary_guard: &crate::app::agent::security::CanaryGuard,
) { channel_runtime::manager::message_response::handle_llm_result(&super::runtime_host::HOST, ctx, msg, target_channel, route, llm_result, history_key, history, history_len_before_tools, started_at, draft_message_id, user_turn_content, turn_canary_token, reaction_done_emoji, canary_guard).await }
pub(crate) async fn mark_retryable_reply(
    snapshot: &ChannelMessageSnapshot,
    id: &str,
    msg: &traits::ChannelMessage,
    reply: String,
) { channel_runtime::manager::message_response::mark_retryable_reply(&super::runtime_host::HOST, snapshot, id, msg, reply).await }
pub(crate) async fn reconcile_channel_reply(
    snapshot: &ChannelMessageSnapshot,
    msg: &traits::ChannelMessage,
    channel: Option<&Arc<dyn Channel>>,
) -> bool { channel_runtime::manager::message_response::reconcile_channel_reply(&super::runtime_host::HOST, snapshot, msg, channel).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/message_response_tests.rs"]
mod message_response_tests;
