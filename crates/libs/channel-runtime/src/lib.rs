//! 通道监听、反馈与消息执行编排。
mod configured;
pub use configured::ConfiguredChannel;
pub mod manager;
pub mod host;
pub mod startup_host;
pub mod runtime_command;
pub mod command_host;
mod commands;
pub use commands::ChannelCommands;
mod health_state;
pub use health_state::ChannelHealthState;

#[cfg(test)]
mod root_contract_tests;
