//! Shared runtime directory names used by VibeWindow crates.

use std::path::{Path, PathBuf};

pub const PROD_HOME_CONFIG_DIR_NAME: &str = ".vibewindow";
pub const DEV_HOME_CONFIG_DIR_NAME: &str = ".vibewindowdev";
pub const PROD_APP_DIR_NAME: &str = "vibewindow";
pub const DEV_APP_DIR_NAME: &str = "vibewindowdev";
pub const TASK_WORKTREES_DIR_NAME: &str = "task-worktrees";

#[cfg(debug_assertions)]
pub const HOME_CONFIG_DIR_NAME: &str = DEV_HOME_CONFIG_DIR_NAME;
#[cfg(not(debug_assertions))]
pub const HOME_CONFIG_DIR_NAME: &str = PROD_HOME_CONFIG_DIR_NAME;

#[cfg(debug_assertions)]
pub const ESTOP_STATE_FILE_PATH: &str = "~/.vibewindowdev/estop-state.json";
#[cfg(not(debug_assertions))]
pub const ESTOP_STATE_FILE_PATH: &str = "~/.vibewindow/estop-state.json";

#[cfg(debug_assertions)]
pub const APP_DIR_NAME: &str = DEV_APP_DIR_NAME;
#[cfg(not(debug_assertions))]
pub const APP_DIR_NAME: &str = PROD_APP_DIR_NAME;

pub fn home_config_dir(home: impl AsRef<Path>) -> PathBuf {
    home.as_ref().join(HOME_CONFIG_DIR_NAME)
}

pub fn global_database_path(home: impl AsRef<Path>) -> PathBuf {
    home_config_dir(home).join("database").join("vibe-window.sqlite3")
}

pub fn global_database_config_path(home: impl AsRef<Path>) -> PathBuf {
    home_config_dir(home).join("database").join("config.json")
}

pub fn task_worktrees_dir(home: impl AsRef<Path>) -> PathBuf {
    home_config_dir(home).join(TASK_WORKTREES_DIR_NAME)
}

/// 用户级工作流计划存储根目录。
pub fn task_plan_storage_dir(home: impl AsRef<Path>) -> PathBuf {
    home_config_dir(home).join("storage").join("plan")
}

pub fn root_config_dir() -> PathBuf {
    PathBuf::from("/").join(HOME_CONFIG_DIR_NAME)
}

pub fn tilde_config_path(relative: &str) -> String {
    let relative = relative.trim_start_matches('/');
    if relative.is_empty() {
        format!("~/{HOME_CONFIG_DIR_NAME}")
    } else {
        format!("~/{HOME_CONFIG_DIR_NAME}/{relative}")
    }
}

pub fn estop_state_file_path() -> String {
    ESTOP_STATE_FILE_PATH.to_string()
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod paths_tests;
