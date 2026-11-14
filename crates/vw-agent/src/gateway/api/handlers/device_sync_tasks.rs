//! Authorized polling of host-local task records; no SSE replay guarantee.
use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use vw_api_types::{
    ProjectId, ProjectScope, TenantScope,
    agent_apps::DataManagementTarget,
    device_sync_task::{SyncErrorCode, SyncTaskStatus},
    id::TaskId,
};
use crate::{device_sync::status, global_database::GlobalDatabase};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Context {
    project_id: ProjectId,
    target_host_id: String,
}

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/device-sync/tasks/{id}", get(read))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

async fn read(
    Extension(tenant): Extension<TenantScope>,
    injected: Option<Extension<GlobalDatabase>>,
    Path(id): Path<TaskId>,
    headers: HeaderMap,
    Query(query): Query<Context>,
) -> Result<Json<SyncTaskStatus>, StatusCode> {
    let scope = ProjectScope::new(tenant, query.project_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
    // The existing settings grant already permits reading task summaries.
    // Derive the principal from policy, never from caller-supplied query data.
    let principal_id = super::agent_apps::authorize_device_sync_config(
        policy.as_deref(), &headers, &scope, &query.target_host_id)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let target = DataManagementTarget {
        scope, target_host_id: query.target_host_id, principal_id,
    };
    let database = match injected {
        Some(Extension(database)) => database,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    status::task_status(database, &target, &id).map(Json).map_err(|error| match error {
        SyncErrorCode::Unauthorized => StatusCode::NOT_FOUND,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })
}
