//! 识矩账号敏感请求与脱敏凭据契约。
#[derive(Debug, Clone)]
pub struct GatewayShijuAccountClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayShijuAccountClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod shiju_account_api;
pub use shiju_account_api::*;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod shiju_account_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
