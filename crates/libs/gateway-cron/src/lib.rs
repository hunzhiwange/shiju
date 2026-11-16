//! Cron 调度、预览与历史领域客户端。

#[derive(Debug, Clone)]
pub struct GatewayCronClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayCronClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod types;
pub use types::*;
mod cron_api;
#[cfg(test)]
mod test_support;

#[cfg(not(target_arch = "wasm32"))]
pub mod handlers;
#[cfg(not(target_arch = "wasm32"))]
pub mod requests;
#[cfg(not(target_arch = "wasm32"))]
pub mod state;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_9_tests;
