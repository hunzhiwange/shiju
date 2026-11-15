//! 原生动态工具宿主客户端，复用禁止重定向与重试的传输。

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct GatewayHostToolsClient {
    transport: gateway_http::GatewayTransport,
}

#[cfg(not(target_arch = "wasm32"))]
impl GatewayHostToolsClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod dynamic_tools_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod dynamic_tools_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
