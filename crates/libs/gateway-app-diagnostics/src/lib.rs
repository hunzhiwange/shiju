//! 应用诊断提交、查询与取消客户端。
#[derive(Debug, Clone)]
pub struct GatewayAppDiagnosticsClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppDiagnosticsClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

pub use gateway_app_settings::AgentAppsSettingsError;
mod agent_apps_diagnostics_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_diagnostics_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
