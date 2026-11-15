//! Desktop configuration adapter for worktree operations.

use super::*;

pub fn assign_task_execution_worktree(
    project_path: &str,
    task: &Task,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<Option<String>, String> {
    desktop_task_executor::executor::worktree_admin::assign_task_execution_worktree(crate::app::config::gateway_client, project_path, task, sender)
}

pub fn reset_all_managed_worktrees(
    project_path: &str,
    clean_untracked: bool,
) -> Result<usize, String> {
    desktop_task_executor::executor::worktree_admin::reset_all_managed_worktrees(crate::app::config::gateway_client, project_path, clean_untracked)
}

pub fn reset_all_managed_worktrees_with_logs(
    project_path: &str,
    clean_untracked: bool,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<usize, String> {
    desktop_task_executor::executor::worktree_admin::reset_all_managed_worktrees_with_logs(crate::app::config::gateway_client, project_path, clean_untracked, sender)
}

pub fn reset_all_managed_worktrees_async(
    project_path: String,
    clean_untracked: bool,
) -> impl std::future::Future<Output = Result<usize, String>> {
    desktop_task_executor::executor::worktree_admin::reset_all_managed_worktrees_async(crate::app::config::gateway_client, project_path, clean_untracked)
}

pub async fn reset_all_managed_worktrees_async_with_logs(
    project_path: String,
    clean_untracked: bool,
    log_sender: Option<Sender<TaskLogStream>>,
) -> Result<usize, String> {
    desktop_task_executor::executor::worktree_admin::reset_all_managed_worktrees_async_with_logs(crate::app::config::gateway_client, project_path, clean_untracked, log_sender).await
}

pub fn delete_all_managed_worktrees(project_path: &str) -> Result<usize, String> {
    desktop_task_executor::executor::worktree_admin::delete_all_managed_worktrees(crate::app::config::gateway_client, project_path)
}

pub fn delete_all_managed_worktrees_with_logs(
    project_path: &str,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<usize, String> {
    desktop_task_executor::executor::worktree_admin::delete_all_managed_worktrees_with_logs(crate::app::config::gateway_client, project_path, sender)
}

pub fn delete_all_managed_worktrees_async(
    project_path: String,
) -> impl std::future::Future<Output = Result<usize, String>> {
    desktop_task_executor::executor::worktree_admin::delete_all_managed_worktrees_async(crate::app::config::gateway_client, project_path)
}

pub async fn delete_all_managed_worktrees_async_with_logs(
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> Result<usize, String> {
    desktop_task_executor::executor::worktree_admin::delete_all_managed_worktrees_async_with_logs(crate::app::config::gateway_client, project_path, log_sender).await
}

pub fn commit_merge_all_worktrees(project_path: &str, tasks: &[Task]) -> Result<usize, String> {
    desktop_task_executor::executor::worktree_admin::commit_merge_all_worktrees(crate::app::config::gateway_client, project_path, tasks)
}

pub fn commit_merge_all_worktrees_with_logs(
    project_path: &str,
    tasks: &[Task],
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<usize, String> {
    desktop_task_executor::executor::worktree_admin::commit_merge_all_worktrees_with_logs(crate::app::config::gateway_client, project_path, tasks, sender)
}

pub fn commit_merge_all_worktrees_async(
    project_path: String,
    tasks: Vec<Task>,
) -> impl std::future::Future<Output = Result<usize, String>> {
    desktop_task_executor::executor::worktree_admin::commit_merge_all_worktrees_async(crate::app::config::gateway_client, project_path, tasks)
}

pub async fn commit_merge_all_worktrees_async_with_logs(
    project_path: String,
    tasks: Vec<Task>,
    log_sender: Option<Sender<TaskLogStream>>,
) -> Result<usize, String> {
    desktop_task_executor::executor::worktree_admin::commit_merge_all_worktrees_async_with_logs(crate::app::config::gateway_client, project_path, tasks, log_sender).await
}

pub(super) fn resolve_task_execution_workspace(
    task: &Task,
    project_path: &str,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<(desktop_task_executor::executor::state::SelectedExecutionWorkspace, desktop_task_executor::executor::state::WorktreeClaimGuard), String> {
    desktop_task_executor::executor::worktree_admin::resolve_task_execution_workspace(crate::app::config::gateway_client, task, project_path, sender)
}
