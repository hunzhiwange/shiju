//! 通道命令处理模块
//!
//! 本模块提供通道管理命令的处理逻辑，包括：
//! - 列出可用通道及其配置状态
//! - 绑定通道身份标识
//! - 其他通道管理操作的入口
//!
//! # 命令类型
//!
//! 支持的命令由 `ChannelCommands` 枚举定义，包括：
//! - `Start`：启动通道服务（需要在异步运行时中处理）
//! - `Doctor`：诊断通道健康状态（需要在异步运行时中处理）
//! - `List`：列出所有可用通道
//! - `Add`：添加新通道配置
//! - `Remove`：移除通道配置
//! - `BindTelegram`：绑定 Telegram 身份标识

use super::*;
pub(crate) use channel_runtime::manager::command::*;
pub async fn handle_command(command: ChannelCommands, config: &Config) -> Result<()> { channel_runtime::manager::command::handle_command(&super::runtime_host::HOST, command, config).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/command_tests.rs"]
mod command_tests;
