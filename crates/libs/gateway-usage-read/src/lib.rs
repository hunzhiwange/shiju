//! Read-only gateway usage ledger queries and wire responses.
#[derive(Debug, Clone)]
pub struct GatewayUsageReadClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayUsageReadClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod usage_read_api;
mod usage_read_types;
pub use usage_read_api::{UsageReadError, UsageReadScope};
pub use usage_read_types::*;
