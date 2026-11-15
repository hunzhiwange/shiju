//! 通道监听、反馈与消息执行编排。
mod configured;
pub use configured::ConfiguredChannel;
pub mod manager;
pub mod host;
mod commands;
pub use commands::ChannelCommands;
mod health_state;
pub use health_state::ChannelHealthState;
