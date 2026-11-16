mod config_api;
mod lifecycle_api;
mod outbound_api;
mod outbound_probe;
mod outbound_scan;

pub use lifecycle_api::AppRoutingError;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod config_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod lifecycle_api_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod outbound_api_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod outbound_probe_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod outbound_scan_tests;

mod request_status;

mod mode_attach;

mod client_host;

use gateway_http::GatewayTransport;

#[derive(Debug, Clone)]
pub struct GatewayAppRoutingClient {
    transport: GatewayTransport,
}

impl GatewayAppRoutingClient {
    pub fn new(transport: GatewayTransport) -> Self {
        Self { transport }
    }

    pub fn endpoint(&self) -> &gateway_endpoint::GatewayEndpoint {
        self.transport.endpoint()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;

#[cfg(target_os = "macos")]
pub mod server;

#[cfg(all(test, target_os = "macos"))]
mod host_migration_7_tests;
