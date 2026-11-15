//! Composition with the scoped MCP execution host.
use crate::gateway::AppState;
use axum::{extract::State, http::HeaderMap, Json, response::IntoResponse};
pub use gateway_mcp::server::mcp::AppToolCall;
use super::mcp_host::Host;
pub async fn handle_api_mcp_inspect(state: State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    gateway_mcp::server::mcp::handle_api_mcp_inspect::<Host, _, _, _>(state, headers).await
}
pub async fn handle_api_mcp_app_call(state: State<AppState>, headers: HeaderMap, body: Json<AppToolCall>) -> impl IntoResponse {
    gateway_mcp::server::mcp::handle_api_mcp_app_call::<Host, _, _, _>(state, headers, body).await
}
#[cfg(test)]
#[path = "../../../../../libs/gateway-mcp/src/host_tests/mcp_tests.rs"]
mod mcp_tests;
