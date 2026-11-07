//! Fixed OpenCode operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::opencode::*;
impl GatewayClient {
    pub async fn opencode_provider_save(
        &self,
        id: &str,
        body: &OpenCodeWrite,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        self.opencode_request(id, "PUT", "/config", body, token).await
    }
    pub async fn opencode_form_read(
        &self,
        id: &str,
        body: &OpenCodeTarget,
        token: &str,
    ) -> Result<OpenCodeFormView, Error> {
        self.opencode_request(id, "POST", "/form/read", body, token).await
    }
    pub async fn opencode_form_create(
        &self,
        id: &str,
        body: &OpenCodeFormWrite,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        self.opencode_request(id, "POST", "/form", body, token).await
    }
    pub async fn opencode_form_update(
        &self,
        id: &str,
        body: &OpenCodeFormWrite,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        self.opencode_request(id, "PATCH", "/form", body, token).await
    }
    pub async fn opencode_provider_add(
        &self,
        id: &str,
        body: &OpenCodeTarget,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        self.opencode_request(id, "POST", "/add", body, token).await
    }
    pub async fn opencode_provider_remove_live(
        &self,
        id: &str,
        body: &OpenCodeTarget,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        self.opencode_request(id, "POST", "/remove-live", body, token).await
    }
    pub async fn opencode_default_model(
        &self,
        body: &OpenCodeDefaultModel,
        token: &str,
    ) -> Result<OpenCodeResult, Error> {
        self.opencode_request("config", "PUT", "/default-model", body, token).await
    }
    pub async fn opencode_import(
        &self,
        body: &OpenCodeTarget,
        token: &str,
    ) -> Result<OpenCodeImportResult, Error> {
        self.opencode_request("config", "POST", "/import", body, token).await
    }
    pub async fn opencode_state(
        &self,
        body: &OpenCodeTarget,
        token: &str,
    ) -> Result<OpenCodeState, Error> {
        self.opencode_request("config", "POST", "/state", body, token).await
    }
    async fn opencode_request<B: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        id: &str,
        method: &str,
        action: &str,
        body: &B,
        token: &str,
    ) -> Result<R, Error> {
        if id.is_empty()
            || id.len() > 128
            || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err(Error::Transport("invalid provider identity".into()));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, action, body, token);
            Err(Error::Transport("OpenCode requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let prefix = "/v1/app-providers/opencode";
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
                .map_err(|_| Error::Transport("OpenCode request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response.json().await.map_err(|_| Error::Decode("invalid OpenCode response".into()))
        }
    }
}
