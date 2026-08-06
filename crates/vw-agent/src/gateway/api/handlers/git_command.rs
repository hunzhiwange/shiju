//! `/v1/git/command` 通用 Git CLI 协议。
//!
//! 本端点仍通过系统 `git` 可执行文件执行任意参数，故意保留在 git2 迁移范围之外。
//! 核心本地路径（项目发现/元数据、任务池、网关 commit/merge）走 git2，不经过本模块。

use std::path::Path;

use axum::Json;
use axum::Router;
use axum::routing::post;
use vw_api_types::git::{GitCommandRequest, GitCommandResponse};

use crate::app::agent::gateway::ApiError;
use crate::app::agent::shell::git_std_command;

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/git/command", post(git_command_v1))
}

async fn git_command_v1(
    Json(body): Json<GitCommandRequest>,
) -> Result<Json<GitCommandResponse>, ApiError> {
    validate_git_command_request(&body)?;

    let response = tokio::task::spawn_blocking(move || run_git_command_blocking(&body))
        .await
        .map_err(|err| ApiError::internal(format!("git command task failed: {err}")))?
        .map_err(ApiError::bad_request)?;

    Ok(Json(response))
}

fn validate_git_command_request(request: &GitCommandRequest) -> Result<(), ApiError> {
    if request.directory.trim().is_empty() {
        return Err(ApiError::bad_request("git directory is required"));
    }
    if !Path::new(&request.directory).is_dir() {
        return Err(ApiError::bad_request("git directory does not exist"));
    }
    if request.args.is_empty() {
        return Err(ApiError::bad_request("git args are required"));
    }
    if request.args.iter().any(|arg| arg.contains('\0')) {
        return Err(ApiError::bad_request("git args contain invalid null byte"));
    }
    if request.directory.contains('\0') {
        return Err(ApiError::bad_request("git directory contains invalid null byte"));
    }
    if matches!(request.timeout_secs, Some(0)) {
        return Err(ApiError::bad_request("git timeout must be positive"));
    }
    Ok(())
}

fn run_git_command_blocking(request: &GitCommandRequest) -> Result<GitCommandResponse, String> {
    let output = git_std_command()
        .current_dir(&request.directory)
        .args(&request.args)
        .output()
        .map_err(|err| err.to_string())?;

    Ok(GitCommandResponse {
        success: output.status.success(),
        code: output.status.code(),
        stdout: normalize_git_output_bytes(&output.stdout),
        stderr: normalize_git_output_bytes(&output.stderr),
    })
}

fn normalize_git_output_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
#[path = "git_command_tests.rs"]
mod git_command_tests;
