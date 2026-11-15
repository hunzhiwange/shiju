//! OMO and OMO Slim gateway requests.
#[derive(Debug, Clone)]
pub struct GatewayAggregationOmoClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAggregationOmoClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }

    pub fn endpoint(&self) -> &gateway_endpoint::GatewayEndpoint {
        self.transport.endpoint()
    }
}

mod aggregation_omo_api;
pub use aggregation_omo_api::AggregationOmoError;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod aggregation_omo_api_tests;
