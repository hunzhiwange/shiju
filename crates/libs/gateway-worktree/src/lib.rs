//! Tenant/project-scoped facade over the existing worktree lifecycle services.
#![cfg(not(target_arch = "wasm32"))]
use gateway_types::ApiError;
use worktree_core::{management, recovery};
use axum::{
    Json, Router,
    extract::{Extension, Path},
    routing::{get, post},
};
use std::{future::Future, path::PathBuf, pin::Pin};

/// Host project authorization and tenant configuration, using the existing errors.
pub type HostFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;
#[derive(Clone, Copy)]
pub struct WorktreeHost {
    pub context: fn(TenantScope, String) -> HostFuture<'static, (ProjectScope, PathBuf)>,
    pub settings: for<'a> fn(&'a ProjectScope) -> HostFuture<'a, config_worktree::WorktreeSettings>,
}

use {
    vw_api_transport::common::OperationAck,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
    vw_api_worktree::*,
};
pub fn router<S: Clone + Send + Sync + 'static>(host: WorktreeHost) -> Router<S> {
    Router::new()
        .route("/projects/{project_id}/managed-worktrees", get(list))
        .route("/projects/{project_id}/managed-worktrees/{id}", axum::routing::delete(delete))
        .route("/projects/{project_id}/managed-worktrees/{id}/deletion-preflight", post(preflight))
        .route("/projects/{project_id}/managed-worktrees/{id}/retry-initialization", post(retry))
        .route("/projects/{project_id}/managed-worktrees/{id}/record-use", post(record_use))
        .route("/projects/{project_id}/worktree-snapshots", get(snapshots))
        .route("/projects/{project_id}/worktree-snapshots/{id}/restore", post(restore))
        .route("/projects/{project_id}/worktree-cleanup-proposal", get(proposal))
        .route("/projects/{project_id}/worktree-cleanup-status", get(cleanup_status))
        .layer(Extension(host))
}
fn error(error: impl std::fmt::Display) -> ApiError {
    ApiError::bad_request(error.to_string())
}
async fn list(
    Extension(host): Extension<WorktreeHost>,
    Extension(tenant): Extension<TenantScope>,
    Path(project): Path<String>,
) -> Result<Json<ListManagedWorktreesResponse>, ApiError> {
    let (scope, primary) = (host.context)(tenant, project).await?;
    Ok(Json(management::list(&scope, &primary).await.map_err(error)?))
}
async fn preflight(
    Extension(host): Extension<WorktreeHost>,
    Extension(tenant): Extension<TenantScope>,
    Path((project, id)): Path<(String, String)>,
) -> Result<Json<WorktreeDeletionPreflight>, ApiError> {
    let (scope, primary) = (host.context)(tenant, project).await?;
    Ok(Json(
        management::preflight(&scope, &primary, &id, &(host.settings)(&scope).await?)
            .await
            .map_err(error)?,
    ))
}
async fn delete(
    Extension(host): Extension<WorktreeHost>,
    Extension(tenant): Extension<TenantScope>,
    Path((project, id)): Path<(String, String)>,
    Json(request): Json<DeleteManagedWorktreeRequest>,
) -> Result<Json<DeleteManagedWorktreeResponse>, ApiError> {
    let (scope, primary) = (host.context)(tenant, project).await?;
    Ok(Json(
        management::delete(&scope, &primary, &id, &(host.settings)(&scope).await?, &request)
            .await
            .map_err(error)?,
    ))
}
async fn snapshots(
    Extension(host): Extension<WorktreeHost>,
    Extension(tenant): Extension<TenantScope>,
    Path(project): Path<String>,
) -> Result<Json<ListWorktreeSnapshotsResponse>, ApiError> {
    let (scope, primary) = (host.context)(tenant, project).await?;
    Ok(Json(ListWorktreeSnapshotsResponse {
        items: recovery::list(&scope, &primary).await.map_err(error)?,
    }))
}
async fn restore(
    Extension(host): Extension<WorktreeHost>,
    Extension(tenant): Extension<TenantScope>,
    Path((project, id)): Path<(String, String)>,
) -> Result<Json<RestoreWorktreeResponse>, ApiError> {
    let (scope, primary) = (host.context)(tenant, project).await?;
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
    Extension(host): Extension<WorktreeHost>,
    Extension(tenant): Extension<TenantScope>,
    Path((project, id)): Path<(String, String)>,
) -> Result<Json<OperationAck>, ApiError> {
    let (scope, primary) = (host.context)(tenant, project).await?;
    management::retry(&scope, &primary, &id).await.map_err(error)?;
    Ok(Json(OperationAck { ok: true, message: None }))
}
async fn proposal(
    Extension(host): Extension<WorktreeHost>,
    Extension(tenant): Extension<TenantScope>,
    Path(project): Path<String>,
) -> Result<Json<WorktreeCleanupProposal>, ApiError> {
    let (scope, primary) = (host.context)(tenant, project).await?;
    Ok(Json(management::proposal(&scope, &primary, &(host.settings)(&scope).await?).await.map_err(error)?))
}

async fn record_use(
    Extension(host): Extension<WorktreeHost>,
    Extension(tenant): Extension<TenantScope>,
    Path((project, id)): Path<(String, String)>,
) -> Result<Json<OperationAck>, ApiError> {
    let (scope, primary) = (host.context)(tenant, project).await?;
    let _lock = worktree_core::registry::lifecycle_lock(&primary).map_err(error)?;
    let record =
        worktree_core::registry::authorize(&scope, &primary, &id).await.map_err(error)?;
    if record.state != worktree_core::registry::Lifecycle::Ready {
        return Err(ApiError::bad_request("Managed checkout is not ready"));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(error)?
        .as_millis()
        .min(u64::MAX as u128) as u64;
    worktree_core::registry::record_use(&scope, &record.directory, now).map_err(error)?;
    Ok(Json(OperationAck { ok: true, message: None }))
}

async fn cleanup_status(
    Extension(host): Extension<WorktreeHost>,
    Extension(tenant): Extension<TenantScope>,
    Path(project): Path<String>,
) -> Result<Json<worktree_core::cleanup::FlowStatus>, ApiError> {
    let (scope, _primary) = (host.context)(tenant, project).await?;
    let mut report = worktree_core::cleanup::flow_status(&scope).await.map_err(error)?;
    // Read the host's saved plan without competing with an in-flight removal.
    if !(host.settings)(&scope).await?.auto_cleanup_enabled {
        report.proposal = Default::default();
    }
    Ok(Json(report))
}
