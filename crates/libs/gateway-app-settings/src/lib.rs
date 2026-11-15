//! 应用偏好设置与远端启动客户端。
#[derive(Debug, Clone)]
pub struct GatewayAppSettingsClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppSettingsClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod agent_apps_settings_api;
pub use agent_apps_settings_api::AgentAppsSettingsError;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_settings_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
