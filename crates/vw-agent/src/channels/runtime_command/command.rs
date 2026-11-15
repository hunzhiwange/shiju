//! 通道命令解析的兼容导出。
pub(crate) use channel_runtime::manager::runtime_command::*;

#[cfg(test)]
#[path = "command_tests.rs"]
mod command_tests;
