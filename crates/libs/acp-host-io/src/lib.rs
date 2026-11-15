//! ACP 宿主文件、权限确认与终端进程能力。

#![allow(clippy::result_large_err)]

pub mod filesystem;
pub mod permission_prompt;
pub mod permissions;
pub mod spawn_command_options;
pub mod terminal;

#[cfg(test)]
mod filesystem_tests;
