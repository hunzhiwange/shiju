use super::GatewayClient;
use vw_api_types::{app_providers::AppProviderSummary};
use vw_api_app_identity::{AppId};
impl GatewayClient {
    pub async fn app_provider_list(&self, app: AppId) -> Result<Vec<AppProviderSummary>, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_list(app).await
    }
}

impl GatewayClient {
    /// Native transport only; token authorizes the selected execution host independently.
    pub async fn app_provider_catalog(
        &self,
        app: AppId,
        host: &str,
        token: &str,
    ) -> Result<
        vw_api_types::app_providers::catalog::ProviderCatalog,
        super::super::agent_apps_read_api::AgentAppsReadError,
    > {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_catalog(app, host, token).await
    }
}
