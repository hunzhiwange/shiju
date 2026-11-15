//! 系统状态与发现客户端，共享网关传输及连接池。

#[derive(Debug, Clone)]
pub struct GatewaySystemClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewaySystemClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod system_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod system_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
