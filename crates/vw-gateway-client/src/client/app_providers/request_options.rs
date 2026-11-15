use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_edit::request_options::*;
use vw_api_app_identity::{AppId};
impl GatewayClient {
    pub async fn app_provider_request_options_read(
        &self,
        app: AppId,
        id: &str,
        body: &RequestOptionsTarget,
        token: &str,
    ) -> Result<RequestOptionsView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_request_options_read(app, id, body, token).await
    }
    pub async fn app_provider_request_options_save(
        &self,
        app: AppId,
        id: &str,
        body: &RequestOptionsWrite,
        token: &str,
    ) -> Result<RequestOptionsView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_request_options_save(app, id, body, token).await
    }
}
