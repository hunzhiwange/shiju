use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_edit::preview::*;
use vw_api_app_identity::{AppId};
impl GatewayClient {
    /// Native transport only; token authorizes the selected execution host independently.
    pub async fn app_provider_preview(
        &self,
        app: AppId,
        request_body: &ProviderPreviewRequest,
        token: &str,
    ) -> Result<ProviderPreview, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_preview(app, request_body, token).await
    }

    pub async fn app_provider_saved_preview(
        &self,
        app: AppId,
        request_body: &SavedProviderPreviewRequest,
        token: &str,
    ) -> Result<SavedProviderPreview, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_saved_preview(app, request_body, token).await
    }
}
