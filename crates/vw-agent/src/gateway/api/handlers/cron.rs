//! Cron 路由的宿主组合；业务实现由 gateway-cron 提供。
use super::super::types::{CronAddBody, CronUpdateBody};
use crate::gateway::AppState;
use crate::session::ui_store::with_tenant_scope;
use axum::{Extension, extract::{State, Path}, http::{HeaderMap, StatusCode}, response::{IntoResponse, Json as JsonResponse}};
use vw_api_transport::id::TenantScope;

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

pub async fn handle_api_cron_run(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    Extension(tracker): Extension<std::sync::Arc<agent_security::policy::ActionTracker>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(error) = super::super::auth::require_auth(&state, &headers) {
        return error.into_response();
    }
    let config = std::sync::Arc::new(state.config.lock().clone());
    let security = std::sync::Arc::new(agent_security::SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        &config.workspace_dir,
    ));
    let tool = crate::tools::CronRunTool::new(config, security, crate::tools::cron_run::execute_job_now);
    match tool.prepare_background_run(&id, &tracker) {
        Ok(run) => {
            tokio::spawn(with_tenant_scope(tenant, run));
            (StatusCode::ACCEPTED, JsonResponse(serde_json::json!({"status": "accepted"}))).into_response()
        }
        Err(error) => (
            StatusCode::BAD_REQUEST,
            JsonResponse(serde_json::json!({"error": error})),
        ).into_response(),
    }
}

pub async fn handle_api_cron_update(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(id): Path<String>,
    JsonResponse(body): JsonResponse<CronUpdateBody>,
) -> impl IntoResponse {
    with_tenant_scope(
        tenant,
        gateway_cron::handlers::handle_api_cron_update(State(cron_state(state)), headers, Path(id), JsonResponse(body)),
    ).await
}

pub async fn handle_api_cron_add(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    JsonResponse(body): JsonResponse<CronAddBody>,
) -> impl IntoResponse {
    with_tenant_scope(
        tenant,
        gateway_cron::handlers::handle_api_cron_add(State(cron_state(state)), headers, JsonResponse(body)),
    ).await
}

pub async fn handle_api_cron_delete(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    gateway_cron::handlers::handle_api_cron_delete(State(cron_state(state)), headers, Path(id)).await
}

#[cfg(test)]
#[path = "cron_tests.rs"]
mod cron_tests;

/// Managing the notification recipient is a gateway operation scoped to the authenticated account.
#[derive(serde::Deserialize)]
pub struct WeComBindingQuery {
    instance: Option<String>,
}

pub async fn handle_api_cron_wecom_binding(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    method: axum::http::Method,
    axum::extract::Query(query): axum::extract::Query<WeComBindingQuery>,
) -> impl IntoResponse {
    if let Err(error) = super::super::auth::require_auth(&state, &headers) {
        return error.into_response();
    }
    let config = state.config.lock().clone();
    let result = with_tenant_scope(tenant, async {
        use crate::channels::wecom::runtime::{
            connected_with_credentials, cron_binding_pending, cron_recipient, forget_cron_binding, start_cron_binding,
        };
        use crate::cron::delivery::{cron_wecom_bot, cron_wecom_scope, latest_config};
        let id = query.instance.as_deref().unwrap_or(config_channel_services::WeComConfig::CRON_INSTANCE);
        let scope = cron_wecom_scope(&config, id)?;
        if method == axum::http::Method::DELETE {
            forget_cron_binding(&scope)?;
            return Ok::<_, anyhow::Error>(serde_json::json!({}));
        }
        let config = latest_config(&config).await?;
        let bot = cron_wecom_bot(&config, id)?;
        if method == axum::http::Method::POST {
            let pairing_code = start_cron_binding(&scope, &bot.bot_id, &bot.secret)?;
            return Ok(serde_json::json!({"pairing_code": pairing_code}));
        }
        let bound = cron_recipient(&scope, &bot.bot_id, &bot.secret)?.is_some();
        let connected = connected_with_credentials(&scope, &bot.bot_id, &bot.secret).is_ok();
        let pairing_pending = cron_binding_pending(&scope, &bot.bot_id, &bot.secret);
        Ok(serde_json::json!({"bound": bound, "connected": connected, "pairing_pending": pairing_pending}))
    })
    .await;
    match result {
        Ok(status) => JsonResponse(status).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            JsonResponse(serde_json::json!({"error": error.to_string()})),
        )
            .into_response(),
    }
}
