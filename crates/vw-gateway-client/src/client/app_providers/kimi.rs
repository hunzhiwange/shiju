//! Fixed Kimi operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::kimi::*;
impl GatewayClient {
    pub async fn kimi_form_read(
        &self,
        id: &str,
        body: &KimiTarget,
        token: &str,
    ) -> Result<KimiFormView, Error> {
        self.kimi_request(id, "POST", "/form/read", body, token).await
    }
    pub async fn kimi_form_create(
        &self,
        id: &str,
        body: &KimiFormWrite,
        token: &str,
    ) -> Result<KimiResult, Error> {
        self.kimi_request(id, "POST", "/form", body, token).await
    }
    pub async fn kimi_form_update(
        &self,
        id: &str,
        body: &KimiFormWrite,
        token: &str,
    ) -> Result<KimiResult, Error> {
        self.kimi_request(id, "PATCH", "/form", body, token).await
    }

    pub async fn kimi_provider_editor_save(
        &self,
        id: &str,
        body: &KimiEditorWrite,
        token: &str,
    ) -> Result<KimiResult, Error> {
        self.kimi_request(id, "PUT", "/editor", body, token).await
    }

    pub async fn kimi_provider_save(
        &self,
        id: &str,
        body: &KimiWrite,
        token: &str,
    ) -> Result<KimiResult, Error> {
        self.kimi_request(id, "PUT", "/config", body, token).await
    }
    pub async fn kimi_provider_add(
        &self,
        id: &str,
        body: &KimiTarget,
        token: &str,
    ) -> Result<KimiResult, Error> {
        self.kimi_request(id, "POST", "/add", body, token).await
    }
    pub async fn kimi_provider_remove_live(
        &self,
        id: &str,
        body: &KimiTarget,
        token: &str,
    ) -> Result<KimiResult, Error> {
        self.kimi_request(id, "POST", "/remove-live", body, token).await
    }
    pub async fn kimi_import(
        &self,
        body: &KimiTarget,
        token: &str,
    ) -> Result<KimiImportResult, Error> {
        self.kimi_request("config", "POST", "/import", body, token).await
    }
    pub async fn kimi_state(&self, body: &KimiTarget, token: &str) -> Result<KimiState, Error> {
        self.kimi_request("config", "POST", "/state", body, token).await
    }
    async fn kimi_request<B: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        id: &str,
        method: &str,
        action: &str,
        body: &B,
        token: &str,
    ) -> Result<R, Error> {
        if id.is_empty()
            || id.len() > 128
            || id.trim().is_empty()
            || id.chars().any(char::is_control)
            || matches!(id, "." | "..")
            || id.contains(['/', '\\'])
        {
            return Err(Error::Transport("invalid provider identity".into()));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, action, body, token);
            Err(Error::Transport("Kimi requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let prefix = "/v1/app-providers/kimi";
            let method = reqwest::Method::from_bytes(method.as_bytes())
                .map_err(|_| Error::Transport("invalid method".into()))?;
            let mut url = reqwest::Url::parse(&format!("{}{prefix}/", self.endpoint.base_url()))
                .map_err(|_| Error::Transport("invalid gateway endpoint".into()))?;
            url.path_segments_mut()
                .map_err(|_| Error::Transport("invalid gateway endpoint".into()))?
                .pop_if_empty()
                .push(id)
                .extend(action.trim_start_matches('/').split('/'));
            let request = self
                .dynamic_host_client
                .request(method, url)
                .header("x-agent-apps-token", credential)
                .json(body);
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("Kimi request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response.json().await.map_err(|_| Error::Decode("invalid Kimi response".into()))
        }
    }
}
