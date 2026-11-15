//! 应用目录、安装观察和自定义图标的网关客户端。
pub use gateway_http::AgentAppsReadError;

#[derive(Debug, Clone)]
pub struct GatewayAppReadClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppReadClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod agent_apps_read_api;
mod agent_apps_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_read_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
