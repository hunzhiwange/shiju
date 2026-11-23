//! Application backup, backup and authenticated inventory transport.
mod backup_transport;
pub use backup_transport::DataBackupError;
mod backup_sdk;
mod backup_inventory_api;

pub use backup_sdk::*;

#[derive(Debug, Clone)]
pub struct GatewayAppDataClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppDataClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }

    pub fn endpoint(&self) -> &gateway_endpoint::GatewayEndpoint {
        self.transport.endpoint()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod backup_sdk_tests;
