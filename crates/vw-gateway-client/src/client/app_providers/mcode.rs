//! Fixed Mcode operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_terminal::mcode::*;

impl GatewayClient {
    pub async fn mcode_form_read(
        &self,
        id: &str,
        body: &McodeTarget,
        token: &str,
    ) -> Result<McodeFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .mcode_form_read(id, body, token).await
    }
    pub async fn mcode_form_create(
        &self,
        id: &str,
        body: &McodeFormWrite,
        token: &str,
    ) -> Result<McodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .mcode_form_create(id, body, token).await
    }
    pub async fn mcode_form_update(
        &self,
        id: &str,
        body: &McodeFormWrite,
        token: &str,
    ) -> Result<McodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .mcode_form_update(id, body, token).await
    }

    pub async fn mcode_provider_editor_save(
        &self,
        id: &str,
        body: &McodeEditorWrite,
        token: &str,
    ) -> Result<McodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .mcode_provider_editor_save(id, body, token).await
    }

    pub async fn mcode_provider_save(
        &self,
        id: &str,
        body: &McodeWrite,
        token: &str,
    ) -> Result<McodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .mcode_provider_save(id, body, token).await
    }
    pub async fn mcode_provider_add(
        &self,
        id: &str,
        body: &McodeTarget,
        token: &str,
    ) -> Result<McodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .mcode_provider_add(id, body, token).await
    }
    pub async fn mcode_provider_remove_live(
        &self,
        id: &str,
        body: &McodeTarget,
        token: &str,
    ) -> Result<McodeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .mcode_provider_remove_live(id, body, token).await
    }
    pub async fn mcode_import(
        &self,
        body: &McodeTarget,
        token: &str,
    ) -> Result<McodeImportResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .mcode_import(body, token).await
    }
    pub async fn mcode_state(&self, body: &McodeTarget, token: &str) -> Result<McodeState, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .mcode_state(body, token).await
    }
}
