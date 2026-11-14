//! Fixed actions run on the authorized gateway executor, never the desktop client.
use axum::{extract::Query, http::{HeaderMap, StatusCode}, routing::post, Extension, Json, Router};
use serde::Deserialize;
use std::path::PathBuf;
use vw_agent_apps::workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory as Directory};
use vw_api_types::{id::{ProjectId, ProjectScope, TenantScope}, workspace_memory::{WorkspaceDirectory, WorkspaceOpenDirectoryRequest}};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Scope { project_id: ProjectId, target_host_id: String }
#[derive(Clone)]
struct Policy(Option<PathBuf>);

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/openclaw/workspace/open-directory", post(open))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}

async fn open(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap, Query(scope): Query<Scope>,
    Json(body): Json<WorkspaceOpenDirectoryRequest>,
) -> Result<StatusCode, StatusCode> {
    tokio::task::spawn_blocking(move || {
        // Creation requires workspace write permission, including project scope.
        // authorize_workspace rejects offline and remote bindings before any IO.
        let (host, _) = super::access::authorize_workspace(policy.0.as_deref(), &headers,
            &ProjectScope::new(tenant, scope.project_id), &scope.target_host_id, true)?;
        let area = match body.subdir {
            WorkspaceDirectory::Workspace => WorkspaceArea::OpenClawWorkspace,
            WorkspaceDirectory::Memory => WorkspaceArea::OpenClawMemory,
        };
        let directory = Directory::open(&scope.target_host_id, &host.id, &host.home,
            &host.home.join(".openclaw/workspace"), area, true)
            .map_err(path_error)?.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        directory.open_in_finder().map_err(path_error)?;
        Ok(StatusCode::NO_CONTENT)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
fn path_error(error: PathError) -> StatusCode {
    match error {
        PathError::UnauthorizedHost => StatusCode::FORBIDDEN,
        PathError::InvalidPath | PathError::UnsafeEntry => StatusCode::UNPROCESSABLE_ENTITY,
        PathError::Io => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
