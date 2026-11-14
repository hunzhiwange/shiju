//! Read-only inventory backed by the authorized host-local backup owner.
use axum::{Extension, Json, Router, extract::Query, http::{HeaderMap, StatusCode}, routing::get};
use serde::Deserialize;
use vw_api_types::{ProjectId, ProjectScope, TenantScope, agent_apps::data_management::DataBackupSummary};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Context { project_id: ProjectId, target_host_id: String }

pub(super) fn router() -> Router {
    Router::new().route("/agent-apps/data-backup-inventory", get(list))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}
async fn list(Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(query): Query<Context>)
    -> Result<Json<Vec<DataBackupSummary>>, StatusCode> {
    let project = ProjectScope::new(tenant, query.project_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
    let (root, principal) = super::agent_apps::authorize_backup_inventory(
        policy.as_deref(), &headers, &project, &query.target_host_id)?;
    #[cfg(target_os = "macos")]
    {
        tokio::task::spawn_blocking(move || {
            use vw_agent_apps::data_management::{backup_api, backup_list, safety_snapshot::SafetySnapshotScope};
            let scope = SafetySnapshotScope { project, principal_id: principal, host_id: query.target_host_id };
            let _guard = backup_api::lock_directory(&root).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let entries = backup_list::list_local_backups(&root, &scope).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let summaries = entries.into_iter().map(|entry| backup_api::summary(&root, &scope, &entry.id)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)).collect::<Result<Vec<_>, _>>()?;
            Ok(Json(summaries))
        }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = (root, principal, project); Err(StatusCode::NOT_IMPLEMENTED) }
}
