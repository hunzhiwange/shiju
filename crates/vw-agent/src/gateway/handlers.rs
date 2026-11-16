//! 外部消息通道的 Webhook 处理器，统一校验、对话执行与回复投递。
//!
//! 各提供者先完成签名或令牌验证，再把消息规范化为通道模型；失败响应不进入代理执行。

#[cfg(test)]
use super::chat::{log_channel_message, run_gateway_chat_with_tools, sanitize_gateway_response};
use super::state::AppState;
#[cfg(test)]
use super::util::{
    linq_memory_key, nextcloud_talk_memory_key, qq_memory_key, wati_memory_key, whatsapp_memory_key,
};
#[cfg(test)]
use channel_types::SendMessage;
#[cfg(test)]
use channel_types::{Channel, ChannelMessage};
#[cfg(test)]
use crate::app::agent::memory::MemoryCategory;

#[cfg(test)]
use crate::app::agent::tools::Tool;
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::HeaderMap,
    response::Response,
};
#[cfg(test)]
use std::future::Future;
#[cfg(test)]
use std::pin::Pin;

#[cfg(test)]
type GatewayChatFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>>;
#[cfg(test)]
type GatewayChatRunner = for<'a> fn(&'a AppState, &'a str, &'a str) -> GatewayChatFuture<'a>;

#[cfg(test)]
fn run_gateway_chat_with_tools_boxed<'a>(
    state: &'a AppState,
    message: &'a str,
    key: &'a str,
) -> GatewayChatFuture<'a> {
    Box::pin(run_gateway_chat_with_tools(state, message, key))
}

#[cfg(test)]
async fn process_channel_messages_with_chat<C, K>(
    state: &AppState,
    channel_name: &str,
    channel: &C,
    messages: &[ChannelMessage],
    memory_key: K,
    llm_error_label: &str,
    include_thread: bool,
    run_chat: GatewayChatRunner,
) where
    C: Channel + ?Sized,
    K: Fn(&ChannelMessage) -> String,
{
    for msg in messages {
        log_channel_message(channel_name, &msg.sender, &msg.content);

        let key = memory_key(msg);
        if state.auto_save {
            let _ = state.mem.store(&key, &msg.content, MemoryCategory::Conversation, None).await;
        }

        match run_chat(state, &msg.content, &key).await {
            Ok(response) => {
                send_successful_channel_reply(
                    channel_name,
                    channel,
                    msg,
                    &response,
                    state.tools_registry_exec.as_ref(),
                    include_thread,
                )
                .await;
            }
            Err(e) => {
                tracing::error!("LLM error for {llm_error_label}: {e:#}");
                let mut reply = SendMessage::new(
                    "Sorry, I couldn't process your message right now.",
                    &msg.reply_target,
                );
                if include_thread {
                    reply = reply.in_thread(msg.thread_ts.clone());
                }
                let _ = channel.send(&reply).await;
            }
        }
    }
}

#[cfg(test)]
async fn send_successful_channel_reply<C: Channel + ?Sized>(
    channel_name: &str,
    channel: &C,
    msg: &ChannelMessage,
    response: &str,
    tools: &[Box<dyn Tool>],
    include_thread: bool,
) {
    let safe_response = sanitize_gateway_response(response, tools);
    let mut reply = SendMessage::new(safe_response, &msg.reply_target);
    if include_thread {
        reply = reply.in_thread(msg.thread_ts.clone());
    }
    if let Err(e) = channel.send(&reply).await {
        tracing::error!("Failed to send {channel_name} reply: {e}");
    }
}

pub use agent_gateway::handlers::{WhatsAppVerifyQuery, WatiVerifyQuery, verify_whatsapp_signature};
#[cfg(test)]
impl agent_gateway::handlers::WhatsAppWebhook for crate::channels::WhatsAppChannel {
 fn verify_token(&self) -> &str { self.verify_token() }
 fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage> { self.parse_webhook_payload(payload) }
}
pub async fn handle_whatsapp_verify(State(state): State<AppState>, Query(params): Query<WhatsAppVerifyQuery>) -> Response { agent_gateway::handlers::handle_whatsapp_verify(State(state), Query(params)).await }
pub async fn handle_wati_verify(State(state): State<AppState>, Query(params): Query<WatiVerifyQuery>) -> Response { agent_gateway::handlers::handle_wati_verify(State(state), Query(params)).await }
pub async fn handle_whatsapp_message(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response { agent_gateway::handlers::handle_whatsapp_message(State(state), headers, body, super::channel_runtime::channel_runtime, |ctx, msg, cancellation| Box::pin(crate::channels::manager::process_channel_message(ctx, msg, cancellation))).await }
pub async fn handle_linq_webhook(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response { agent_gateway::handlers::handle_linq_webhook(State(state), headers, body, super::channel_runtime::channel_runtime, |ctx, msg, cancellation| Box::pin(crate::channels::manager::process_channel_message(ctx, msg, cancellation))).await }
pub async fn handle_nextcloud_talk_webhook(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response { agent_gateway::handlers::handle_nextcloud_talk_webhook(State(state), headers, body, super::channel_runtime::channel_runtime, |ctx, msg, cancellation| Box::pin(crate::channels::manager::process_channel_message(ctx, msg, cancellation))).await }
pub async fn handle_qq_webhook(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response { agent_gateway::handlers::handle_qq_webhook(State(state), headers, body, super::channel_runtime::channel_runtime, |ctx, msg, cancellation| Box::pin(crate::channels::manager::process_channel_message(ctx, msg, cancellation))).await }
pub async fn handle_wati_webhook(State(state): State<AppState>, body: Bytes) -> Response { agent_gateway::handlers::handle_wati_webhook(State(state), body, super::channel_runtime::channel_runtime, |ctx, msg, cancellation| Box::pin(crate::channels::manager::process_channel_message(ctx, msg, cancellation))).await }
#[cfg(test)]
#[path = "../../../libs/agent-gateway/src/handlers_tests.rs"]
mod handlers_tests;

#[cfg(test)]
use axum::{http::StatusCode, response::{IntoResponse, Json}};
