//! Catalog mapping operations; response never asserts a running proxy.
use crate::AgentAppsReadError as Error;
use super::GatewayAppProviderClient;
use vw_api_app_provider_profiles::claude_desktop::*;
use vw_api_app_provider_profiles::desktop_mapping::*;
impl GatewayAppProviderClient {
    pub async fn claude_desktop_form_read(
        &self,
        id: &str,
        body: &ClaudeDesktopTarget,
        token: &str,
    ) -> Result<DesktopFormView, Error> {
        self.claude_desktop_request(id, "POST", "/form/read", body, token).await
    }

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
