//! 授权模型目录读取、编辑与发现。

pub use gateway_http::AgentAppsReadError;

#[derive(Debug, Clone)]
pub struct GatewayModelCatalogClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayModelCatalogClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod model_catalog_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod model_catalog_api_tests;
mod model_catalog_edits_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod model_catalog_edits_api_tests;
mod model_discovery_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod model_discovery_api_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
