//! Hermes 记忆与面板的授权网关客户端。
#[derive(Debug, Clone)]
pub struct GatewayHermesMemoryClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayHermesMemoryClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod hermes_memory_api;
pub use hermes_memory_api::{HermesMemoryError, HermesMemoryScope};
pub mod hermes_dashboard_api;
pub use hermes_dashboard_api::HermesDashboardError;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod hermes_memory_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;

#[cfg(target_os = "macos")]
mod server;
#[cfg(target_os = "macos")]
pub use server::{Authorize, AuthorizedHost, router};
