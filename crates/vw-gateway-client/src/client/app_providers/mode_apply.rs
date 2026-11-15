//! Apply an explicit mode operation; responses never expose native configuration.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::{app_providers::mode_apply::*};
use vw_api_app_identity::{AppId};
impl GatewayClient {
    pub async fn provider_mode_apply(
        &self,
        app: AppId,
        request: &ModeApplyRequest,
        token: &str,
    ) -> Result<ModeApplyResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .provider_mode_apply(app, request, token).await
    }
}
