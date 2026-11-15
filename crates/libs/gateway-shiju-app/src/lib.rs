//! 识矩应用目录、安装状态、密钥绑定与启动传输契约。
#[derive(Debug, Clone)]
pub struct GatewayShijuAppClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayShijuAppClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod shiju_app_api;
pub use shiju_app_api::*;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod shiju_app_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
