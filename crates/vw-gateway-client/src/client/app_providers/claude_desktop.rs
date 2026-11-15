//! Fixed Claude Desktop operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::claude_desktop::*;
impl GatewayClient {
    pub async fn claude_desktop_provider_import_live(
        &self,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<ClaudeDesktopResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_desktop_provider_import_live(body, token).await
    }
    pub async fn claude_desktop_provider_save(
        &self,
        id: &str,
        body: &ClaudeDesktopWrite,
        token: &str,
    ) -> Result<ClaudeDesktopResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_desktop_provider_save(id, body, token).await
    }
    pub async fn claude_desktop_provider_switch(
        &self,
        id: &str,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<ClaudeDesktopResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_desktop_provider_switch(id, body, token).await
    }
    pub async fn claude_desktop_provider_reapply(
        &self,
        id: &str,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<ClaudeDesktopResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_desktop_provider_reapply(id, body, token).await
    }
}
