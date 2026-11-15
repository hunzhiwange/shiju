//! 桌面配置到任务 Git 执行器的窄适配。

use super::*;

pub(super) use desktop_task_executor::executor::git::{GitCommandOutput, git_output_failure_detail};

pub(super) fn run_git(cwd: &str, args: &[&str]) -> Result<GitCommandOutput, String> {
    desktop_task_executor::executor::git::run_git(crate::app::config::gateway_client, cwd, args)
}

pub(super) fn run_git_with_timeout(
    cwd: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<(GitCommandOutput, Duration), String> {
    desktop_task_executor::executor::git::run_git_with_timeout(crate::app::config::gateway_client, cwd, args, timeout)
}

pub(super) fn run_git_logged(
    sender: Option<&Sender<TaskLogStream>>,
    cwd: &str,
    args: &[&str],
) -> Result<GitCommandOutput, String> {
    desktop_task_executor::executor::git::run_git_logged(crate::app::config::gateway_client, sender, cwd, args)
}

pub(super) fn run_git_logged_with_timeout(
    sender: Option<&Sender<TaskLogStream>>,
    cwd: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<(GitCommandOutput, Duration), String> {
    desktop_task_executor::executor::git::run_git_logged_with_timeout(crate::app::config::gateway_client, sender, cwd, args, timeout)
}

pub(super) fn run_git_maintenance_logged(
    sender: Option<&Sender<TaskLogStream>>,
    cwd: &str,
    args: &[&str],
) -> Result<GitCommandOutput, String> {
    desktop_task_executor::executor::git::run_git_maintenance_logged(crate::app::config::gateway_client, sender, cwd, args)
}

pub(super) fn abort_git_in_progress_states(cwd: &str, sender: Option<&Sender<TaskLogStream>>) {
    desktop_task_executor::executor::git::abort_git_in_progress_states(crate::app::config::gateway_client, cwd, sender)
}

pub(super) fn git_branch_name(cwd: &str) -> Option<String> {
    desktop_task_executor::executor::git::git_branch_name(crate::app::config::gateway_client, cwd)
}

pub(super) fn git_repo_root(cwd: &str) -> Option<String> {
    desktop_task_executor::executor::git::git_repo_root(crate::app::config::gateway_client, cwd)
}

pub(super) fn git_has_staged_changes(cwd: &str) -> Result<bool, String> {
    desktop_task_executor::executor::git::git_has_staged_changes(crate::app::config::gateway_client, cwd)
}

pub fn build_review_diff_context(
    project_path: &str,
    source_branch: Option<&str>,
    target_branch: Option<&str>,
) -> Result<(String, String, String, String, String), String> {
    desktop_task_executor::executor::git::build_review_diff_context(crate::app::config::gateway_client, project_path, source_branch, target_branch)
}
