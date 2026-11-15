use super::GatewayClient;
use vw_api_types::app_providers::universal::{UniversalProviderSummary, UniversalProviderWrite};
impl GatewayClient {

    pub async fn universal_provider_save(
        &self,
        id: &str,
        request: &UniversalProviderWrite,
    ) -> Result<UniversalProviderSummary, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .universal_provider_save(id, request).await
    }
    pub async fn universal_provider_get(
        &self,
        id: &str,
    ) -> Result<UniversalProviderSummary, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .universal_provider_get(id).await
    }
    pub async fn universal_providers_list(&self) -> Result<Vec<UniversalProviderSummary>, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .universal_providers_list().await
    }
    pub async fn universal_provider_delete(&self, id: &str) -> Result<bool, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .universal_provider_delete(id).await
    }
}
