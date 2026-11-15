//! 状态路由组合：注入宿主现有 skey 认证边界。
use crate::app::agent::gateway::AppState;
use axum::{extract::State, http::HeaderMap, response::IntoResponse};

pub async fn handle_api_status(state: State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    gateway_status::handle_api_status(state, headers, super::super::auth::require_auth).await
}

pub async fn handle_api_health(state: State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    gateway_status::handle_api_health(state, headers, super::super::auth::require_auth).await
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod status_tests;
