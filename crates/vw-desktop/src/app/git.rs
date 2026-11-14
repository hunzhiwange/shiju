//! 桌面 Git 门面，保留终端 Shell 类型并转换到独立后端。

use super::Shell;

pub use desktop_git::{
    DIFF_CONTEXT, git_diff_for_file, git_discard_file, git_revert_line_delete,
    git_revert_line_restore,
};
#[cfg(not(target_arch = "wasm32"))]
pub use desktop_git::git_log;

fn backend_shell(shell: Shell) -> desktop_git::Shell {
    match shell {
        Shell::Bash => desktop_git::Shell::Bash,
        Shell::Zsh => desktop_git::Shell::Zsh,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn git_stage_file(repo_path: &str, file: &str, shell: Shell) -> Result<(), String> {
    desktop_git::git_stage_file(repo_path, file, backend_shell(shell))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn git_commit(repo_path: &str, message: &str, shell: Shell) -> Result<(), String> {
    desktop_git::git_commit(repo_path, message, backend_shell(shell))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn git_commit_with_body(
    repo_path: &str,
    summary: &str,
    body: &str,
    shell: Shell,
) -> Result<(), String> {
    desktop_git::git_commit_with_body(repo_path, summary, body, backend_shell(shell))
}

pub fn git_discard_hunk(
    repo_path: &str,
    file: &str,
    idx: usize,
    shell: Shell,
) -> Result<(), String> {
    desktop_git::git_discard_hunk(repo_path, file, idx, backend_shell(shell))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn git_stage_hunk(repo_path: &str, file: &str, idx: usize, shell: Shell) -> Result<(), String> {
    desktop_git::git_stage_hunk(repo_path, file, idx, backend_shell(shell))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn git_stage_line_insert(
    repo_path: &str,
    file_path: &str,
    new_idx: usize,
    shell: Shell,
) -> Result<(), String> {
    desktop_git::git_stage_line_insert(repo_path, file_path, new_idx, backend_shell(shell))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn git_stage_line_delete(
    repo_path: &str,
    file_path: &str,
    old_idx: usize,
    shell: Shell,
) -> Result<(), String> {
    desktop_git::git_stage_line_delete(repo_path, file_path, old_idx, backend_shell(shell))
}
