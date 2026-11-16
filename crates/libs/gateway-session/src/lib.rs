//! 会话生命周期、消息与 UI 持久化领域客户端。
mod session_api;
mod ui_load;
pub use ui_load::SessionUiLoadResult;

#[derive(Debug, Clone)]
pub struct GatewaySessionClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewaySessionClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

#[cfg(test)]
mod test_support;

/// Archived tree validation shared by session handlers.
pub mod archived_delete;

/// Pending raw artifacts awaiting durable UI snapshot acknowledgement.
#[cfg(not(target_arch = "wasm32"))]
pub mod raw_commit;

#[cfg(not(target_arch = "wasm32"))]
pub mod message_host;
#[cfg(not(target_arch = "wasm32"))]
pub mod message_ops;

#[cfg(not(target_arch = "wasm32"))]
pub mod acp_host;
#[cfg(not(target_arch = "wasm32"))]
pub mod acp_options;

#[cfg(not(target_arch = "wasm32"))]
pub mod shared;

#[cfg(not(target_arch = "wasm32"))]
pub mod session_host;

#[cfg(not(target_arch = "wasm32"))]
pub mod session_ops;

#[cfg(not(target_arch = "wasm32"))]
pub mod archived_routes;

#[cfg(not(target_arch = "wasm32"))]
pub mod stream;
#[cfg(not(target_arch = "wasm32"))]
pub mod stream_host;
#[cfg(not(target_arch = "wasm32"))]
pub mod prompt_adapter;

#[cfg(not(target_arch = "wasm32"))]
pub mod ui_host;
#[cfg(not(target_arch = "wasm32"))]
pub mod ui_ops;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod stream_value_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_11_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_10_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_12_tests;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod host_lifecycle_acceptance;

#[cfg(not(target_arch = "wasm32"))]
pub mod native_ui;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod child_acceptance;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod fork_acceptance;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod stream_persistence_acceptance;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod stream_acceptance;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod profile_acceptance;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod profile_acceptance_host;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod acp_source_acceptance;

#[cfg(all(feature = "test-support", not(target_arch = "wasm32")))]
pub mod archived_acceptance;
