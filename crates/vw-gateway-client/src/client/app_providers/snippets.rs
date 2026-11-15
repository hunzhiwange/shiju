use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_provider_edit::snippets::*;
use vw_api_app_identity::{AppId};
impl GatewayClient {
    pub async fn app_provider_snippet_get(
        &self,
        app: AppId,
        request: &SnippetRead,
        token: &str,
    ) -> Result<ConfigSnippet, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_snippet_get(app, request, token).await
    }
    pub async fn app_provider_snippet_save(
        &self,
        app: AppId,
        request: &SnippetWrite,
        token: &str,
    ) -> Result<ConfigSnippet, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_snippet_save(app, request, token).await
    }
    pub async fn app_provider_snippet_preview(
        &self,
        app: AppId,
        request: &SnippetWrite,
        token: &str,
    ) -> Result<ConfigSnippet, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_snippet_preview(app, request, token).await
    }
    pub async fn app_provider_snippet_extract(
        &self,
        app: AppId,
        request: &SnippetExtract,
        token: &str,
    ) -> Result<ConfigSnippet, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_snippet_extract(app, request, token).await
    }
}
