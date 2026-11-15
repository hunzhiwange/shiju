//! 网关应用授权查询、流程与账号操作。
#[derive(Debug, Clone)]
pub struct GatewayAppAuthClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppAuthClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self { Self { transport } }
}
#[cfg(not(target_arch = "wasm32"))]
mod agent_apps_auth_read;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_auth_read_tests;
#[cfg(not(target_arch = "wasm32"))]
mod agent_apps_auth_flow;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_auth_flow_tests;
#[cfg(not(target_arch = "wasm32"))]
mod agent_apps_auth_accounts;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod agent_apps_auth_accounts_tests;
#[cfg(not(target_arch = "wasm32"))]
pub use agent_apps_auth_read::{AgentAppsAuthBinding, AgentAppsAuthCredentials, AgentAppsAuthReadError};
#[cfg(not(target_arch = "wasm32"))]
pub use agent_apps_auth_flow::{AgentAppsAuthError, AgentAppsAuthError as AgentAppsAuthFlowError};
#[cfg(not(target_arch = "wasm32"))]
pub use agent_apps_auth_accounts::AgentAppsAuthLogout;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
