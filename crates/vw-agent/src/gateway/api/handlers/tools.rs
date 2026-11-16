//! Tool routes assembled with the existing host authentication and diagnostics.

use crate::app::agent::gateway::AppState;
use axum::{
    extract::State, http::HeaderMap, response::IntoResponse,
};

pub async fn handle_api_tools(state: State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    agent_gateway::api::tools::handle_api_tools(state, headers).await
}

pub async fn handle_api_doctor(state: State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    agent_gateway::api::tools::handle_api_doctor(state, headers).await
}
