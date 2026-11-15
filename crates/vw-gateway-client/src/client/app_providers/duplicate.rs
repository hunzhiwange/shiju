use super::GatewayClient;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, duplicate::AppProviderDuplicate},
};
use vw_api_app_identity::{AppId};
impl GatewayClient {
    /// Copy a catalog record within its current tenant/application, without activation.
    pub async fn app_provider_duplicate(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderDuplicate,
    ) -> Result<AppProviderSummary, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_duplicate(app, id, request).await
    }
}
