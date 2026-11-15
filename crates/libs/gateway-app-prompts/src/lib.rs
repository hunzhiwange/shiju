//! Prompt catalog and native selection client.
mod catalog;
pub use catalog::*;

#[derive(Debug, Clone)]
pub struct GatewayAppPromptsClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppPromptsClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}
