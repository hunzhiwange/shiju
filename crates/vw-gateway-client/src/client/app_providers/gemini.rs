//! Fixed Gemini operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_profiles::gemini::*;
impl GatewayClient {
    pub async fn gemini_provider_save(
        &self,
        id: &str,
        body: &GeminiWrite,
        token: &str,
    ) -> Result<GeminiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .gemini_provider_save(id, body, token).await
    }
    pub async fn gemini_provider_switch(
        &self,
        id: &str,
        body: &GeminiTarget,
        token: &str,
    ) -> Result<GeminiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .gemini_provider_switch(id, body, token).await
    }
    pub async fn gemini_provider_reapply(
        &self,
        id: &str,
        body: &GeminiTarget,
        token: &str,
    ) -> Result<GeminiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .gemini_provider_reapply(id, body, token).await
    }
    pub async fn gemini_form_read(
        &self,
        id: &str,
        body: &GeminiTarget,
        token: &str,
    ) -> Result<GeminiFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .gemini_form_read(id, body, token).await
    }
    pub async fn gemini_form_create(
        &self,
        id: &str,
        body: &GeminiFormWrite,
        token: &str,
    ) -> Result<GeminiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .gemini_form_create(id, body, token).await
    }
    pub async fn gemini_form_update(
        &self,
        id: &str,
        body: &GeminiFormWrite,
        token: &str,
    ) -> Result<GeminiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .gemini_form_update(id, body, token).await
    }
}
