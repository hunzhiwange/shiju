use super::{error, store};
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::delete,
};
use std::path::PathBuf;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::delete::{AppProviderDelete, AppProviderDeleted},
    id::TenantScope,
};
#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route(
            "/app-providers/{app}/{id}",
            delete(remove).layer(axum::extract::DefaultBodyLimit::max(16 * 1024)),
        )
        .layer(Extension(PolicyFile(path)))
}
async fn remove(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path((app, id)): Path<(AppId, String)>,
    body: Result<Json<AppProviderDelete>, JsonRejection>,
) -> Result<Response, StatusCode> {
    let Ok(Json(request)) = body else {
        return Ok(error("invalid input"));
    };
    tokio::task::spawn_blocking(move || {
        let home = super::super::agent_apps::provider_config_home(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
        )?;
        match store(database).and_then(|s| s.delete(&tenant, app, &id, &home)) {
            Ok(()) => Ok(Json(AppProviderDeleted { deleted: true }).into_response()),
            Err(message) => Ok(error(&message)),
        }
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
