//! Host usage scan operations, maintenance jobs and revision subscriptions.
#[derive(Debug, Clone)]
pub struct GatewayUsageScanClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayUsageScanClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod usage_scan_api;
pub use usage_scan_api::*;
