//! OpenClaw 设置的授权网关客户端。
#[derive(Debug, Clone)]
pub struct GatewayOpenClawSettingsClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayOpenClawSettingsClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod openclaw_settings_api;
pub use openclaw_settings_api::{OpenClawSettingsError, OpenClawSettingsScope};
#[cfg(all(test, not(target_arch = "wasm32")))]
mod openclaw_settings_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
