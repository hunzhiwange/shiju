//! Explicit read permission is checked before resolving any host path.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use std::path::PathBuf;
use vw_api_types::{
    agent_apps::AppId, app_providers::paths::ProviderPathsRequest, id::TenantScope,
};
#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/{app}/config/paths", post(read).layer(DefaultBodyLimit::max(4096)))
        .layer(Extension(PolicyFile(path)))
}
async fn read(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<ProviderPathsRequest>, JsonRejection>,
) -> Result<Response, StatusCode> {
    let Json(request) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        let home = super::super::agent_apps::provider_config_home(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
        )?;
        let environment = vw_agent_apps::providers::paths::execution_environment();
        vw_agent_apps::providers::paths::read(
            &home,
            app,
            request.directory_override.as_deref(),
            &environment,
        )
        .map(|value| Json(value).into_response())
        .map_err(|_| StatusCode::BAD_REQUEST)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
