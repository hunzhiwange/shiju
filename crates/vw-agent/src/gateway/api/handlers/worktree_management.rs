//! Tenant/project-scoped facade over the existing worktree lifecycle services.
use super::project::{load_project, project_context_directory, project_scope};
use crate::{
    gateway::ApiError,
    worktree::{management, recovery},
};
use axum::{
    Json, Router,
    extract::{Extension, Path},
    routing::{get, post},
};
use std::path::PathBuf;
use vw_api_types::{
    common::OperationAck,
    id::{ProjectScope, TenantScope},
    worktree::*,
};
pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/projects/{project_id}/managed-worktrees", get(list))
        .route("/projects/{project_id}/managed-worktrees/{id}", axum::routing::delete(delete))
        .route("/projects/{project_id}/managed-worktrees/{id}/deletion-preflight", post(preflight))
        .route("/projects/{project_id}/managed-worktrees/{id}/retry-initialization", post(retry))
        .route("/projects/{project_id}/worktree-snapshots", get(snapshots))
        .route("/projects/{project_id}/worktree-snapshots/{id}/restore", post(restore))
        .route("/projects/{project_id}/worktree-cleanup-proposal", get(proposal))
}
fn error(error: impl std::fmt::Display) -> ApiError {
    ApiError::bad_request(error.to_string())
}
async fn context(
    tenant: TenantScope,
    project: String,
) -> Result<(ProjectScope, PathBuf), ApiError> {
    let scope = project_scope(tenant, project)?;
    let info = load_project(&scope).await?;
    Ok((scope, project_context_directory(&info)?.into()))
}
async fn settings(
    scope: &ProjectScope,
) -> Result<vw_config_types::worktree::WorktreeSettings, ApiError> {
    Ok(crate::config::get_global_for_tenant(scope.tenant_scope())
        .await
        .map_err(error)?
        .worktree_settings)
}
async fn list(
    Extension(tenant): Extension<TenantScope>,
    Path(project): Path<String>,
) -> Result<Json<ListManagedWorktreesResponse>, ApiError> {
    let (scope, primary) = context(tenant, project).await?;
    Ok(Json(management::list(&scope, &primary).await.map_err(error)?))
}
async fn preflight(
    Extension(tenant): Extension<TenantScope>,
    Path((project, id)): Path<(String, String)>,
) -> Result<Json<WorktreeDeletionPreflight>, ApiError> {
    let (scope, primary) = context(tenant, project).await?;
    Ok(Json(
        management::preflight(&scope, &primary, &id, &settings(&scope).await?)
            .await
            .map_err(error)?,
    ))
}
async fn delete(
    Extension(tenant): Extension<TenantScope>,
    Path((project, id)): Path<(String, String)>,
    Json(request): Json<DeleteManagedWorktreeRequest>,
) -> Result<Json<DeleteManagedWorktreeResponse>, ApiError> {
    let (scope, primary) = context(tenant, project).await?;
    Ok(Json(
        management::delete(&scope, &primary, &id, &settings(&scope).await?, &request)
            .await
            .map_err(error)?,
    ))
}
async fn snapshots(
    Extension(tenant): Extension<TenantScope>,
    Path(project): Path<String>,
) -> Result<Json<ListWorktreeSnapshotsResponse>, ApiError> {
    let (scope, primary) = context(tenant, project).await?;
    Ok(Json(ListWorktreeSnapshotsResponse {
        items: recovery::list(&scope, &primary).await.map_err(error)?,
    }))
}
async fn restore(
    Extension(tenant): Extension<TenantScope>,
    Path((project, id)): Path<(String, String)>,
) -> Result<Json<RestoreWorktreeResponse>, ApiError> {
    let (scope, primary) = context(tenant, project).await?;
    let restored = recovery::restore(&scope, &primary, &id).await.map_err(error)?;
    let worktree = management::list(&scope, &primary)
        .await
        .map_err(error)?
        .items
        .into_iter()
        .find(|w| w.worktree.id.as_ref() == restored.record.id)
        .ok_or_else(|| ApiError::internal("Restored worktree missing from registry"))?;
    Ok(Json(RestoreWorktreeResponse {
        worktree,
        original_worktree_id: restored.original_id,
        branch_changed: restored.branch_changed,
    }))
}
async fn retry(
    Extension(tenant): Extension<TenantScope>,
    Path((project, id)): Path<(String, String)>,
) -> Result<Json<OperationAck>, ApiError> {
    let (scope, primary) = context(tenant, project).await?;
    management::retry(&scope, &primary, &id).await.map_err(error)?;
    Ok(Json(OperationAck { ok: true, message: None }))
}
async fn proposal(
    Extension(tenant): Extension<TenantScope>,
    Path(project): Path<String>,
) -> Result<Json<WorktreeCleanupProposal>, ApiError> {
    let (scope, primary) = context(tenant, project).await?;
    Ok(Json(management::proposal(&scope, &primary, &settings(&scope).await?).await.map_err(error)?))
}
#[cfg(test)]
#[path = "worktree_management_tests.rs"]
mod tests;
