//! Project-scoped Codex settings and client restart requests.
pub use gateway_app_settings::AgentAppsSettingsError;

#[derive(Debug, Clone)]
pub struct GatewayAggregationCodexClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAggregationCodexClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }

    pub fn endpoint(&self) -> &gateway_endpoint::GatewayEndpoint {
        self.transport.endpoint()
    }
}

mod aggregation_codex_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_codex_api_tests;
