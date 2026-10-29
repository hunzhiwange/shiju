//! Shared runtime directory names used by VibeWindow crates.
//! 定义各 crate 共享的运行目录名称，并按构建配置选择开发或生产路径。
//! 所有辅助函数只负责拼接路径，不访问文件系统或创建目录。

use std::path::{Path, PathBuf};

/// 用户配置目录下的运行时兼容配置文件名；生产系统配置存储在数据库中。
pub const CONFIG_JSON_FILENAME: &str = "shiju.json";

/// 正式构建在用户主目录下使用的配置目录名。
pub const PROD_HOME_CONFIG_DIR_NAME: &str = ".shiju-agent";
/// 调试构建在用户主目录下使用的隔离配置目录名。
pub const DEV_HOME_CONFIG_DIR_NAME: &str = ".shiju-agent-dev";
/// 正式构建在平台数据目录下使用的应用目录名。
pub const PROD_APP_DIR_NAME: &str = "shiju";
/// 调试构建在平台数据目录下使用的隔离应用目录名。
pub const DEV_APP_DIR_NAME: &str = "shiju-dev";
/// 任务工作树在用户配置目录下的固定子目录名。
pub const TASK_WORKTREES_DIR_NAME: &str = "task-worktrees";

/// 当前构建配置对应的用户配置目录名。
#[cfg(debug_assertions)]
pub const HOME_CONFIG_DIR_NAME: &str = DEV_HOME_CONFIG_DIR_NAME;
/// 当前构建配置对应的用户配置目录名。
#[cfg(not(debug_assertions))]
pub const HOME_CONFIG_DIR_NAME: &str = PROD_HOME_CONFIG_DIR_NAME;

/// 调试构建使用的紧急停止状态文件路径。
#[cfg(debug_assertions)]
pub const ESTOP_STATE_FILE_PATH: &str = "~/.shiju-agent-dev/estop-state.json";
/// 正式构建使用的紧急停止状态文件路径。
#[cfg(not(debug_assertions))]
pub const ESTOP_STATE_FILE_PATH: &str = "~/.shiju-agent/estop-state.json";

/// 当前构建配置对应的平台应用目录名。
#[cfg(debug_assertions)]
pub const APP_DIR_NAME: &str = DEV_APP_DIR_NAME;
/// 当前构建配置对应的平台应用目录名。
#[cfg(not(debug_assertions))]
pub const APP_DIR_NAME: &str = PROD_APP_DIR_NAME;

/// 返回指定主目录下、与当前构建配置隔离的用户配置目录。
pub fn home_config_dir(home: impl AsRef<Path>) -> PathBuf {
    home.as_ref().join(HOME_CONFIG_DIR_NAME)
}

/// 返回共享 SQLite 数据库文件路径，不保证父目录已经存在。
pub fn global_database_path(home: impl AsRef<Path>) -> PathBuf {
    home_config_dir(home).join("database").join("vibe-window.sqlite3")
}

/// 返回共享数据库配置文件路径，不保证父目录已经存在。
pub fn global_database_config_path(home: impl AsRef<Path>) -> PathBuf {
    home_config_dir(home).join("database").join("config.json")
}

/// 返回任务工作树的统一存储目录。
pub fn task_worktrees_dir(home: impl AsRef<Path>) -> PathBuf {
    home_config_dir(home).join(TASK_WORKTREES_DIR_NAME)
}

/// 用户级工作流计划存储根目录。
pub fn task_plan_storage_dir(home: impl AsRef<Path>) -> PathBuf {
    home_config_dir(home).join("storage").join("plan")
}

/// 返回以文件系统根目录为基准的当前构建配置目录。
pub fn root_config_dir() -> PathBuf {
    PathBuf::from("/").join(HOME_CONFIG_DIR_NAME)
}

/// 将配置目录内的相对路径表示为 `~/...` 形式。
///
/// 输入开头的 `/` 会被移除；空输入只返回配置目录本身。
pub fn tilde_config_path(relative: &str) -> String {
    let relative = relative.trim_start_matches('/');
    if relative.is_empty() {
        format!("~/{HOME_CONFIG_DIR_NAME}")
    } else {
        format!("~/{HOME_CONFIG_DIR_NAME}/{relative}")
    }
}

/// 返回当前构建配置的紧急停止状态文件路径字符串。
pub fn estop_state_file_path() -> String {
    ESTOP_STATE_FILE_PATH.to_string()
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod paths_tests;
