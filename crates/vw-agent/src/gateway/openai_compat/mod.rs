//! OpenAI protocol host composition.
use super::AppState;
use axum::{extract::{State, ConnectInfo}, http::HeaderMap, response::IntoResponse};
use std::net::SocketAddr;
#[cfg(any(test, target_arch = "wasm32"))]
pub use agent_gateway::openai_compat::*;
pub async fn handle_v1_chat_completions(state: State<AppState>, peer: ConnectInfo<SocketAddr>, headers: HeaderMap, body: axum::body::Bytes) -> impl IntoResponse { agent_gateway::openai_compat::handle_v1_chat_completions(state, peer, headers, body, super::client_key_from_request).await }
pub async fn handle_v1_models(state: State<AppState>, headers: HeaderMap) -> impl IntoResponse { agent_gateway::openai_compat::handle_v1_models(state, headers).await }
