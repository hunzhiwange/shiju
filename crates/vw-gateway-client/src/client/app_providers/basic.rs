use super::GatewayClient;
use vw_api_types::{app_providers::basic::ProviderBasic, app_providers::basic::ProviderBasicWrite};
use vw_api_app_identity::{AppId};
impl GatewayClient {
    pub async fn app_provider_basic(&self, app: AppId, id: &str) -> Result<ProviderBasic, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_basic(app, id).await
    }
    pub async fn app_provider_basic_write(
        &self,
        app: AppId,
        id: &str,
        value: &ProviderBasicWrite,
    ) -> Result<ProviderBasic, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_basic_write(app, id, value).await
    }
}
