//! Fixed OpenClaw operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_extended::openclaw::*;
impl GatewayClient {
    pub async fn openclaw_provider_save(
        &self,
        id: &str,
        body: &OpenClawWrite,
        token: &str,
    ) -> Result<OpenClawResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .openclaw_provider_save(id, body, token).await
    }
    pub async fn openclaw_provider_add(
        &self,
        id: &str,
        body: &OpenClawTarget,
        token: &str,
    ) -> Result<OpenClawResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .openclaw_provider_add(id, body, token).await
    }
    pub async fn openclaw_provider_remove_live(
        &self,
        id: &str,
        body: &OpenClawTarget,
        token: &str,
    ) -> Result<OpenClawResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .openclaw_provider_remove_live(id, body, token).await
    }
    pub async fn openclaw_default_model(
        &self,
        body: &OpenClawDefaultModel,
        token: &str,
    ) -> Result<OpenClawResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .openclaw_default_model(body, token).await
    }
    pub async fn openclaw_import(
        &self,
        body: &OpenClawTarget,
        token: &str,
    ) -> Result<OpenClawImportResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .openclaw_import(body, token).await
    }
    pub async fn openclaw_state(
        &self,
        body: &OpenClawTarget,
        token: &str,
    ) -> Result<OpenClawState, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .openclaw_state(body, token).await
    }
    pub async fn openclaw_form_read(
        &self,
        id: &str,
        body: &OpenClawTarget,
        token: &str,
    ) -> Result<OpenClawFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .openclaw_form_read(id, body, token).await
    }
    pub async fn openclaw_form_create(
        &self,
        id: &str,
        body: &OpenClawFormWrite,
        token: &str,
    ) -> Result<OpenClawResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .openclaw_form_create(id, body, token).await
    }
    pub async fn openclaw_form_update(
        &self,
        id: &str,
        body: &OpenClawFormWrite,
        token: &str,
    ) -> Result<OpenClawResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .openclaw_form_update(id, body, token).await
    }
}
