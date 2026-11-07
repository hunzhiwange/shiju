use super::*;
use axum::{
    http::HeaderMap,
    routing::{post, put},
};
use std::path::PathBuf;
use vw_api_types::app_providers::request_options::*;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/{app}/{id}/request-options/read", post(read))
        .route("/app-providers/{app}/{id}/request-options", put(write))
        .layer(DefaultBodyLimit::max(144 * 1024))
        .layer(Extension(Policy(path)))
}
async fn read(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<RequestOptionsTarget>, JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else { return error("invalid input") };
    if let Err(status) = super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        &headers,
        &tenant,
        &body.target_host_id,
        false,
    ) {
        return status.into_response();
    }
    match tokio::task::spawn_blocking(move || store(db)?.request_options_view(&tenant, app, &id))
        .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(e)) => error(&e),
        Err(_) => error("application provider storage unavailable"),
    }
}
async fn write(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<RequestOptionsWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else { return error("invalid input") };
    if let Err(status) = super::super::agent_apps::provider_editor_context(
        policy.0.as_deref(),
        &headers,
        &tenant,
        &body.target_host_id,
        true,
    ) {
        return status.into_response();
    }
    match tokio::task::spawn_blocking(move || {
        store(db)?.request_options_save(&tenant, app, &id, &body.options)
    })
    .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(e)) => error(&e),
        Err(_) => error("application provider storage unavailable"),
    }
}
