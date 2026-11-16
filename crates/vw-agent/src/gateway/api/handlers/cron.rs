//! Cron 路由的宿主组合；业务实现由 gateway-cron 提供。
use super::super::types::{CronAddBody, CronUpdateBody};
use crate::gateway::AppState;
use axum::{extract::{State, Path}, http::HeaderMap, response::{IntoResponse, Json as JsonResponse}};

fn cron_state(state: AppState) -> gateway_cron::state::CronState {
    gateway_cron::state::CronState {
        config: state.config.clone(),
        authenticate: std::sync::Arc::new(move |headers| super::super::auth::require_auth(&state, headers)),
        delivery: std::sync::Arc::new(crate::cron::scheduler::Host),
    }
}

pub async fn handle_api_cron_preview(State(state): State<AppState>, headers: HeaderMap, JsonResponse(body): JsonResponse<CronAddBody>) -> impl IntoResponse {
    gateway_cron::handlers::handle_api_cron_preview(State(cron_state(state)), headers, JsonResponse(body)).await
}

pub async fn handle_api_cron_list(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    gateway_cron::handlers::handle_api_cron_list(State(cron_state(state)), headers).await
}

pub async fn handle_api_cron_history(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    gateway_cron::handlers::handle_api_cron_history(State(cron_state(state)), headers).await
}

pub async fn handle_api_cron_runs(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    gateway_cron::handlers::handle_api_cron_runs(State(cron_state(state)), headers, Path(id)).await
}

pub async fn handle_api_cron_update(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>, JsonResponse(body): JsonResponse<CronUpdateBody>) -> impl IntoResponse {
    gateway_cron::handlers::handle_api_cron_update(State(cron_state(state)), headers, Path(id), JsonResponse(body)).await
}

pub async fn handle_api_cron_add(State(state): State<AppState>, headers: HeaderMap, JsonResponse(body): JsonResponse<CronAddBody>) -> impl IntoResponse {
    gateway_cron::handlers::handle_api_cron_add(State(cron_state(state)), headers, JsonResponse(body)).await
}

pub async fn handle_api_cron_delete(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    gateway_cron::handlers::handle_api_cron_delete(State(cron_state(state)), headers, Path(id)).await
}

#[cfg(test)]
#[path = "cron_tests.rs"]
mod cron_tests;
