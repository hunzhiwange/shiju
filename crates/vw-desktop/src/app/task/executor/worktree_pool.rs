#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

//! 任务执行器的 worktree_pool.rs 子模块。
//!
//! 桌面端不再持有任务池 worktree 槽位租约真相；本模块只保留普通 worktree
//! 管理仍需要的路径识别、git worktree 列表解析和稳定命名工具。

use super::process_utils::normalize_path;
use super::state::WorktreeEntry;
use super::*;

pub(super) fn parse_worktree_list(output: &str) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch: Option<String> = None;

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(path) = current_path.take() {
                entries.push(WorktreeEntry { path, branch: current_branch.take() });
            }
            current_path = Some(path.trim().to_string());
            current_branch = None;
            continue;
        }
        if let Some(branch) = line.strip_prefix("branch ") {
            current_branch = branch
                .trim()
                .strip_prefix("refs/heads/")
                .map(str::to_string)
                .or_else(|| Some(branch.trim().to_string()));
            continue;
        }
        if line.trim().is_empty() {
            if let Some(path) = current_path.take() {
                entries.push(WorktreeEntry { path, branch: current_branch.take() });
            }
            current_branch = None;
        }
    }

    if let Some(path) = current_path.take() {
        entries.push(WorktreeEntry { path, branch: current_branch.take() });
    }
    entries
}

pub(super) fn worktree_pool_root(repo_root: &str) -> PathBuf {
    let home = super::programs::user_home_dir().unwrap_or_else(|| PathBuf::from(repo_root));
    vw_config_types::paths::task_worktrees_dir(home)
}

fn legacy_worktree_pool_root(repo_root: &str) -> PathBuf {
    Path::new(repo_root)
        .join(vw_config_types::paths::PROD_HOME_CONFIG_DIR_NAME)
        .join(vw_config_types::paths::TASK_WORKTREES_DIR_NAME)
}

fn managed_worktree_roots(repo_root: &str) -> [PathBuf; 2] {
    [worktree_pool_root(repo_root), legacy_worktree_pool_root(repo_root)]
}

fn normalized_path_buf(path: PathBuf) -> PathBuf {
    PathBuf::from(normalize_path(path.to_string_lossy().as_ref()))
}

fn is_pool_slot_path(path: &Path) -> bool {
    path.file_name().and_then(|value| value.to_str()).is_some_and(|name| name.starts_with("slot-"))
}

pub(super) fn is_managed_worktree_path(repo_root: &str, path: &str) -> bool {
    let normalized_path = PathBuf::from(normalize_path(path));
    if !is_pool_slot_path(&normalized_path) {
        return false;
    }
    managed_worktree_roots(repo_root)
        .into_iter()
        .map(normalized_path_buf)
        .any(|root| normalized_path.starts_with(root))
}

#[cfg(test)]
#[cfg(test)]
pub(super) fn sanitize_branch_token(value: &str) -> String {
    value
        .chars()
        .map(
            |ch| if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') { ch } else { '-' },
        )
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
#[path = "worktree_pool_tests.rs"]
mod worktree_pool_tests;
