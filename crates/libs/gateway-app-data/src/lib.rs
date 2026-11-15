//! Application data migration, backup and authenticated inventory transport.
mod migration_sdk;
mod backup_sdk;
mod backup_inventory_api;

pub use migration_sdk::{DataMigrationError, DataExportTransfer};
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
#[cfg(all(test, not(target_arch = "wasm32")))]
mod migration_sdk_tests;
