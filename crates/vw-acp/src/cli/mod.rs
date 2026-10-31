//! CLI 子命令、参数与输出模块集合。

pub mod config_command;
pub mod flags;
pub mod json_output;
pub mod output_render;
pub mod session_target;
pub mod status_command;

#[cfg(test)]
#[path = "output_render_tests.rs"]
mod output_render_tests;
