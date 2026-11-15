//! 当前用户连接器绑定与授权客户端。
#[derive(Debug, Clone)]
pub struct GatewayShijuConnectorClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayShijuConnectorClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod shiju_connector_api;
pub use shiju_connector_api::*;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
