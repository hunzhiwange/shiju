//! Fixed Mcode operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::mcode::*;
impl GatewayClient {
    pub async fn mcode_provider_editor_save(
        &self,
        id: &str,
        body: &McodeEditorWrite,
        token: &str,
    ) -> Result<McodeResult, Error> {
        self.mcode_request(id, "PUT", "/editor", body, token).await
    }

    pub async fn mcode_provider_save(
        &self,
        id: &str,
        body: &McodeWrite,
        token: &str,
    ) -> Result<McodeResult, Error> {
        self.mcode_request(id, "PUT", "/config", body, token).await
    }
    pub async fn mcode_provider_add(
        &self,
        id: &str,
        body: &McodeTarget,
        token: &str,
    ) -> Result<McodeResult, Error> {
        self.mcode_request(id, "POST", "/add", body, token).await
    }
    pub async fn mcode_provider_remove_live(
        &self,
        id: &str,
        body: &McodeTarget,
        token: &str,
    ) -> Result<McodeResult, Error> {
        self.mcode_request(id, "POST", "/remove-live", body, token).await
    }
    pub async fn mcode_import(
        &self,
        body: &McodeTarget,
        token: &str,
    ) -> Result<McodeImportResult, Error> {
        self.mcode_request("config", "POST", "/import", body, token).await
    }
    pub async fn mcode_state(&self, body: &McodeTarget, token: &str) -> Result<McodeState, Error> {
        self.mcode_request("config", "POST", "/state", body, token).await
    }
    async fn mcode_request<B: serde::Serialize, R: serde::de::DeserializeOwned>(
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
            Err(Error::Transport("Mcode requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let prefix = "/v1/app-providers/mcode";
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
                .map_err(|_| Error::Transport("Mcode request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response.json().await.map_err(|_| Error::Decode("invalid Mcode response".into()))
        }
    }
}
