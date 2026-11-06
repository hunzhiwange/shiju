//! Tenant catalog and independently authorized host configuration metadata.
use super::{error, store};
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use std::path::PathBuf;
use vw_api_types::{agent_apps::AppId, id::TenantScope};
#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    target_host_id: String,
}
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/{app}", get(list))
        .route("/app-providers/{app}/catalog/state", get(read))
        .layer(Extension(PolicyFile(path)))
}
async fn list(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path(app): Path<AppId>,
) -> Response {
    match tokio::task::spawn_blocking(move || store(database)?.list(&tenant, app)).await {
        Ok(Ok(rows)) => Json(rows).into_response(),
        Ok(Err(message)) => error(&message),
        Err(_) => error("application provider storage unavailable"),
    }
}
async fn read(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    Query(target): Query<Target>,
) -> Result<Response, StatusCode> {
    tokio::task::spawn_blocking(move || {
        // Authorize before opening storage or configuration. No fallback to the client home.
        let home = super::super::agent_apps::provider_config_home(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &target.target_host_id,
        )?;
        match store(database).and_then(|store| store.list(&tenant, app)) {
            Ok(rows) => {
                Ok(Json(vw_agent_apps::providers::catalog::read(&home, app, rows)).into_response())
            }
            Err(message) => Ok(error(&message)),
        }
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
