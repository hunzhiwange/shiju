//! Native Claude config operations with explicit host authorization and redacted transport errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::claude::*;
impl GatewayClient {
    pub async fn claude_form_read(
        &self,
        id: &str,
        target: &ClaudeTarget,
        token: &str,
    ) -> Result<ClaudeFormView, Error> {
        self.claude_request(id, "POST", "/form/read", target, token).await
    }

    pub async fn claude_form_create(
        &self,
        id: &str,
        body: &ClaudeFormWrite,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        self.claude_request(id, "POST", "/form", body, token).await
    }

    pub async fn claude_form_update(
        &self,
        id: &str,
        body: &ClaudeFormWrite,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        self.claude_request(id, "PATCH", "/form", body, token).await
    }

    pub async fn claude_provider_create(
        &self,
        id: &str,
        body: &ClaudeWrite,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        self.claude_request(id, "POST", "", body, token).await
    }
    pub async fn claude_provider_save(
        &self,
        id: &str,
        body: &ClaudeWrite,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        self.claude_request(id, "PUT", "", body, token).await
    }
    pub async fn claude_provider_update(
        &self,
        id: &str,
        body: &ClaudeWrite,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        self.claude_request(id, "PATCH", "", body, token).await
    }
    pub async fn claude_provider_switch(
        &self,
        id: &str,
        body: &ClaudeTarget,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        self.claude_request(id, "POST", "/switch", body, token).await
    }
    pub async fn claude_provider_reapply(
        &self,
        id: &str,
        body: &ClaudeTarget,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        self.claude_request(id, "POST", "/reapply", body, token).await
    }
    pub async fn claude_provider_import(
        &self,
        id: &str,
        body: &ClaudeImport,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        self.claude_request(id, "POST", "/import", body, token).await
    }
    pub async fn claude_provider_sync(
        &self,
        id: &str,
        body: &ClaudeSync,
        token: &str,
    ) -> Result<ClaudeResult, Error> {
        self.claude_request(id, "POST", "/sync/claude", body, token).await
    }
    async fn claude_request<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        id: &str,
        method: &str,
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
            let _ = (method, action, body, token);
            Err(Error::Transport("Claude requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let prefix = if action == "/sync/claude" {
                "/v1/universal-providers"
            } else {
                "/v1/app-providers/claude"
            };
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
                .map_err(|_| Error::Transport("Claude request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response.json().await.map_err(|_| Error::Decode("invalid Claude response".into()))
        }
    }
}
