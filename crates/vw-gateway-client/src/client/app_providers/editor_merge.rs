use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::{app_providers::editor_merge::*};
use vw_api_app_identity::{AppId};
impl GatewayClient {
    pub async fn app_provider_editor_open(
        &self,
        app: AppId,
        id: &str,
        body: &EditorOpen,
        token: &str,
    ) -> Result<EditorView, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_editor_open(app, id, body, token).await
    }
    pub async fn app_provider_editor_save(
        &self,
        app: AppId,
        id: &str,
        body: &EditorSave,
        token: &str,
    ) -> Result<EditorResult, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_editor_save(app, id, body, token).await
    }
}
