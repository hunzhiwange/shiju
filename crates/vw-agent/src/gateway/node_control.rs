//! Host bindings for gateway HTTP services.
use super::state::AppState;
pub use agent_gateway::node_control::*;
pub async fn handle_node_control(state: axum::extract::State<AppState>, headers: axum::http::HeaderMap, body: Result<axum::Json<NodeControlRequest>, axum::extract::rejection::JsonRejection>) -> axum::response::Response { agent_gateway::node_control::handle_node_control(state, headers, body).await }
