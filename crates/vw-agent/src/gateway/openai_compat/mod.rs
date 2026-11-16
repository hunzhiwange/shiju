//! OpenAI protocol host composition.
use super::AppState;
use axum::{extract::{State, ConnectInfo}, http::HeaderMap, response::IntoResponse};
use std::net::SocketAddr;
pub use agent_gateway::openai_compat::*;
pub async fn handle_v1_chat_completions(state: State<AppState>, peer: ConnectInfo<SocketAddr>, headers: HeaderMap, body: axum::body::Bytes) -> impl IntoResponse { agent_gateway::openai_compat::handle_v1_chat_completions(state, peer, headers, body, super::client_key_from_request).await }
pub async fn handle_v1_models(state: State<AppState>, headers: HeaderMap) -> impl IntoResponse { agent_gateway::openai_compat::handle_v1_models(state, headers).await }
#[cfg(test)]
#[path = "../../../../libs/agent-gateway/src/openai_compat/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../../libs/agent-gateway/src/openai_compat/mod_tests.rs"]
mod mod_tests;

#[cfg(test)]
use model_types::{ChatMessage, StreamOptions};
#[cfg(test)]
use axum::body::Body;
#[cfg(test)]
use axum::http::{StatusCode, header};
#[cfg(test)]
use axum::response::Json;
#[cfg(test)]
use futures_util::StreamExt;
#[cfg(test)]
use std::time::Instant;
#[cfg(test)]
use uuid::Uuid;
