//! Catalog mapping operations; response never asserts a running proxy.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::{claude_desktop::*, desktop_mapping::*};
impl GatewayClient {
    pub async fn claude_desktop_form_read(
        &self,
        id: &str,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<DesktopFormView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_desktop_form_read(id, body, token).await
    }

    pub async fn claude_desktop_default_routes(
        &self,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<Vec<DesktopDefaultRoute>, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_desktop_default_routes(body, token).await
    }
    pub async fn claude_desktop_import_claude(
        &self,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<DesktopImportResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_desktop_import_claude(body, token).await
    }
    pub async fn claude_desktop_mapping_save(
        &self,
        id: &str,
        body: &DesktopMappingWrite,
        token: &str,
    ) -> Result<ClaudeDesktopResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .claude_desktop_mapping_save(id, body, token).await
    }
}
