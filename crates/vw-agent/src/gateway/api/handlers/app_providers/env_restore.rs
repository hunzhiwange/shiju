//! Restore requires host-config write permission before receipt lookup or file access.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use std::path::PathBuf;
use vw_api_types::{
    agent_apps::AppId, app_providers::env_restore::EnvRestoreRequest, id::TenantScope,
};
#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route(
            "/app-providers/{app}/environment/restore",
            post(restore).layer(DefaultBodyLimit::max(64 * 1024)),
        )
        .layer(Extension(PolicyFile(path)))
}
async fn restore(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<EnvRestoreRequest>, JsonRejection>,
) -> Result<Response, StatusCode> {
    let Json(request) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        let (home, private_root) = super::super::agent_apps::provider_editor_context(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
            true,
        )?;
        vw_agent_apps::providers::env_restore::restore(
            &home,
            &private_root,
            app,
            &super::env_clean::scope(&tenant, &request.target_host_id)?,
            &request.backup_id,
        )
        .map(|value| Json(value).into_response())
        .map_err(|message| match message {
            "unsupported environment application" | "unsupported environment platform" => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            "invalid environment backup identifier" => StatusCode::BAD_REQUEST,
            "environment backup changed" => StatusCode::CONFLICT,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
