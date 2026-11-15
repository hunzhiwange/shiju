//! Fixed Codex operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_profiles::codex_catalog::*;
impl GatewayClient {
    pub async fn codex_provider_save(
        &self,
        id: &str,
        body: &CodexWrite,
        token: &str,
    ) -> Result<CodexResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .codex_provider_save(id, body, token).await
    }
    pub async fn codex_provider_switch(
        &self,
        id: &str,
        body: &CodexTarget,
        token: &str,
    ) -> Result<CodexResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .codex_provider_switch(id, body, token).await
    }
    pub async fn codex_provider_reapply(
        &self,
        id: &str,
        body: &CodexTarget,
        token: &str,
    ) -> Result<CodexResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .codex_provider_reapply(id, body, token).await
    }
    pub async fn codex_form_read(
        &self,
        id: &str,
        body: &CodexTarget,
        token: &str,
    ) -> Result<CodexFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .codex_form_read(id, body, token).await
    }
    pub async fn codex_form_create(
        &self,
        id: &str,
        body: &CodexFormWrite,
        token: &str,
    ) -> Result<CodexResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .codex_form_create(id, body, token).await
    }
    pub async fn codex_form_update(
        &self,
        id: &str,
        body: &CodexFormWrite,
        token: &str,
    ) -> Result<CodexResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .codex_form_update(id, body, token).await
    }
}
