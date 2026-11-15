//! Fixed Dsh operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_terminal::dsh::*;

impl GatewayClient {
    pub async fn dsh_form_read(
        &self,
        id: &str,
        body: &DshTarget,
        token: &str,
    ) -> Result<DshFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .dsh_form_read(id, body, token).await
    }
    pub async fn dsh_form_create(
        &self,
        id: &str,
        body: &DshFormWrite,
        token: &str,
    ) -> Result<DshResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .dsh_form_create(id, body, token).await
    }
    pub async fn dsh_form_update(
        &self,
        id: &str,
        body: &DshFormWrite,
        token: &str,
    ) -> Result<DshResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .dsh_form_update(id, body, token).await
    }

    pub async fn dsh_provider_remove_live(
        &self,
        id: &str,
        body: &DshTarget,
        token: &str,
    ) -> Result<DshResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .dsh_provider_remove_live(id, body, token).await
    }

    pub async fn dsh_provider_save(
        &self,
        id: &str,
        body: &DshWrite,
        token: &str,
    ) -> Result<DshResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .dsh_provider_save(id, body, token).await
    }
    pub async fn dsh_provider_add(
        &self,
        id: &str,
        body: &DshTarget,
        token: &str,
    ) -> Result<DshResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .dsh_provider_add(id, body, token).await
    }
    pub async fn dsh_state(&self, body: &DshTarget, token: &str) -> Result<DshState, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .dsh_state(body, token).await
    }
}
