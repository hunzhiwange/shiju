//! 通道运行实现与宿主能力注入。
use super::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_runtime::manager::command::*;
pub async fn handle_command(command: ChannelCommands, config: &Config) -> Result<()> { channel_runtime::manager::command::handle_command(&super::runtime_host::HOST, command, config).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/command_tests.rs"]
mod command_tests;
