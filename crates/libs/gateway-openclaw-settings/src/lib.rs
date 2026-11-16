//! OpenClaw 设置网关客户端，以及 macOS 服务端路由。
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

#[cfg(target_os = "macos")]
mod handlers;
#[cfg(target_os = "macos")]
pub use handlers::{Authorization, Host, router, router_with_policy};

#[cfg(target_os = "macos")]
mod authorization;
#[cfg(target_os = "macos")]
pub use authorization::authorized_router;
