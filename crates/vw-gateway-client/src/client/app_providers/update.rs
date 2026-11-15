use super::GatewayClient;
use vw_api_app_provider_base::AppProviderSummary;
use vw_api_app_provider_base::update::AppProviderUpdate;
use vw_api_app_identity::{AppId};
impl GatewayClient {
    /// Partial catalog update without native activation.
    pub async fn app_provider_update(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderUpdate,
    ) -> Result<AppProviderSummary, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_update(app, id, request).await
    }
}
