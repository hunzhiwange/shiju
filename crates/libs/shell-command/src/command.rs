//! 命令构造及 Git 默认环境。

use std::path::PathBuf;
use crate::environment::{common_executable, effective_path_env, resolve_executable};

pub(crate) const GIT_DEFAULT_ENV: &[(&str, &str)] =
    &[("GIT_PAGER", "cat"), ("GIT_MERGE_AUTOEDIT", "no"), ("GIT_EDITOR", "true")];

fn git_program() -> PathBuf {
    if std::env::var_os("VIBEWINDOW_TEST_PREFER_SYSTEM_GIT").is_some()
        && let Some(found) = common_executable("git")
    {
        return found;
    }

    resolved_program_or_original("git")
}

pub(crate) fn resolved_program_or_original(program: &str) -> PathBuf {
    resolve_executable(program).unwrap_or_else(|| PathBuf::from(program))
}

/// 将补全后的 shell PATH 注入标准命令对象。
pub fn apply_augmented_path_std_command(command: &mut std::process::Command) {
    if let Some(path) = effective_path_env() {
        command.env("PATH", path);
    }
}

/// 将补全后的 shell PATH 注入异步命令对象。
#[cfg(feature = "shell-tokio")]
pub fn apply_augmented_path_tokio_command(command: &mut tokio::process::Command) {
    if let Some(path) = effective_path_env() {
        command.env("PATH", path);
    }
}

/// 构建带有补全 PATH 的标准命令对象。
pub fn std_command(program: &str) -> std::process::Command {
    let mut command = std::process::Command::new(resolved_program_or_original(program));
    apply_augmented_path_std_command(&mut command);
    command
}

/// 构建带有补全 PATH 的异步命令对象。
#[cfg(feature = "shell-tokio")]
pub fn tokio_command(program: &str) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(resolved_program_or_original(program));
    apply_augmented_path_tokio_command(&mut command);
    command
}

/// 构建标准系统命令对象。
///
/// 适用于 `open`、`launchctl`、`kill` 等系统级命令；这类命令通常依赖系统默认
/// 环境，而不是用户 shell profile 中的 PATH 扩展。
pub fn std_system_command(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

/// 构建异步系统命令对象。
#[cfg(feature = "shell-tokio")]
pub fn tokio_system_command(program: &str) -> tokio::process::Command {
    tokio::process::Command::new(program)
}

/// 为标准 Git 命令附加统一环境变量。
pub fn apply_git_defaults_std_command(command: &mut std::process::Command) {
    for (key, value) in GIT_DEFAULT_ENV {
        command.env(key, value);
    }
}

/// 为异步 Git 命令附加统一环境变量。
#[cfg(feature = "shell-tokio")]
pub fn apply_git_defaults_tokio_command(command: &mut tokio::process::Command) {
    for (key, value) in GIT_DEFAULT_ENV {
        command.env(key, value);
    }
}

/// 构建标准 Git 命令对象。
pub fn git_std_command() -> std::process::Command {
    let mut command = std::process::Command::new(git_program());
    apply_augmented_path_std_command(&mut command);
    apply_git_defaults_std_command(&mut command);
    command
}

/// 构建异步 Git 命令对象。
#[cfg(feature = "shell-tokio")]
pub fn git_tokio_command() -> tokio::process::Command {
    let mut command = tokio::process::Command::new(git_program());
    apply_augmented_path_tokio_command(&mut command);
    apply_git_defaults_tokio_command(&mut command);
    command
}
