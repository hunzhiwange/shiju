use gateway_app_read::AgentAppsReadError as Error;
use super::{GatewayAppProviderClient, provider_collection_path};
use vw_api_app_provider_edit::snippets::*;
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
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
            let url = format!(
                "{}{}/config/snippet",
                self.transport.endpoint().base_url(),
                provider_collection_path(app)
            );
            let builder = match action {
                "read" => self.transport.dynamic_host_client().get(url).query(body),
                "save" => self.transport.dynamic_host_client().put(url).json(body),
                "preview" => self.transport.dynamic_host_client().post(format!("{url}/preview")).json(body),
                "extract" => self.transport.dynamic_host_client().post(format!("{url}/extract")).json(body),
                _ => unreachable!(),
            };
            let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
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
