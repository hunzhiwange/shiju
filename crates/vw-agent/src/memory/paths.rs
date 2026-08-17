use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Return the user-scoped data root for project-level memory.
///
/// Runtime memory data must never be written into project repositories. The
/// workspace path is only used as a stable identity source for the project
/// directory under the active home config directory's `worktree/projects`.
pub(crate) fn project_data_dir(workspace_dir: &Path) -> Result<PathBuf> {
    Ok(user_worktree_root()?.join("projects").join(project_state_id(workspace_dir)))
}

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

/// Return the user-scoped data root for workspace-level runtime state.
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
