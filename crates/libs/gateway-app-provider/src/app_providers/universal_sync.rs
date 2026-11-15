use super::GatewayAppProviderClient;
use vw_api_app_provider_edit::universal_sync::UniversalSyncRequest;
use vw_api_app_provider_edit::universal_sync::UniversalSyncResult;
impl GatewayAppProviderClient {
    pub async fn universal_provider_sync(
        &self,
        id: &str,
        request: &UniversalSyncRequest,
    ) -> Result<Vec<UniversalSyncResult>, String> {
        let path = super::universal::path(id)?;
        self.universal_response(
            self.transport.client().post(format!("{}{path}/sync", self.transport.endpoint().base_url())).json(request),
        )
        .await
    }
}
