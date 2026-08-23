//! 记忆运行时数据目录的派生与隔离规则。
//!
//! 工作区路径只作为稳定身份来源，运行数据始终写入用户级目录或临时回退目录。

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// 返回项目级记忆数据的用户作用域目录。
///
/// 运行时记忆不得写入项目仓库；工作区路径仅用于派生稳定项目标识。
pub(crate) fn project_data_dir(workspace_dir: &Path) -> Result<PathBuf> {
    Ok(user_worktree_root()?.join("projects").join(project_state_id(workspace_dir)))
}

/// 返回项目数据目录；用户目录不可用时退回系统临时目录。
pub(crate) fn project_data_dir_best_effort(workspace_dir: &Path) -> PathBuf {
    project_data_dir(workspace_dir).unwrap_or_else(|_| fallback_project_data_dir(workspace_dir))
}

fn fallback_project_data_dir(workspace_dir: &Path) -> PathBuf {
    std::env::temp_dir()
        .join(vw_config_types::paths::APP_DIR_NAME)
        .join("worktree")
        .join("projects")
        .join(project_state_id(workspace_dir))
}

/// 返回工作区级运行状态的用户作用域目录。
pub(crate) fn workspace_data_dir(workspace_dir: &Path) -> Result<PathBuf> {
    Ok(user_worktree_root()?.join("workspaces").join(workspace_state_id(workspace_dir)))
}

#[cfg(not(target_arch = "wasm32"))]
fn user_worktree_root() -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory for memory data")?;
    Ok(vw_config_types::paths::home_config_dir(home).join("worktree"))
}

#[cfg(target_arch = "wasm32")]
fn user_worktree_root() -> Result<PathBuf> {
    Ok(vw_config_types::paths::root_config_dir().join("worktree"))
}

fn project_state_id(workspace_dir: &Path) -> String {
    let workspace = canonical_path(workspace_dir);
    let identity = git_common_dir(&workspace).unwrap_or(workspace);
    scoped_id(&identity)
}

fn workspace_state_id(workspace_dir: &Path) -> String {
    scoped_id(&canonical_path(workspace_dir))
}

fn canonical_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path)
    };
    std::fs::canonicalize(&absolute).unwrap_or(absolute)
}

fn git_common_dir(workspace_dir: &Path) -> Option<PathBuf> {
    crate::git_repository::discover(workspace_dir).map(|repository| repository.common_dir)
}

fn scoped_id(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let hash = hex::encode(hasher.finalize());
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("workspace");
    format!("{}-{}", safe_name(name), &hash[..16])
}

fn safe_name(value: &str) -> String {
    let safe = value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_ascii_lowercase();
    if safe.is_empty() { "workspace".to_string() } else { safe }
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod paths_tests;
