//! Scoped aggregation membership and publication requests.
#[derive(Debug, Clone)]
pub struct GatewayAggregationClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAggregationClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod aggregation_members_api;
mod aggregation_publish_api;
pub use aggregation_members_api::AggregationApiError;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_members_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_publish_api_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod aggregation_clients;

#[cfg(not(target_arch = "wasm32"))]
pub mod aggregation_codex_settings;

#[cfg(not(target_arch = "wasm32"))]
pub mod publication_host;
#[cfg(not(target_arch = "wasm32"))]
pub mod aggregation_members;

#[cfg(not(target_arch = "wasm32"))]
pub mod aggregation_desktop;

#[cfg(not(target_arch = "wasm32"))]
pub mod exit_host;
#[cfg(not(target_arch = "wasm32"))]
pub mod aggregation_exit;
