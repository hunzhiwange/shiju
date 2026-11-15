//! 桌面设置、技能、插件及宿主集成领域客户端。
mod catalog;
pub use catalog::*;
mod skills;
mod plugins;
mod hooks;
mod external;
mod preferences;
mod service;

#[derive(Debug, Clone)]
pub struct GatewayDesktopSettingsClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayDesktopSettingsClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod desktop_settings_api_tests;
