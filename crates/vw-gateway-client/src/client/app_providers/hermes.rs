//! Fixed Hermes operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::hermes::*;
impl GatewayClient {
    pub async fn hermes_form_read(
        &self,
        id: &str,
        body: &HermesTarget,
        token: &str,
    ) -> Result<HermesFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .hermes_form_read(id, body, token).await
    }
    pub async fn hermes_form_create(
        &self,
        id: &str,
        body: &HermesFormWrite,
        token: &str,
    ) -> Result<HermesResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .hermes_form_create(id, body, token).await
    }
    pub async fn hermes_form_update(
        &self,
        id: &str,
        body: &HermesFormWrite,
        token: &str,
    ) -> Result<HermesResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .hermes_form_update(id, body, token).await
    }

    pub async fn hermes_provider_save(
        &self,
        id: &str,
        body: &HermesWrite,
        token: &str,
    ) -> Result<HermesResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .hermes_provider_save(id, body, token).await
    }
    pub async fn hermes_provider_add(
        &self,
        id: &str,
        body: &HermesTarget,
        token: &str,
    ) -> Result<HermesResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .hermes_provider_add(id, body, token).await
    }
    pub async fn hermes_provider_remove_live(
        &self,
        id: &str,
        body: &HermesTarget,
        token: &str,
    ) -> Result<HermesResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .hermes_provider_remove_live(id, body, token).await
    }
    pub async fn hermes_import(
        &self,
        body: &HermesTarget,
        token: &str,
    ) -> Result<HermesImportResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .hermes_import(body, token).await
    }
    pub async fn hermes_state(
        &self,
        body: &HermesTarget,
        token: &str,
    ) -> Result<HermesState, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .hermes_state(body, token).await
    }
}
