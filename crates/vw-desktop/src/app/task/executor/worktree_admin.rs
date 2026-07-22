#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

//! 任务执行器的 worktree_admin.rs 子模块。
//!
//! 任务池 slot 的租约、回收和合并锁已迁移到 gateway/agent。桌面端这里仅保留
//! 选定工作区解析、进程内路径占用保护，以及普通 git worktree 的批量管理入口。

use super::git::{
    abort_git_in_progress_states, git_branch_name, git_has_staged_changes,
    git_output_failure_detail, git_repo_root, run_git_logged, run_git_maintenance_logged,
};
use super::process_utils::{emit_stderr_log, emit_stdout_log, normalize_path};
use super::state::{TaskLogStream, WorktreeClaimGuard, WorktreeEntry, claimed_worktrees};
use super::worktree_pool::{is_managed_worktree_path, parse_worktree_list};
use super::*;

fn valid_branch_name(branch: &str) -> bool {
    let branch = branch.trim();
    !branch.is_empty() && branch != "HEAD"
}

fn resolve_merge_target_branch(task: &Task, project_path: &str) -> Option<String> {
    task.merge_target_branch
        .as_deref()
        .map(str::trim)
        .filter(|branch| valid_branch_name(branch))
        .map(str::to_string)
        .or_else(|| git_branch_name(project_path).filter(|branch| valid_branch_name(branch)))
        .or_else(|| {
            git_repo_root(project_path)
                .and_then(|repo_root| git_branch_name(&repo_root))
                .filter(|branch| valid_branch_name(branch))
        })
}

pub fn assign_task_execution_worktree(
    project_path: &str,
    task: &Task,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<Option<String>, String> {
    let selected_path = task
        .selected_worktree_path
        .clone()
        .map(|path| normalize_path(&path))
        .filter(|path| Path::new(path).exists());
    if let Some(path) = selected_path {
        emit_stdout_log(
            sender,
            format!("[WORKTREE] 使用已选择工作区 task={} path={}", task.id, path),
        );
        return Ok(Some(path));
    }

    if resolve_merge_target_branch(task, project_path).is_some() {
        emit_stdout_log(
            sender,
            format!("[WORKTREE] 任务池 worktree 由 gateway 预分配 task={}", task.id),
        );
    }
    Ok(None)
}

pub(super) fn claim_worktree_path(path: &str) -> bool {
    let normalized = normalize_path(path);
    let Ok(mut claimed) = claimed_worktrees().lock() else {
        return false;
    };
    if claimed.contains(&normalized) {
        return false;
    }
    claimed.insert(normalized);
    true
}

pub(super) fn release_claimed_worktree(path: &str) {
    let normalized = normalize_path(path);
    if let Ok(mut claimed) = claimed_worktrees().lock() {
        claimed.remove(&normalized);
    }
}

pub(super) fn resolve_task_execution_workspace(
    task: &Task,
    project_path: &str,
    _sender: Option<&Sender<TaskLogStream>>,
) -> Result<(super::state::SelectedExecutionWorkspace, WorktreeClaimGuard), String> {
    let selected_path = task
        .selected_worktree_path
        .clone()
        .map(|path| normalize_path(&path))
        .filter(|path| Path::new(path).exists());
    let normalized_project_path = normalize_path(project_path);
    let execution_path = selected_path.clone().unwrap_or_else(|| normalized_project_path.clone());
    let selected_worktree_branch = selected_path
        .as_deref()
        .and_then(git_branch_name)
        .filter(|branch| valid_branch_name(branch))
        .or_else(|| task.merge_source_branch.clone().filter(|branch| valid_branch_name(branch)));
    let claimed = selected_path.as_deref().map(claim_worktree_path).unwrap_or(false);
    let guard = WorktreeClaimGuard::new(if claimed { selected_path.clone() } else { None });

    Ok((
        super::state::SelectedExecutionWorkspace {
            slot_id: task.selected_worktree_slot_id.clone(),
            execution_path,
            selected_worktree_path: selected_path,
            selected_worktree_branch,
            merge_target_branch: resolve_merge_target_branch(task, project_path),
            project_path: normalized_project_path,
        },
        guard,
    ))
}

fn repo_root_for_worktree_admin(project_path: &str) -> Result<String, String> {
    git_repo_root(project_path)
        .map(|repo_root| normalize_path(&repo_root))
        .ok_or_else(|| "当前目录不在 git 仓库中，无法管理 worktree".to_string())
}

fn reset_all_managed_worktrees_internal(
    project_path: &str,
    clean_untracked: bool,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<usize, String> {
    let repo_root = repo_root_for_worktree_admin(project_path)?;
    let base_branch = git_branch_name(&repo_root)
        .filter(|branch| valid_branch_name(branch))
        .unwrap_or_else(|| "main".to_string());

    emit_stdout_log(
        sender,
        format!(
            "[WORKTREE CLEANUP] 开始一键清理 repo_root={} base_branch={} clean_untracked={}",
            repo_root, base_branch, clean_untracked
        ),
    );

    let output =
        run_git_maintenance_logged(sender, &repo_root, &["worktree", "list", "--porcelain"])?;
    if !output.status.success() {
        let error = format!("读取 worktree 列表失败: {}", git_output_failure_detail(&output));
        emit_stderr_log(sender, format!("[WORKTREE CLEANUP] {}", error));
        return Err(error);
    }

    let mut reset_count = 0usize;
    for entry in parse_worktree_list(&String::from_utf8_lossy(&output.stdout)) {
        let Some((normalized, branch)) = cleanup_worktree_candidate(&repo_root, entry) else {
            continue;
        };

        emit_stdout_log(
            sender,
            format!(
                "[WORKTREE CLEANUP] 处理工作区 path={} branch={}",
                normalized,
                branch.as_deref().unwrap_or("HEAD")
            ),
        );

        abort_git_in_progress_states(&normalized, sender);
        let reset_output =
            run_git_maintenance_logged(sender, &normalized, &["reset", "--hard", &base_branch])?;
        if !reset_output.status.success() {
            let error = format!(
                "重置 worktree 失败 path={} detail={}",
                normalized,
                git_output_failure_detail(&reset_output)
            );
            emit_stderr_log(sender, format!("[WORKTREE CLEANUP] {}", error));
            return Err(error);
        }

        if clean_untracked {
            let clean_output = run_git_maintenance_logged(sender, &normalized, &["clean", "-fd"])?;
            if !clean_output.status.success() {
                let error = format!(
                    "清理 worktree 失败 path={} detail={}",
                    normalized,
                    git_output_failure_detail(&clean_output)
                );
                emit_stderr_log(sender, format!("[WORKTREE CLEANUP] {}", error));
                return Err(error);
            }
        }
        emit_stdout_log(sender, format!("[WORKTREE CLEANUP] 工作区已重置 path={}", normalized));
        reset_count = reset_count.saturating_add(1);
    }

    emit_stdout_log(
        sender,
        format!("[WORKTREE CLEANUP] 一键清理完成，共处理 {} 个工作区", reset_count),
    );
    Ok(reset_count)
}

fn cleanup_worktree_candidate(
    repo_root_normalized: &str,
    entry: WorktreeEntry,
) -> Option<(String, Option<String>)> {
    let normalized = normalize_path(&entry.path);
    if normalized == repo_root_normalized {
        return None;
    }
    if entry.branch.as_deref().is_none_or(|branch| branch == "HEAD") {
        return None;
    }
    Some((normalized, entry.branch))
}

pub fn reset_all_managed_worktrees(
    project_path: &str,
    clean_untracked: bool,
) -> Result<usize, String> {
    reset_all_managed_worktrees_internal(project_path, clean_untracked, None)
}

pub fn reset_all_managed_worktrees_with_logs(
    project_path: &str,
    clean_untracked: bool,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<usize, String> {
    reset_all_managed_worktrees_internal(project_path, clean_untracked, sender)
}

pub fn reset_all_managed_worktrees_async(
    project_path: String,
    clean_untracked: bool,
) -> impl std::future::Future<Output = Result<usize, String>> {
    reset_all_managed_worktrees_async_with_logs(project_path, clean_untracked, None)
}

pub async fn reset_all_managed_worktrees_async_with_logs(
    project_path: String,
    clean_untracked: bool,
    log_sender: Option<Sender<TaskLogStream>>,
) -> Result<usize, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::task::spawn_blocking(move || {
            reset_all_managed_worktrees_with_logs(
                &project_path,
                clean_untracked,
                log_sender.as_ref(),
            )
        })
        .await
        .unwrap_or_else(|error| Err(format!("worktree 一键清理线程异常: {}", error)))
    }
    #[cfg(target_arch = "wasm32")]
    {
        reset_all_managed_worktrees_with_logs(&project_path, clean_untracked, log_sender.as_ref())
    }
}

fn delete_all_managed_worktrees_internal(
    project_path: &str,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<usize, String> {
    let repo_root = repo_root_for_worktree_admin(project_path)?;

    emit_stdout_log(
        sender,
        format!("[WORKTREE DELETE] 开始删除所有受管 worktree repo_root={}", repo_root),
    );

    let worktree_list_output =
        run_git_maintenance_logged(sender, &repo_root, &["worktree", "list", "--porcelain"])?;
    if !worktree_list_output.status.success() {
        let error =
            format!("读取 worktree 列表失败: {}", git_output_failure_detail(&worktree_list_output));
        emit_stderr_log(sender, format!("[WORKTREE DELETE] {}", error));
        return Err(error);
    }

    let managed_entries = delete_worktree_candidates(
        &repo_root,
        parse_worktree_list(&String::from_utf8_lossy(&worktree_list_output.stdout)),
    );

    if managed_entries.is_empty() {
        emit_stdout_log(sender, "[WORKTREE DELETE] 没有可删除的受管 worktree".to_string());
        return Ok(0);
    }

    emit_force_delete_state_logs(&managed_entries, sender);

    let mut deleted_count = 0usize;
    let deleted_paths = managed_entries.iter().map(|(path, _)| path.clone()).collect::<Vec<_>>();
    for (path, branch) in managed_entries {
        emit_stdout_log(
            sender,
            format!(
                "[WORKTREE DELETE] 删除槽位 path={} branch={}",
                path,
                branch.as_deref().unwrap_or("HEAD")
            ),
        );
        abort_git_in_progress_states(&path, sender);
        let remove_output = run_git_logged(
            sender,
            &repo_root,
            &["worktree", "remove", "--force", "--force", &path],
        )?;
        if !remove_output.status.success() {
            let error = format!(
                "删除 worktree 失败 path={} detail={}",
                path,
                git_output_failure_detail(&remove_output)
            );
            emit_stderr_log(sender, format!("[WORKTREE DELETE] {}", error));
            return Err(error);
        }
        if let Some(branch) = branch.filter(|value| value != "HEAD" && !value.trim().is_empty()) {
            let branch_output = run_git_logged(sender, &repo_root, &["branch", "-D", &branch])?;
            if !branch_output.status.success() {
                emit_stderr_log(
                    sender,
                    format!(
                        "[WORKTREE DELETE] 删除分支失败 branch={} detail={}",
                        branch,
                        git_output_failure_detail(&branch_output)
                    ),
                );
            }
        }
        emit_stdout_log(sender, format!("[WORKTREE DELETE] 槽位已删除 path={}", path));
        deleted_count = deleted_count.saturating_add(1);
    }

    if let Ok(mut claimed) = claimed_worktrees().lock() {
        for path in &deleted_paths {
            claimed.remove(path);
        }
    }

    emit_stdout_log(sender, format!("[WORKTREE DELETE] 删除完成，共处理 {} 个槽位", deleted_count));
    Ok(deleted_count)
}

fn emit_force_delete_state_logs(
    managed_entries: &[(String, Option<String>)],
    sender: Option<&Sender<TaskLogStream>>,
) {
    match claimed_worktrees().lock() {
        Ok(claimed) => {
            let claimed_count =
                managed_entries.iter().filter(|(path, _)| claimed.contains(path)).count();
            if claimed_count > 0 {
                emit_stderr_log(
                    sender,
                    format!(
                        "[WORKTREE DELETE] 强制删除继续执行，忽略 {} 个正在使用中的 worktree",
                        claimed_count
                    ),
                );
            }
        }
        Err(_) => {
            emit_stderr_log(sender, "[WORKTREE DELETE] worktree 占用锁定失败，强制删除继续执行");
        }
    }
}

fn delete_worktree_candidates(
    repo_root: &str,
    entries: Vec<WorktreeEntry>,
) -> Vec<(String, Option<String>)> {
    entries
        .into_iter()
        .filter_map(|entry| {
            let normalized = normalize_path(&entry.path);
            if !is_managed_worktree_path(repo_root, &normalized) {
                return None;
            }
            Some((normalized, entry.branch))
        })
        .collect()
}

pub fn delete_all_managed_worktrees(project_path: &str) -> Result<usize, String> {
    delete_all_managed_worktrees_internal(project_path, None)
}

pub fn delete_all_managed_worktrees_with_logs(
    project_path: &str,
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<usize, String> {
    delete_all_managed_worktrees_internal(project_path, sender)
}

pub fn delete_all_managed_worktrees_async(
    project_path: String,
) -> impl std::future::Future<Output = Result<usize, String>> {
    delete_all_managed_worktrees_async_with_logs(project_path, None)
}

pub async fn delete_all_managed_worktrees_async_with_logs(
    project_path: String,
    log_sender: Option<Sender<TaskLogStream>>,
) -> Result<usize, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::task::spawn_blocking(move || {
            delete_all_managed_worktrees_with_logs(&project_path, log_sender.as_ref())
        })
        .await
        .unwrap_or_else(|error| Err(format!("worktree 批量删除线程异常: {}", error)))
    }
    #[cfg(target_arch = "wasm32")]
    {
        delete_all_managed_worktrees_with_logs(&project_path, log_sender.as_ref())
    }
}

fn commit_merge_all_worktrees_internal(
    project_path: &str,
    tasks: &[Task],
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<usize, String> {
    let repo_root = repo_root_for_worktree_admin(project_path)
        .map_err(|_| "当前目录不在 git 仓库中，无法批量合并".to_string())?;
    let base_branch = git_branch_name(&repo_root)
        .filter(|branch| branch != "HEAD")
        .unwrap_or_else(|| "main".to_string());

    emit_stdout_log(
        sender,
        format!(
            "[WORKTREE MERGE] 开始批量合并 repo_root={} base_branch={}",
            repo_root, base_branch
        ),
    );

    let worktree_list_output =
        run_git_maintenance_logged(sender, &repo_root, &["worktree", "list", "--porcelain"])?;
    if !worktree_list_output.status.success() {
        let error =
            format!("读取 worktree 列表失败: {}", git_output_failure_detail(&worktree_list_output));
        emit_stderr_log(sender, format!("[WORKTREE MERGE] {}", error));
        return Err(error);
    }
    let worktrees = parse_worktree_list(&String::from_utf8_lossy(&worktree_list_output.stdout));
    if worktrees.is_empty() {
        return Err("没有可用的 worktree".to_string());
    }

    let checkout_output = run_git_logged(sender, &repo_root, &["checkout", &base_branch])?;
    if !checkout_output.status.success() {
        let error = format!(
            "切换主分支失败 branch={} detail={}",
            base_branch,
            git_output_failure_detail(&checkout_output)
        );
        emit_stderr_log(sender, format!("[WORKTREE MERGE] {}", error));
        return Err(error);
    }

    let mut merged_branches = Vec::new();
    let mut failed_commit_branches = Vec::new();
    let mut failed_merge_branches = Vec::new();

    for entry in worktrees {
        let worktree_path = normalize_path(&entry.path);
        if worktree_path == repo_root || !Path::new(&worktree_path).is_dir() {
            continue;
        }

        let Some(branch_name) =
            entry.branch.filter(|branch| branch != "HEAD" && !branch.is_empty())
        else {
            continue;
        };

        emit_stdout_log(
            sender,
            format!("[WORKTREE MERGE] 处理分支 {} path={}", branch_name, worktree_path),
        );

        let add_output = run_git_logged(sender, &worktree_path, &["add", "."])?;
        if !add_output.status.success() {
            failed_commit_branches.push(format!(
                "{}(git add 失败: {})",
                branch_name,
                git_output_failure_detail(&add_output)
            ));
            continue;
        }

        if git_has_staged_changes(&worktree_path)? {
            let commit_message = tasks
                .iter()
                .find(|task| task.merge_source_branch.as_deref() == Some(branch_name.as_str()))
                .map(|task| build_task_commit_message(task, &format!("提交 {}", branch_name)))
                .unwrap_or_else(|| format!("提交 {}", branch_name));
            let commit_output = run_git_logged(
                sender,
                &worktree_path,
                &["commit", "--no-verify", "-m", &commit_message],
            )?;
            if !commit_output.status.success() {
                failed_commit_branches.push(format!(
                    "{}({})",
                    branch_name,
                    git_output_failure_detail(&commit_output)
                ));
                continue;
            }
        }

        let checkout_output = run_git_logged(sender, &repo_root, &["checkout", &base_branch])?;
        if !checkout_output.status.success() {
            let error = format!(
                "切回主分支失败 branch={} detail={}",
                base_branch,
                git_output_failure_detail(&checkout_output)
            );
            emit_stderr_log(sender, format!("[WORKTREE MERGE] {}", error));
            return Err(error);
        }

        let branch_ref = format!("refs/heads/{}", branch_name);
        let show_ref_output =
            run_git_logged(sender, &repo_root, &["show-ref", "--verify", "--quiet", &branch_ref])?;
        if !show_ref_output.status.success() {
            continue;
        }

        let merge_base_output = run_git_logged(
            sender,
            &repo_root,
            &["merge-base", "--is-ancestor", &branch_name, &base_branch],
        )?;
        if matches!(merge_base_output.status.code(), Some(0)) {
            continue;
        }
        if !matches!(merge_base_output.status.code(), Some(1)) {
            failed_merge_branches.push(format!(
                "{}(merge-base 检查失败: {})",
                branch_name,
                git_output_failure_detail(&merge_base_output)
            ));
            continue;
        }

        let merge_output = run_git_logged(
            sender,
            &repo_root,
            &["merge", "--no-verify", "--no-edit", "--no-stat", &branch_name],
        )?;
        if merge_output.status.success() {
            merged_branches.push(branch_name);
            continue;
        }

        failed_merge_branches.push(format!(
            "{}({})",
            branch_name,
            git_output_failure_detail(&merge_output)
        ));
        let abort_output = run_git_logged(sender, &repo_root, &["merge", "--abort"])?;
        if !abort_output.status.success() {
            let _ = run_git_logged(sender, &repo_root, &["reset", "--merge"]);
        }
    }

    let checkout_output = run_git_logged(sender, &repo_root, &["checkout", &base_branch])?;
    if !checkout_output.status.success() {
        let error = format!(
            "恢复主分支失败 branch={} detail={}",
            base_branch,
            git_output_failure_detail(&checkout_output)
        );
        emit_stderr_log(sender, format!("[WORKTREE MERGE] {}", error));
        return Err(error);
    }

    if failed_commit_branches.is_empty() && failed_merge_branches.is_empty() {
        emit_stdout_log(
            sender,
            format!("[WORKTREE MERGE] 批量合并完成，共合并 {} 个分支", merged_branches.len()),
        );
        return Ok(merged_branches.len());
    }

    let error = format!(
        "批量合并完成但存在失败 | merged={} | failed_commit={} [{}] | failed_merge={} [{}] | base_branch={}",
        merged_branches.len(),
        failed_commit_branches.len(),
        failed_commit_branches.join(", "),
        failed_merge_branches.len(),
        failed_merge_branches.join(", "),
        base_branch
    );
    emit_stderr_log(sender, format!("[WORKTREE MERGE] {}", error));
    Err(error)
}

pub fn commit_merge_all_worktrees(project_path: &str, tasks: &[Task]) -> Result<usize, String> {
    commit_merge_all_worktrees_internal(project_path, tasks, None)
}

pub fn commit_merge_all_worktrees_with_logs(
    project_path: &str,
    tasks: &[Task],
    sender: Option<&Sender<TaskLogStream>>,
) -> Result<usize, String> {
    commit_merge_all_worktrees_internal(project_path, tasks, sender)
}

pub fn commit_merge_all_worktrees_async(
    project_path: String,
    tasks: Vec<Task>,
) -> impl std::future::Future<Output = Result<usize, String>> {
    commit_merge_all_worktrees_async_with_logs(project_path, tasks, None)
}

pub async fn commit_merge_all_worktrees_async_with_logs(
    project_path: String,
    tasks: Vec<Task>,
    log_sender: Option<Sender<TaskLogStream>>,
) -> Result<usize, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::task::spawn_blocking(move || {
            commit_merge_all_worktrees_with_logs(&project_path, &tasks, log_sender.as_ref())
        })
        .await
        .unwrap_or_else(|error| Err(format!("worktree 批量合并线程异常: {}", error)))
    }
    #[cfg(target_arch = "wasm32")]
    {
        commit_merge_all_worktrees_with_logs(&project_path, &tasks, log_sender.as_ref())
    }
}

#[cfg(test)]
#[path = "worktree_admin_tests.rs"]
mod worktree_admin_tests;
