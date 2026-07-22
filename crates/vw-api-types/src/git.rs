//! Git 状态、分支、diff 与提交相关类型。
//!
//! 本模块封装项目仓库相关的只读与写入协议，覆盖：
//! - 工作区状态与文件变更列表
//! - 不同范围的 diff 查询
//! - 分支枚举与检出
//! - 基于文件、hunk、行级粒度的提交选择
//!
//! 这些 DTO 主要服务于桌面端 Git 面板、变更预览和提交流程。

use crate::id::{ProjectId, WorktreeId};
use serde::{Deserialize, Serialize};

/// Git 中文件变更状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitFileStatus {
    /// 新增文件。
    Added,
    /// 已修改文件。
    Modified,
    /// 已删除文件。
    Deleted,
    /// 已重命名文件。
    Renamed,
    /// 已复制文件。
    Copied,
    /// 未跟踪文件。
    Untracked,
    /// 文件类型发生变化。
    TypeChanged,
}

/// 单个变更文件摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitChangedFileDto {
    /// 文件路径。
    pub path: String,
    /// 变更状态。
    pub status: GitFileStatus,
}

/// 获取 Git 状态请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitStatusRequest {
    pub project_id: ProjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
}

/// 仓库 Git 状态摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitStatusDto {
    pub branch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ahead: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behind: Option<u32>,
    #[serde(default)]
    pub staged: Vec<GitChangedFileDto>,
    #[serde(default)]
    pub unstaged: Vec<GitChangedFileDto>,
    #[serde(default)]
    pub untracked: Vec<GitChangedFileDto>,
}

/// Git diff 的取值范围。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitDiffScope {
    /// 工作区相对暂存区或 HEAD 的差异。
    WorkingTree,
    /// 暂存区相对 HEAD 的差异。
    Staged,
    /// 基线到当前 HEAD 的整体差异。
    BaseToHead,
}

/// 获取 Git diff 请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitDiffRequest {
    pub project_id: ProjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    pub scope: GitDiffScope,
}

/// Git diff 响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitDiffDto {
    pub patch: String,
}

/// 列出分支请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListGitBranchesRequest {
    pub project_id: ProjectId,
}

/// Git 分支摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitBranchDto {
    pub name: String,
    pub current: bool,
}

/// 列出分支响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListGitBranchesResponse {
    pub items: Vec<GitBranchDto>,
}

/// 检出分支请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitCheckoutRequest {
    pub project_id: ProjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    pub branch: String,
    #[serde(default)]
    pub create_if_missing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_ref: Option<String>,
}

/// 检出分支响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitCheckoutResponse {
    pub ok: bool,
    pub branch: GitBranchDto,
}

/// 创建提交请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitCommitRequest {
    pub project_id: ProjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    pub message: String,
    #[serde(default)]
    pub stage_all: bool,
    #[serde(default)]
    pub selected_files: Vec<String>,
    #[serde(default)]
    pub selected_hunks: Vec<GitHunkSelectionDto>,
    #[serde(default)]
    pub selected_lines: Vec<GitLineSelectionDto>,
    #[serde(default)]
    pub selected_old_lines: Vec<GitLineSelectionDto>,
}

/// 单个 hunk 选择项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHunkSelectionDto {
    pub path: String,
    pub index: usize,
}

/// 单行选择项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitLineSelectionDto {
    pub path: String,
    pub line: usize,
}

/// 提交结果摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitCommitDto {
    pub sha: String,
    pub message: String,
}

/// 创建提交响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitCommitResponse {
    pub ok: bool,
    #[serde(default = "default_commit_created")]
    pub created: bool,
    pub commit: GitCommitDto,
}

fn default_commit_created() -> bool {
    true
}

/// 受限 Git 命令请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitCommandRequest {
    pub directory: String,
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

/// 受限 Git 命令响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitCommandResponse {
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// 合并分支请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitMergeRequest {
    pub project_id: ProjectId,
    pub source_branch: String,
    pub target_branch: String,
}

/// 合并分支响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitMergeResponse {
    pub ok: bool,
    pub source_branch: String,
    pub target_branch: String,
    pub workspace: String,
    #[serde(default)]
    pub already_merged: bool,
    pub message: String,
}
