//! 计算机控制管理客户端，复用原生受限传输。

#[derive(Debug, Clone)]
pub struct GatewayComputerControlClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayComputerControlClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod computer_control_api;
pub use computer_control_api::ComputerControlApiError;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod computer_control_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
