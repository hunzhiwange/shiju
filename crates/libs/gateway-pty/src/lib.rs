//! PTY 生命周期与 WebSocket 领域客户端。

#[derive(Debug, Clone)]
pub struct GatewayPtyClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayPtyClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod pty_api;
pub use pty_api::*;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
