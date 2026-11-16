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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_40_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_41_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_42_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_43_tests;
pub mod prompt;

pub mod session_events;

pub mod format;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_14_tests;

#[cfg(any(test, feature = "test-support"))]
pub mod command_test_support;
#[cfg(feature = "test-support")]
pub mod command_history_contracts;

#[cfg(feature = "test-support")]
pub mod command_catalog_contracts;

#[cfg(feature = "test-support")]
pub mod session_contracts;

#[cfg(any(test, feature = "test-support"))]
pub mod dispatch_test_support;

#[cfg(feature = "test-support")]
pub mod dispatch_contracts;

#[cfg(feature = "test-support")]
pub mod cancel_contracts;
