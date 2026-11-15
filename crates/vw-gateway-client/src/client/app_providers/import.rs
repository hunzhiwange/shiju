//! Native import returns counts only; errors discard upstream payloads.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_edit::import::*;
impl GatewayClient {
    pub async fn provider_import(
        &self,
        app: vw_api_app_identity::AppId,
        body: &ProviderImport,
        token: &str,
    ) -> Result<ProviderImportResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .provider_import(app, body, token).await
    }
}
