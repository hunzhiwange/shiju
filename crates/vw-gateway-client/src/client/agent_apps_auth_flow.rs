//! 应用授权兼容门面，行为由 gateway-app-auth 拥有。
use super::{GatewayClient, AgentAppsAuthCredentials};
use vw_api_app_auth::*;
pub use gateway_app_auth::{AgentAppsAuthError};

impl GatewayClient {
    pub async fn agent_apps_auth_start(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        input: &AuthStartRequest,
    ) -> Result<AuthStartResponse, AgentAppsAuthError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_start(host_id, service, credentials, input).await
    }

    pub async fn agent_apps_auth_poll(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        flow_id: &AuthFlowId,
    ) -> Result<AuthFlow, AgentAppsAuthError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_poll(host_id, service, credentials, flow_id).await
    }

    pub async fn agent_apps_auth_cancel(
        &self,
        host_id: &str,
        service: AuthService,
        credentials: &AgentAppsAuthCredentials<'_>,
        flow_id: &AuthFlowId,
    ) -> Result<(), AgentAppsAuthError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_cancel(host_id, service, credentials, flow_id).await
    }

    pub async fn agent_apps_auth_paste(
        &self,
        host_id: &str,
        credentials: &AgentAppsAuthCredentials<'_>,
        flow_id: &AuthFlowId,
        input: &AuthPasteRequest,
    ) -> Result<(), AgentAppsAuthError> {
        gateway_app_auth::GatewayAppAuthClient::new(self.transport.clone())
            .agent_apps_auth_paste(host_id, credentials, flow_id, input).await
    }
}
