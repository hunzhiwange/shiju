//! MCP inspection and host-bound application calls.

#[derive(Debug, Clone)]
pub struct GatewayMcpClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayMcpClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod mcp_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod mcp_api_tests;
