//! 应用供应商记录、配置与操作客户端。
pub use gateway_app_read::AgentAppsReadError;

#[derive(Debug, Clone)]
pub struct GatewayAppProviderClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppProviderClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod app_providers_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
#[cfg(test)]
mod app_providers_api_tests;
