use super::GatewayClient;
use vw_api_types::app_providers::universal_sync::{UniversalSyncRequest, UniversalSyncResult};
impl GatewayClient {
    pub async fn universal_provider_sync(
        &self,
        id: &str,
        request: &UniversalSyncRequest,
    ) -> Result<Vec<UniversalSyncResult>, String> {
        let path = super::universal::path(id)?;
        self.universal_response(
            self.client.post(format!("{}{path}/sync", self.endpoint.base_url())).json(request),
        )
        .await
    }
}
