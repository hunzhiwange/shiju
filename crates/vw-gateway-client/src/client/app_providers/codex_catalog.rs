//! Fixed Codex operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::codex_catalog::*;
impl GatewayClient {
    pub async fn codex_provider_save(
        &self,
        id: &str,
        body: &CodexWrite,
        token: &str,
    ) -> Result<CodexResult, Error> {
        self.codex_request(id, "PUT", "/config", body, token).await
    }
    pub async fn codex_provider_switch(
        &self,
        id: &str,
        body: &CodexTarget,
        token: &str,
    ) -> Result<CodexResult, Error> {
        self.codex_request(id, "POST", "/switch", body, token).await
    }
    pub async fn codex_provider_reapply(
        &self,
        id: &str,
        body: &CodexTarget,
        token: &str,
    ) -> Result<CodexResult, Error> {
        self.codex_request(id, "POST", "/reapply", body, token).await
    }
    async fn codex_request<B: serde::Serialize>(
        &self,
        id: &str,
        method: &str,
        action: &str,
        body: &B,
        token: &str,
    ) -> Result<CodexResult, Error> {
        if id.is_empty()
            || id.len() > 128
            || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err(Error::Transport("invalid provider identity".into()));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, action, body, token);
            Err(Error::Transport("Codex requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let prefix = "/v1/app-providers/codex";
            let method = reqwest::Method::from_bytes(method.as_bytes())
                .map_err(|_| Error::Transport("invalid method".into()))?;
            let request = self
                .dynamic_host_client
                .request(method, format!("{}{prefix}/{id}{action}", self.endpoint.base_url()))
                .header("x-agent-apps-token", credential)
                .json(body);
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("Codex request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response.json().await.map_err(|_| Error::Decode("invalid Codex response".into()))
        }
    }
}
