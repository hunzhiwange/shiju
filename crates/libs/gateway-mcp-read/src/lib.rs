//! Typed reads of the selected gateway's managed MCP catalog.
pub use gateway_app_read::AgentAppsReadError;

#[derive(Debug, Clone)]
pub struct GatewayMcpReadClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayMcpReadClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod mcp_management_read;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod mcp_management_read_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
