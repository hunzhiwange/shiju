//! 通道的无宿主运行时依赖支撑。
pub mod config;
pub mod format;
pub mod http;
pub mod markdown_chunk;
pub mod outbound_delivery;
pub mod history;
pub mod outbound_format;
pub mod scope;
pub mod routing;
pub mod prompt;
pub mod session;
pub mod session_events;
pub mod runtime_state;
pub mod session_turn;

#[cfg(test)]
mod root_contract_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_42_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_43_tests;
#[cfg(not(target_arch = "wasm32"))]
pub mod blocking;
