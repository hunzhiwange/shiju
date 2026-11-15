//! 应用生命周期请求与操作恢复客户端。
#[derive(Debug, Clone)]
pub struct GatewayAppLifecycleClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppLifecycleClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod agent_apps_lifecycle_api;
pub use agent_apps_lifecycle_api::{AgentAppsLifecycleError, operation_path};
#[cfg(test)]
mod agent_apps_lifecycle_api_tests;
#[cfg(test)]
mod test_support;
