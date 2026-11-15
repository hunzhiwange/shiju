//! HTTP agent host bindings.
use super::{state::AppState, types::AgentBody};
use super::chat::run_gateway_chat_with_tools;
use axum::{extract::{ConnectInfo, State}, http::{HeaderMap, StatusCode}, response::{Json, Response}};
use std::net::SocketAddr;
struct Host;
impl agent_gateway::agent::AgentHost<super::state::SessionQueryEngineStore, super::state::GatewayChannels, crate::hooks::HookRuntime> for Host {
    fn client_key_from_request(peer: Option<SocketAddr>, headers: &HeaderMap, trust: bool) -> String {
        super::util::client_key_from_request(peer, headers, trust)
    }
    fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
        super::api::auth::require_auth(state, headers)
    }
    fn sanitize_gateway_response(response: &str, tools: &[Box<dyn tool_types::Tool>]) -> String {
        super::chat::sanitize_gateway_response(response, tools)
    }
    fn run_agent_chat<'a>(state: &'a AppState, message: &'a str, key: &'a str) -> futures_util::future::BoxFuture<'a, anyhow::Result<String>> {
        Box::pin(run_agent_chat(state, message, key))
    }
}
pub async fn handle_agent(state: State<AppState>, peer: ConnectInfo<SocketAddr>, headers: HeaderMap,
    body: Result<Json<AgentBody>, axum::extract::rejection::JsonRejection>) -> Response {
    agent_gateway::agent::handle_agent::<_, _, _, Host>(state, peer, headers, body).await
}
#[cfg(not(test))]
async fn run_agent_chat(state: &AppState, message: &str, key: &str) -> anyhow::Result<String> {
    run_gateway_chat_with_tools(state, message, key).await
}

#[cfg(test)]
async fn run_agent_chat(state: &AppState, message: &str, key: &str) -> anyhow::Result<String> {
    if state.config.lock().default_provider.as_deref() == Some("static") {
        return super::chat::run_gateway_chat_simple(state, message).await;
    }
    run_gateway_chat_with_tools(state, message, key).await
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod agent_tests;
