//! 应用高可用队列、健康、路由与策略客户端。
#[derive(Debug, Clone)]
pub struct GatewayAppFailoverClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppFailoverClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod agent_apps_high_availability_api;
pub use agent_apps_high_availability_api::AgentAppsHighAvailabilityError;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_high_availability_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
