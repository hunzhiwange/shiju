use super::GatewayClient;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::presentation::{AppProviderPresentation, AppProviderPresentationEntry},
};
use vw_api_app_identity::{AppId};
impl GatewayClient {
    /// Read persisted presentation and apply its category/index/identity ordering.
    /// Callers render these rows directly, including after a refresh or save.
    pub async fn app_provider_presented_catalog(
        &self,
        app: AppId,
        host: &str,
        token: &str,
    ) -> Result<(vw_api_types::app_providers::catalog::ProviderCatalog, Vec<AppProviderPresentationEntry>), String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_presented_catalog(app, host, token).await
    }

    pub async fn app_provider_presentation_set(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderPresentation,
    ) -> Result<AppProviderPresentation, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_presentation_set(app, id, request).await
    }
    pub async fn app_provider_presentation_list(
        &self,
        app: AppId,
    ) -> Result<Vec<AppProviderPresentationEntry>, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_presentation_list(app).await
    }
}
