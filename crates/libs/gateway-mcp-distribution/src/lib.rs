//! MCP 导入、绑定、批量分发与重同步客户端。
pub use gateway_app_settings::AgentAppsSettingsError;
pub use gateway_app_lifecycle::AgentAppsLifecycleError;

#[derive(Debug, Clone)]
pub struct GatewayMcpDistributionClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayMcpDistributionClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod mcp_management_distribution;
pub use mcp_management_distribution::*;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod mcp_management_distribution_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
