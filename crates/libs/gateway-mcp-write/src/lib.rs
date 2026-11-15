//! Typed catalog mutations on the selected gateway.
pub use gateway_app_settings::AgentAppsSettingsError;

#[derive(Debug, Clone)]
pub struct GatewayMcpWriteClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayMcpWriteClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod mcp_management_write;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod mcp_management_write_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
