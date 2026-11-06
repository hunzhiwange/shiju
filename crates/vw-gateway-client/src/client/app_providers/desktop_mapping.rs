//! Catalog mapping operations; response never asserts a running proxy.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::{claude_desktop::*, desktop_mapping::*};
impl GatewayClient {
    pub async fn claude_desktop_default_routes(
        &self,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<Vec<DesktopDefaultRoute>, Error> {
        self.claude_desktop_request("model-mapping", "POST", "/defaults", body, token).await
    }
    pub async fn claude_desktop_import_claude(
        &self,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<DesktopImportResult, Error> {
        self.claude_desktop_request("providers", "POST", "/import-claude", body, token).await
    }
    pub async fn claude_desktop_mapping_save(
        &self,
        id: &str,
        body: &DesktopMappingWrite,
        token: &str,
    ) -> Result<ClaudeDesktopResult, Error> {
        self.claude_desktop_request(id, "PUT", "/model-mapping", body, token).await
    }
}
