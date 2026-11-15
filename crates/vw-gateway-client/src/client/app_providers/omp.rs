//! Fixed Omp operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::omp::*;
impl GatewayClient {
    pub async fn omp_form_read(
        &self,
        id: &str,
        body: &OmpTarget,
        token: &str,
    ) -> Result<OmpFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .omp_form_read(id, body, token).await
    }
    pub async fn omp_form_create(
        &self,
        id: &str,
        body: &OmpFormWrite,
        token: &str,
    ) -> Result<OmpResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .omp_form_create(id, body, token).await
    }
    pub async fn omp_form_update(
        &self,
        id: &str,
        body: &OmpFormWrite,
        token: &str,
    ) -> Result<OmpResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .omp_form_update(id, body, token).await
    }

    pub async fn omp_provider_save(
        &self,
        id: &str,
        body: &OmpWrite,
        token: &str,
    ) -> Result<OmpResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .omp_provider_save(id, body, token).await
    }
    pub async fn omp_provider_add(
        &self,
        id: &str,
        body: &OmpTarget,
        token: &str,
    ) -> Result<OmpResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .omp_provider_add(id, body, token).await
    }
    pub async fn omp_provider_remove_live(
        &self,
        id: &str,
        body: &OmpTarget,
        token: &str,
    ) -> Result<OmpResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .omp_provider_remove_live(id, body, token).await
    }
    pub async fn omp_import(
        &self,
        body: &OmpTarget,
        token: &str,
    ) -> Result<OmpImportResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .omp_import(body, token).await
    }
    pub async fn omp_state(&self, body: &OmpTarget, token: &str) -> Result<OmpState, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .omp_state(body, token).await
    }
}
