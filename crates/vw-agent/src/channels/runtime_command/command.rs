//! 非 CLI 通道运行时命令的解析与分类。
//!
//! 本模块把聊天消息中的斜杠命令和受限自然语言意图转换为明确枚举，供处理层
//! 统一执行。授权相关命令在这里保持可识别但不直接放权，实际权限检查位于 handler。

pub(crate) use channel_runtime::manager::runtime_command::*;

#[cfg(test)]
#[path = "command_tests.rs"]
mod command_tests;
