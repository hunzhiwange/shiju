//! Concrete query-engine adapter for webhook ingress.
use super::{AppState, WebhookBody};
use axum::{Json, http::{HeaderMap, StatusCode}, extract::rejection::JsonRejection};
use std::net::SocketAddr;
use serde_json::Value;
#[cfg(test)]
use super::util::hash_webhook_secret;
#[cfg(test)]
use agent_gateway::webhook_ingress::*;
pub(super) async fn handle_webhook_inner(state: AppState, peer: SocketAddr, headers: HeaderMap, body: Result<Json<WebhookBody>, JsonRejection>) -> (StatusCode, Json<serde_json::Value>) {
    agent_gateway::webhook_ingress::handle_webhook_inner(state, peer, headers, body, |state, message| Box::pin(super::chat::run_gateway_chat_simple(state, message))).await
}
#[cfg(test)]
#[path = "../../../../libs/agent-gateway/src/webhook_ingress/tests.rs"]
mod tests;
