use super::GatewayClient;
use vw_api_types::app_providers::universal_sync::{UniversalSyncRequest, UniversalSyncResult};
impl GatewayClient {
    pub async fn universal_provider_sync(
        &self,
        id: &str,
        request: &UniversalSyncRequest,
    ) -> Result<Vec<UniversalSyncResult>, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .universal_provider_sync(id, request).await
    }
}
