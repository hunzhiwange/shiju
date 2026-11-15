//! Host-owned usage pricing, model sync configuration and display preferences.
#[derive(Debug, Clone)]
pub struct GatewayUsagePriceClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayUsagePriceClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod usage_price_types;
pub use usage_price_types::*;
mod usage_price_api;
pub use usage_price_api::UsagePriceError;
