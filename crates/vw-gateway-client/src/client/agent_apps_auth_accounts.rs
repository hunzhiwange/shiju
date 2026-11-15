//! 应用授权兼容门面，行为由 gateway-app-auth 拥有。
use super::{GatewayClient, AgentAppsAuthCredentials, AgentAppsAuthError};
use vw_api_app_auth::*;
pub use gateway_app_auth::{AgentAppsAuthLogout};

impl GatewayClient {
    pub async fn agent_apps_auth_bind(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        provider_id: &str,
        account_id: Option<&str>,
    ) -> Result<(), AgentAppsAuthError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_bind(host_id, service, credentials, provider_id, account_id).await
    }

    pub async fn agent_apps_auth_set_default(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        account_id: &str,
    ) -> Result<(), AgentAppsAuthError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_set_default(host_id, service, credentials, account_id).await
    }

    pub async fn agent_apps_auth_delete(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        account_id: &str,
    ) -> Result<(), AgentAppsAuthError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_delete(host_id, service, credentials, account_id).await
    }

    pub async fn agent_apps_auth_logout(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        selection: AgentAppsAuthLogout<'_>,
    ) -> Result<(), AgentAppsAuthError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_logout(host_id, service, credentials, selection).await
    }

    pub async fn agent_apps_auth_revoke(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        account_id: &str,
    ) -> Result<(), AgentAppsAuthError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_revoke(host_id, service, credentials, account_id).await
    }
}
