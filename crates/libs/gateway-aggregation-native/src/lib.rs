//! Pi and OMP native directory gateway requests.
#[derive(Debug, Clone)]
pub struct GatewayAggregationNativeClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAggregationNativeClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }

    pub fn endpoint(&self) -> &gateway_endpoint::GatewayEndpoint {
        self.transport.endpoint()
    }
}

mod aggregation_native_api;
pub use gateway_aggregation::AggregationApiError;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_native_api_tests;
