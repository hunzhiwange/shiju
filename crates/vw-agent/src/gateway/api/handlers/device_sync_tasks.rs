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
    device_sync_task::{SyncErrorCode, SyncTaskStatus, SyncTaskState, SyncTaskCancelResponse},
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
        .route("/agent-apps/device-sync/tasks/{id}", get(read).delete(cancel))
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
    let target = authorize(tenant, &headers, query)?;
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

fn authorize(tenant: TenantScope, headers: &HeaderMap, query: Context)
    -> Result<DataManagementTarget, StatusCode> {
    let scope = ProjectScope::new(tenant, query.project_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
    let principal_id = super::agent_apps::authorize_device_sync_config(
        policy.as_deref(), headers, &scope, &query.target_host_id)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(DataManagementTarget {
        scope, target_host_id: query.target_host_id, principal_id,
    })
}

async fn cancel(
    Extension(tenant): Extension<TenantScope>,
    injected: Option<Extension<GlobalDatabase>>,
    Path(id): Path<TaskId>,
    headers: HeaderMap,
    Query(query): Query<Context>,
) -> Result<(StatusCode, Json<SyncTaskCancelResponse>), StatusCode> {
    // Authorize before consulting either live tasks or the durable journal.
    let target = authorize(tenant, &headers, query)?;
    let database = match injected {
        Some(Extension(database)) => database,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    let current = status::task_status(database.clone(), &target, &id)
        .map_err(|error| match error {
            SyncErrorCode::Unauthorized => StatusCode::NOT_FOUND,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        })?;
    let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
    // Cancellation requires the existing execution grant for this direction.
    match current.direction {
        vw_api_types::device_sync_task::SyncDirection::Upload => {
            super::agent_apps::authorize_device_sync_upload(policy.as_deref(), &headers, &target)
                .map_err(|_| StatusCode::NOT_FOUND)?;
        }
        vw_api_types::device_sync_task::SyncDirection::Download => {
            super::agent_apps::authorize_device_sync_download(policy.as_deref(), &headers, &target)
                .map_err(|_| StatusCode::NOT_FOUND)?;
        }
    }
    let outcome = match crate::device_sync::cancel::cancel(&target, &id) {
        Ok(outcome) => outcome,
        Err(SyncErrorCode::Unauthorized) => {
            let status = status::task_status(database, &target, &id)
                .map_err(|error| match error {
                    SyncErrorCode::Unauthorized => StatusCode::NOT_FOUND,
                    _ => StatusCode::SERVICE_UNAVAILABLE,
                })?;
            // Never initialize recovery from an HTTP request or claim to cancel
            // a durable active record without its execution owner.
            if !status.state.is_terminal() { return Err(StatusCode::SERVICE_UNAVAILABLE); }
            if status.state == SyncTaskState::Cancelled {
                crate::device_sync::cancel::CancelOutcome::Accepted(status)
            } else {
                crate::device_sync::cancel::CancelOutcome::TooLate(status)
            }
        }
        Err(_) => return Err(StatusCode::SERVICE_UNAVAILABLE),
    };
    use crate::device_sync::cancel::CancelOutcome;
    Ok(match outcome {
        CancelOutcome::Accepted(status) =>
            (StatusCode::OK, Json(SyncTaskCancelResponse::Accepted { status })),
        CancelOutcome::TooLate(status) =>
            (StatusCode::CONFLICT, Json(SyncTaskCancelResponse::TooLate { status })),
    })
}
