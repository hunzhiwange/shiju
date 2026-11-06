use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::{GatewayClient, provider_collection_path};
use vw_api_types::{agent_apps::AppId, app_providers::editor_merge::*};
impl GatewayClient {
    pub async fn app_provider_editor_open(
        &self,
        app: AppId,
        id: &str,
        body: &EditorOpen,
        token: &str,
    ) -> Result<EditorView, Error> {
        self.editor_request(app, id, "open", body, token).await
    }
    pub async fn app_provider_editor_save(
        &self,
        app: AppId,
        id: &str,
        body: &EditorSave,
        token: &str,
    ) -> Result<EditorResult, Error> {
        self.editor_request(app, id, "save", body, token).await
    }
    async fn editor_request<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        app: AppId,
        id: &str,
        action: &str,
        body: &B,
        token: &str,
    ) -> Result<T, Error> {
        if id.is_empty()
            || id.len() > 128
            || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err(Error::Transport("invalid provider identity".into()));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (app, action, body, token);
            Err(Error::Transport("editor requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let request = self
                .dynamic_host_client
                .post(format!(
                    "{}{}/{id}/editor/{action}",
                    self.endpoint.base_url(),
                    provider_collection_path(app)
                ))
                .header("x-agent-apps-token", credential)
                .json(body);
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("editor request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response.json().await.map_err(|_| Error::Decode("invalid editor response".into()))
        }
    }
}
