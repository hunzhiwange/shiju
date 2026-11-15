//! 应用授权兼容门面，行为由 gateway-app-auth 拥有。
use super::GatewayClient;
use vw_api_app_auth::*;
pub use gateway_app_auth::{AgentAppsAuthBinding, AgentAppsAuthCredentials, AgentAppsAuthReadError};

impl GatewayClient {
    pub async fn agent_apps_auth_accounts(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
    ) -> Result<Vec<AuthAccount>, AgentAppsAuthReadError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_accounts(host_id, service, credentials).await
    }

    pub async fn agent_apps_auth_status(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
    ) -> Result<AuthStatus, AgentAppsAuthReadError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_status(host_id, service, credentials).await
    }

    pub async fn agent_apps_auth_bindings(
        &self,
        host_id: &str,
        service: AuthService,
        account_id: Option<&str>,
        credentials: &AgentAppsAuthCredentials<'_>,
    ) -> Result<Vec<AgentAppsAuthBinding>, AgentAppsAuthReadError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_bindings(host_id, service, account_id, credentials).await
    }
}
