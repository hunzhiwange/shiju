//! 任务池 worktree 池路由模块。

use axum::Json;
use axum::Router;
use axum::routing::post;
use std::sync::LazyLock;
use vw_api_types::id::TaskId;
use vw_api_types::task::{
    TaskPoolWorktreeAcquireRequest, TaskPoolWorktreeAcquireResponse,
    TaskPoolWorktreeMaintenanceRequest, TaskPoolWorktreeMaintenanceResponse,
    TaskPoolWorktreeMergeLockRequest, TaskPoolWorktreeMergeLockResponse,
    TaskPoolWorktreeMergeUnlockRequest, TaskPoolWorktreeMergeUnlockResponse,
    TaskPoolWorktreeRecycleRequest, TaskPoolWorktreeRecycleResponse,
    TaskPoolWorktreeReleaseRequest, TaskPoolWorktreeReleaseResponse,
    TaskPoolWorktreeSlotRecycleRequest, TaskPoolWorktreeSlotRecycleResponse,
    TaskPoolWorktreeSnapshotRequest, TaskPoolWorktreeSnapshotResponse,
};

use crate::app::agent::gateway::ApiError;
use crate::app::agent::project;
use crate::task_worktree_pool::TaskWorktreePoolService;

static TASK_WORKTREE_POOL_SERVICE: LazyLock<TaskWorktreePoolService> =
    LazyLock::new(TaskWorktreePoolService::new);

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/task-pool/worktrees/snapshot", post(handle_api_task_pool_worktree_snapshot))
        .route("/task-pool/worktrees/maintain", post(handle_api_task_pool_worktree_maintain))
        .route("/task-pool/worktrees/acquire", post(handle_api_task_pool_worktree_acquire))
        .route("/task-pool/worktrees/release", post(handle_api_task_pool_worktree_release))
        .route("/task-pool/worktrees/recycle", post(handle_api_task_pool_worktree_recycle))
        .route(
            "/task-pool/worktrees/slots/recycle",
            post(handle_api_task_pool_worktree_slot_recycle),
        )
        .route("/task-pool/worktrees/merge-lock", post(handle_api_task_pool_worktree_merge_lock))
        .route(
            "/task-pool/worktrees/merge-unlock",
            post(handle_api_task_pool_worktree_merge_unlock),
        )
}

pub async fn handle_api_task_pool_worktree_snapshot(
    Json(body): Json<TaskPoolWorktreeSnapshotRequest>,
) -> Result<Json<TaskPoolWorktreeSnapshotResponse>, ApiError> {
    let project = load_project(body.project_id.as_ref()).await?;
    let response = task_worktree_pool_service()
        .snapshot(&project)
        .await
        .map_err(map_task_worktree_pool_error)?;
    Ok(Json(response))
}

pub async fn handle_api_task_pool_worktree_maintain(
    Json(body): Json<TaskPoolWorktreeMaintenanceRequest>,
) -> Result<Json<TaskPoolWorktreeMaintenanceResponse>, ApiError> {
    let project = load_project(body.project_id.as_ref()).await?;
    let response = task_worktree_pool_service()
        .maintain(&project, body.desired_capacity)
        .await
        .map_err(map_task_worktree_pool_error)?;
    Ok(Json(response))
}

pub async fn handle_api_task_pool_worktree_acquire(
    Json(body): Json<TaskPoolWorktreeAcquireRequest>,
) -> Result<Json<TaskPoolWorktreeAcquireResponse>, ApiError> {
    let project = load_project(body.project_id.as_ref()).await?;
    let response = task_worktree_pool_service()
        .acquire_and_prepare(&project, &body.task_id)
        .await
        .map_err(map_task_worktree_pool_error)?;
    Ok(Json(response))
}

pub async fn handle_api_task_pool_worktree_release(
    Json(body): Json<TaskPoolWorktreeReleaseRequest>,
) -> Result<Json<TaskPoolWorktreeReleaseResponse>, ApiError> {
    let project = load_project(body.project_id.as_ref()).await?;
    let response = task_worktree_pool_service()
        .release_task_slot(&project, &body.task_id)
        .await
        .map_err(map_task_worktree_pool_error)?;
    Ok(Json(response))
}

pub async fn handle_api_task_pool_worktree_recycle(
    Json(body): Json<TaskPoolWorktreeRecycleRequest>,
) -> Result<Json<TaskPoolWorktreeRecycleResponse>, ApiError> {
    let project = load_project(body.project_id.as_ref()).await?;
    let response = task_worktree_pool_service()
        .recycle_task_slot(&project, &body.task_id, body.taint_reason)
        .await
        .map_err(map_task_worktree_pool_error)?;
    Ok(Json(response))
}

pub async fn handle_api_task_pool_worktree_slot_recycle(
    Json(body): Json<TaskPoolWorktreeSlotRecycleRequest>,
) -> Result<Json<TaskPoolWorktreeSlotRecycleResponse>, ApiError> {
    let project = load_project(body.project_id.as_ref()).await?;
    let response = task_worktree_pool_service()
        .recycle_slot(&project, &body.slot_id)
        .await
        .map_err(map_task_worktree_pool_error)?;
    Ok(Json(response))
}

pub async fn handle_api_task_pool_worktree_merge_lock(
    Json(body): Json<TaskPoolWorktreeMergeLockRequest>,
) -> Result<Json<TaskPoolWorktreeMergeLockResponse>, ApiError> {
    let project = load_project(body.project_id.as_ref()).await?;
    let response = task_worktree_pool_service()
        .try_lock_merge_target(&project, &body.task_id, &body.target_branch)
        .await
        .map_err(map_task_worktree_pool_error)?;
    Ok(Json(response))
}

pub async fn handle_api_task_pool_worktree_merge_unlock(
    Json(body): Json<TaskPoolWorktreeMergeUnlockRequest>,
) -> Result<Json<TaskPoolWorktreeMergeUnlockResponse>, ApiError> {
    let project = load_project(body.project_id.as_ref()).await?;
    let holder_before = merge_lock_holder(&project, &body.target_branch).await?;
    let response = task_worktree_pool_service()
        .unlock_merge_target(&project, &body.task_id, &body.target_branch)
        .await
        .map_err(map_task_worktree_pool_error)?;
    Ok(Json(TaskPoolWorktreeMergeUnlockResponse {
        released: holder_before.as_ref() == Some(&body.task_id)
            && response.holder_task_id.as_ref() != Some(&body.task_id),
    }))
}

pub(crate) fn task_worktree_pool_service() -> &'static TaskWorktreePoolService {
    &TASK_WORKTREE_POOL_SERVICE
}

async fn load_project(project_id: &str) -> Result<project::Info, ApiError> {
    crate::app::agent::gateway::instance::load_local_project(project_id).await
}

async fn merge_lock_holder(
    project: &project::Info,
    target_branch: &str,
) -> Result<Option<TaskId>, ApiError> {
    task_worktree_pool_service()
        .merge_lock_holder(project, target_branch)
        .await
        .map_err(map_task_worktree_pool_error)
}

fn map_task_worktree_pool_error(error: anyhow::Error) -> ApiError {
    let message = error.to_string();
    if message.contains("task worktree slot missing") {
        return ApiError::not_found(message);
    }
    ApiError::bad_request(message)
}

#[cfg(test)]
#[path = "task_pool_tests.rs"]
mod task_pool_tests;
