//! 外部消息通道的 Webhook 处理器，统一校验、对话执行与回复投递。
//!
//! 各提供者先完成签名或令牌验证，再把消息规范化为通道模型；失败响应不进入代理执行。

#[cfg(test)]
use super::chat::run_gateway_chat_with_tools;
use super::state::AppState;
#[cfg(test)]
use super::util::{
    linq_memory_key, nextcloud_talk_memory_key, qq_memory_key, wati_memory_key, whatsapp_memory_key,
};
#[cfg(test)]
use channel_types::ChannelMessage;

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::HeaderMap,
    response::Response,
};

#[cfg(test)]
use agent_gateway::channel_reply_fixture::{GatewayChatFuture, process_channel_messages_with_chat, RecordingChannel, channel_message};

#[cfg(test)]
fn run_gateway_chat_with_tools_boxed<'a>(
    state: &'a AppState,
    message: &'a str,
    key: &'a str,
) -> GatewayChatFuture<'a> {
    Box::pin(run_gateway_chat_with_tools(state, message, key))
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
