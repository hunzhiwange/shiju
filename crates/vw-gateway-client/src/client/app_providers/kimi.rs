//! Fixed Kimi operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_terminal::kimi::*;

impl GatewayClient {
    pub async fn kimi_form_read(
        &self,
        id: &str,
        body: &KimiTarget,
        token: &str,
    ) -> Result<KimiFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .kimi_form_read(id, body, token).await
    }
    pub async fn kimi_form_create(
        &self,
        id: &str,
        body: &KimiFormWrite,
        token: &str,
    ) -> Result<KimiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .kimi_form_create(id, body, token).await
    }
    pub async fn kimi_form_update(
        &self,
        id: &str,
        body: &KimiFormWrite,
        token: &str,
    ) -> Result<KimiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .kimi_form_update(id, body, token).await
    }

    pub async fn kimi_provider_editor_save(
        &self,
        id: &str,
        body: &KimiEditorWrite,
        token: &str,
    ) -> Result<KimiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .kimi_provider_editor_save(id, body, token).await
    }

    pub async fn kimi_provider_save(
        &self,
        id: &str,
        body: &KimiWrite,
        token: &str,
    ) -> Result<KimiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .kimi_provider_save(id, body, token).await
    }
    pub async fn kimi_provider_add(
        &self,
        id: &str,
        body: &KimiTarget,
        token: &str,
    ) -> Result<KimiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .kimi_provider_add(id, body, token).await
    }
    pub async fn kimi_provider_remove_live(
        &self,
        id: &str,
        body: &KimiTarget,
        token: &str,
    ) -> Result<KimiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .kimi_provider_remove_live(id, body, token).await
    }
    pub async fn kimi_import(
        &self,
        body: &KimiTarget,
        token: &str,
    ) -> Result<KimiImportResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .kimi_import(body, token).await
    }
    pub async fn kimi_state(&self, body: &KimiTarget, token: &str) -> Result<KimiState, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .kimi_state(body, token).await
    }
}
