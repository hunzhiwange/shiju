use super::GatewayClient;
use vw_api_app_provider_edit::delete::AppProviderDelete;
use vw_api_app_provider_edit::delete::AppProviderDeleted;
use vw_api_app_identity::{AppId};
impl GatewayClient {
    /// Deletes the catalog row only; requires independent host configuration read authorization.
    pub async fn app_provider_delete(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderDelete,
        token: &str,
    ) -> Result<AppProviderDeleted, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_delete(app, id, request, token).await
    }
}
