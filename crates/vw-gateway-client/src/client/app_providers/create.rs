use super::GatewayClient;
use vw_api_types::{app_providers::AppProviderSummary, app_providers::create::AppProviderCreate};
use vw_api_app_identity::{AppId};
impl GatewayClient {
    /// Explicit create-only operation. Never defaults to live activation or upsert.
    pub async fn app_provider_create(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderCreate,
    ) -> Result<AppProviderSummary, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_create(app, id, request).await
    }
}
