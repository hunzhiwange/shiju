//! Native Pi prompt resources and saved-state synchronization.
mod resources;
pub use resources::*;

#[derive(Debug, Clone)]
pub struct GatewayAppPromptResourcesClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppPromptResourcesClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}
