use super::GatewayClient;
use vw_api_app_provider_edit::icons::AppProviderIcon;
use vw_api_app_provider_edit::icons::AppProviderIconChange;
use vw_api_app_identity::{AppId};
impl GatewayClient {
    pub async fn app_provider_icon(&self, app: AppId, id: &str) -> Result<AppProviderIcon, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_icon(app, id).await
    }
    pub async fn app_provider_icon_change(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderIconChange,
    ) -> Result<AppProviderIcon, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_icon_change(app, id, request).await
    }
}
