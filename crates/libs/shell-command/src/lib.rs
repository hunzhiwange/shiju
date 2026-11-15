//! Shell 管理模块
//!
//! 本模块负责 Shell 环境的检测、选择和进程管理，为代理系统提供可靠的命令执行环境。
//!
//! # 核心功能
//!
//! - **Shell 自动检测**：根据操作系统和环境变量自动选择合适的 Shell
//! - **Shell 黑名单机制**：排除不兼容的 Shell（如 fish、nushell）
//! - **进程树终止**：跨平台终止进程及其所有子进程
//!
//! # Shell 选择优先级
//!
//! 1. 优先使用环境变量 `SHELL` 指定的 Shell（除非在黑名单中）
//! 2. Windows：尝试 Git Bash → COMSPEC (cmd.exe)
//! 3. macOS：默认使用 `/bin/zsh`
//! 4. 其他 Unix：尝试 `bash` → `/bin/sh`

mod environment;
mod command;
mod process;

pub use environment::{effective_path_env, shell_profile_env_var, resolve_executable, PREFERRED, ACCEPTABLE};
pub use command::{apply_augmented_path_std_command, std_command, std_system_command, apply_git_defaults_std_command, git_std_command};
#[cfg(feature = "shell-tokio")]
pub use command::{apply_augmented_path_tokio_command, tokio_command, tokio_system_command, apply_git_defaults_tokio_command, git_tokio_command};
pub use process::kill_tree;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
