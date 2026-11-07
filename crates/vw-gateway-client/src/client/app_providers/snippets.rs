use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::{GatewayClient, provider_collection_path};
use vw_api_types::{agent_apps::AppId, app_providers::snippets::*};
impl GatewayClient {
    pub async fn app_provider_snippet_get(
        &self,
        app: AppId,
        request: &SnippetRead,
        token: &str,
    ) -> Result<ConfigSnippet, Error> {
        self.snippet_request(app, "read", request, token).await
    }
    pub async fn app_provider_snippet_save(
        &self,
        app: AppId,
        request: &SnippetWrite,
        token: &str,
    ) -> Result<ConfigSnippet, Error> {
        self.snippet_request(app, "save", request, token).await
    }
    pub async fn app_provider_snippet_preview(
        &self,
        app: AppId,
        request: &SnippetWrite,
        token: &str,
    ) -> Result<ConfigSnippet, Error> {
        self.snippet_request(app, "preview", request, token).await
    }
    pub async fn app_provider_snippet_extract(
        &self,
        app: AppId,
        request: &SnippetExtract,
        token: &str,
    ) -> Result<ConfigSnippet, Error> {
        self.snippet_request(app, "extract", request, token).await
    }
    async fn snippet_request<B: serde::Serialize>(
        &self,
        app: AppId,
        action: &str,
        body: &B,
        token: &str,
    ) -> Result<ConfigSnippet, Error> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (app, action, body, token);
            Err(Error::Transport("snippets require native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let url = format!(
                "{}{}/config/snippet",
                self.endpoint.base_url(),
                provider_collection_path(app)
            );
            let builder = match action {
                "read" => self.dynamic_host_client.get(url).query(body),
                "save" => self.dynamic_host_client.put(url).json(body),
                "preview" => self.dynamic_host_client.post(format!("{url}/preview")).json(body),
                "extract" => self.dynamic_host_client.post(format!("{url}/extract")).json(body),
                _ => unreachable!(),
            }
            .header("x-agent-apps-token", credential);
            let response = crate::http::apply_request_context(builder, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("snippet request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response.json().await.map_err(|_| Error::Decode("invalid snippet response".into()))
        }
    }
}
