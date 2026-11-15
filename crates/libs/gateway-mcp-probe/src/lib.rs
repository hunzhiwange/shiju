//! Explicit MCP discovery and diagnostic jobs on the authorized execution host.
pub use gateway_app_settings::AgentAppsSettingsError;

#[derive(Debug, Clone)]
pub struct GatewayMcpProbeClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayMcpProbeClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod mcp_management_probe;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod mcp_management_probe_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
