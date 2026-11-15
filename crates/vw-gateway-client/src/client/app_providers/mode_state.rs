//! Mode status and decisions never expose the saved client configuration.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::{app_providers::mode_state::*};
use vw_api_app_identity::{AppId};
impl GatewayClient {
    pub async fn provider_mode_state(
        &self,
        app: AppId,
        query: &ModeStateQuery,
        token: &str,
    ) -> Result<ProviderModeStatus, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .provider_mode_state(app, query, token).await
    }
}
