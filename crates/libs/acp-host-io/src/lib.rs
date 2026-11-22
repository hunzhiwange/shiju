//! ACP 宿主文件、权限确认与终端进程能力。

#![allow(clippy::result_large_err)]

pub mod filesystem;
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
pub mod isolated_stdio;
#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
mod isolated_stdio_tests;
pub mod permission_prompt;
pub mod permissions;
pub mod spawn_command_options;
pub mod terminal;

#[cfg(test)]
mod filesystem_tests;
