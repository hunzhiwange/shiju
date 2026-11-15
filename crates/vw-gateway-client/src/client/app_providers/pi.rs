//! Fixed Pi operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::pi::*;
impl GatewayClient {
    pub async fn pi_usage_save(
        &self,
        id: &str,
        body: &PiUsageWrite,
        token: &str,
    ) -> Result<PiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .pi_usage_save(id, body, token).await
    }

    pub async fn pi_form_read(
        &self,
        id: &str,
        body: &PiTarget,
        token: &str,
    ) -> Result<PiFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .pi_form_read(id, body, token).await
    }
    pub async fn pi_form_create(
        &self,
        id: &str,
        body: &PiFormWrite,
        token: &str,
    ) -> Result<PiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .pi_form_create(id, body, token).await
    }
    pub async fn pi_form_update(
        &self,
        id: &str,
        body: &PiFormWrite,
        token: &str,
    ) -> Result<PiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .pi_form_update(id, body, token).await
    }

    pub async fn pi_provider_save(
        &self,
        id: &str,
        body: &PiWrite,
        token: &str,
    ) -> Result<PiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .pi_provider_save(id, body, token).await
    }
    pub async fn pi_provider_add(
        &self,
        id: &str,
        body: &PiTarget,
        token: &str,
    ) -> Result<PiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .pi_provider_add(id, body, token).await
    }
    pub async fn pi_provider_remove_live(
        &self,
        id: &str,
        body: &PiTarget,
        token: &str,
    ) -> Result<PiResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .pi_provider_remove_live(id, body, token).await
    }
    pub async fn pi_import(&self, body: &PiTarget, token: &str) -> Result<PiImportResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .pi_import(body, token).await
    }
    pub async fn pi_state(&self, body: &PiTarget, token: &str) -> Result<PiState, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .pi_state(body, token).await
    }
}
