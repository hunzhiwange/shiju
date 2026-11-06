//! 项目和 worktree 范围内的 Git 历史及远端 API。
use crate::app::agent::gateway::ApiError;
use axum::{Json, Router, routing::post};
use vw_api_types::git::{
    GitHistoryDetailDto, GitHistoryDetailRequest, GitRemoteRequest, GitRemoteResponse,
    GitRepositoryDto, GitRepositoryRequest,
};

#[path = "history.rs"]
mod history;
#[path = "remote.rs"]
mod remote;

pub(super) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/git/repository", post(repository))
        .route("/git/history/detail", post(detail))
        .route("/git/remote", post(sync_remote))
}

async fn repository(
    Json(body): Json<GitRepositoryRequest>,
) -> Result<Json<GitRepositoryDto>, ApiError> {
    if body.limit == 0 || body.limit > 10_000 {
        return Err(ApiError::bad_request("history limit must be between 1 and 10000"));
    }
    let directory =
        super::resolve_git_directory(&body.project_id.0, body.worktree_id.as_ref()).await?;
    let result = tokio::task::spawn_blocking(move || history::snapshot(&directory, body.limit))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .map_err(ApiError::bad_request)?;
    Ok(Json(result))
}

async fn detail(
    Json(body): Json<GitHistoryDetailRequest>,
) -> Result<Json<GitHistoryDetailDto>, ApiError> {
    git2::Oid::from_str(&body.sha).map_err(|_| ApiError::bad_request("invalid commit SHA"))?;
    let directory =
        super::resolve_git_directory(&body.project_id.0, body.worktree_id.as_ref()).await?;
    let result = tokio::task::spawn_blocking(move || history::detail(&directory, &body.sha))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .map_err(ApiError::bad_request)?;
    Ok(Json(result))
}

async fn sync_remote(
    Json(body): Json<GitRemoteRequest>,
) -> Result<Json<GitRemoteResponse>, ApiError> {
    let directory =
        super::resolve_git_directory(&body.project_id.0, body.worktree_id.as_ref()).await?;
    // 等待真实操作结束；不能在 spawn_blocking 外超时后继续修改仓库。
    tokio::task::spawn_blocking(move || remote::sync(&directory, body.action))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .map_err(ApiError::bad_request)?;
    Ok(Json(GitRemoteResponse { ok: true }))
}

#[cfg(test)]
#[path = "repository_tests.rs"]
mod tests;
