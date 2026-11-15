//! Fixed OpenCode operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_extended::opencode::*;
impl GatewayClient {
    pub async fn opencode_provider_save(
        &self,
        id: &str,
        body: &OpenCodeWrite,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .opencode_provider_save(id, body, token).await
    }
    pub async fn opencode_form_read(
        &self,
        id: &str,
        body: &OpenCodeTarget,
        token: &str,
    ) -> Result<OpenCodeFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .opencode_form_read(id, body, token).await
    }
    pub async fn opencode_form_create(
        &self,
        id: &str,
        body: &OpenCodeFormWrite,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .opencode_form_create(id, body, token).await
    }
    pub async fn opencode_form_update(
        &self,
        id: &str,
        body: &OpenCodeFormWrite,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .opencode_form_update(id, body, token).await
    }
    pub async fn opencode_provider_add(
        &self,
        id: &str,
        body: &OpenCodeTarget,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .opencode_provider_add(id, body, token).await
    }
    pub async fn opencode_provider_remove_live(
        &self,
        id: &str,
        body: &OpenCodeTarget,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .opencode_provider_remove_live(id, body, token).await
    }
    pub async fn opencode_default_model(
        &self,
        body: &OpenCodeDefaultModel,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .opencode_default_model(body, token).await
    }
    pub async fn opencode_import(
        &self,
        body: &OpenCodeTarget,
        token: &str,
    ) -> Result<OpenCodeImportResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .opencode_import(body, token).await
    }
    pub async fn opencode_state(
        &self,
        body: &OpenCodeTarget,
        token: &str,
    ) -> Result<OpenCodeState, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .opencode_state(body, token).await
    }
}
