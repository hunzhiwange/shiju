//! Fixed Grok operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::grok::*;
impl GatewayClient {
    pub async fn grok_provider_save(
        &self,
        id: &str,
        body: &GrokWrite,
        token: &str,
    ) -> Result<GrokResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .grok_provider_save(id, body, token).await
    }
    pub async fn grok_provider_switch(
        &self,
        id: &str,
        body: &GrokTarget,
        token: &str,
    ) -> Result<GrokResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .grok_provider_switch(id, body, token).await
    }
    pub async fn grok_provider_reapply(
        &self,
        id: &str,
        body: &GrokTarget,
        token: &str,
    ) -> Result<GrokResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .grok_provider_reapply(id, body, token).await
    }
    pub async fn grok_form_read(
        &self,
        id: &str,
        body: &GrokTarget,
        token: &str,
    ) -> Result<GrokFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .grok_form_read(id, body, token).await
    }
    pub async fn grok_form_create(
        &self,
        id: &str,
        body: &GrokFormWrite,
        token: &str,
    ) -> Result<GrokResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .grok_form_create(id, body, token).await
    }
    pub async fn grok_form_update(
        &self,
        id: &str,
        body: &GrokFormWrite,
        token: &str,
    ) -> Result<GrokResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .grok_form_update(id, body, token).await
    }
}
