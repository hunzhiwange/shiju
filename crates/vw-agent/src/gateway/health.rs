//! Host bindings for gateway HTTP services.
use super::state::AppState;
#[cfg(any(test, target_arch = "wasm32"))]
pub use agent_gateway::health::*;
pub async fn handle_health(state: axum::extract::State<AppState>) -> impl axum::response::IntoResponse { agent_gateway::health::handle_health(state).await }
pub async fn handle_metrics(state: axum::extract::State<AppState>, peer: axum::extract::ConnectInfo<std::net::SocketAddr>, headers: axum::http::HeaderMap) -> axum::response::Response { agent_gateway::health::handle_metrics(state, peer, headers).await }
