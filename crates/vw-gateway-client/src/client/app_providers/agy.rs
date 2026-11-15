//! Agy domain SDK; launch selection uses the existing agent-apps terminal endpoint.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_profiles::agy::*;
impl GatewayClient {
    pub async fn agy_form_delete(
        &self,
        id: &str,
        body: &AgyTarget,
        token: &str,
    ) -> Result<vw_api_app_provider_edit::delete::AppProviderDeleted, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .agy_form_delete(id, body, token).await
    }

    pub async fn agy_form_read(
        &self,
        id: &str,
        body: &AgyTarget,
        token: &str,
    ) -> Result<AgyFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .agy_form_read(id, body, token).await
    }
    pub async fn agy_form_create(
        &self,
        id: &str,
        body: &AgyFormWrite,
        token: &str,
    ) -> Result<AgySaved, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .agy_form_create(id, body, token).await
    }
    pub async fn agy_form_update(
        &self,
        id: &str,
        body: &AgyFormWrite,
        token: &str,
    ) -> Result<AgySaved, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .agy_form_update(id, body, token).await
    }

    pub async fn agy_provider_save(
        &self,
        id: &str,
        body: &AgyWrite,
        token: &str,
    ) -> Result<AgySaved, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .agy_provider_save(id, body, token).await
    }
    pub async fn agy_provider_switch(
        &self,
        id: &str,
        body: &AgyTarget,
        token: &str,
    ) -> Result<AgySaved, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .agy_provider_switch(id, body, token).await
    }
}
