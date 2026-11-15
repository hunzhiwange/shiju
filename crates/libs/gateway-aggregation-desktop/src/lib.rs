//! Project-scoped Desktop mapping and publication requests.
pub use gateway_app_settings::AgentAppsSettingsError;

#[derive(Debug, Clone)]
pub struct GatewayAggregationDesktopClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAggregationDesktopClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }

    pub fn endpoint(&self) -> &gateway_endpoint::GatewayEndpoint {
        self.transport.endpoint()
    }
}

mod aggregation_desktop_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_desktop_api_tests;
