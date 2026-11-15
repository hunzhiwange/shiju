//! Native Claude config operations with explicit host authorization and redacted transport errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_base::claude::*;
impl GatewayClient {
    pub async fn claude_form_read(
        &self,
        id: &str,
        target: &ClaudeTarget,
        token: &str,
    ) -> Result<ClaudeFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_form_read(id, target, token).await
    }

    pub async fn claude_form_create(
        &self,
        id: &str,
        body: &ClaudeFormWrite,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_form_create(id, body, token).await
    }

    pub async fn claude_form_update(
        &self,
        id: &str,
        body: &ClaudeFormWrite,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_form_update(id, body, token).await
    }

    pub async fn claude_provider_create(
        &self,
        id: &str,
        body: &ClaudeWrite,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_provider_create(id, body, token).await
    }
    pub async fn claude_provider_save(
        &self,
        id: &str,
        body: &ClaudeWrite,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_provider_save(id, body, token).await
    }
    pub async fn claude_provider_update(
        &self,
        id: &str,
        body: &ClaudeWrite,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_provider_update(id, body, token).await
    }
    pub async fn claude_provider_switch(
        &self,
        id: &str,
        body: &ClaudeTarget,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_provider_switch(id, body, token).await
    }
    pub async fn claude_provider_reapply(
        &self,
        id: &str,
        body: &ClaudeTarget,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_provider_reapply(id, body, token).await
    }
    pub async fn claude_provider_import(
        &self,
        id: &str,
        body: &ClaudeImport,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_provider_import(id, body, token).await
    }
    pub async fn claude_provider_sync(
        &self,
        id: &str,
        body: &ClaudeSync,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_provider_sync(id, body, token).await
    }
}
