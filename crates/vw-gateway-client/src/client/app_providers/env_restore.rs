use super::GatewayClient;
use vw_api_app_identity::{AppId};
impl GatewayClient {
    /// Native transport only; token authorizes the selected execution host independently.
    pub async fn provider_env_restore(
        &self,
        app: AppId,
        request_body: &vw_api_types::app_providers::env_restore::EnvRestoreRequest,
        token: &str,
    ) -> Result<
        vw_api_types::app_providers::env_restore::EnvRestoreResult,
        super::super::agent_apps_read_api::AgentAppsReadError,
    > {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .provider_env_restore(app, request_body, token).await
    }
}
