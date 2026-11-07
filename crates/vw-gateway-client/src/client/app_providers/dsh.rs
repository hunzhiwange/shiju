//! Fixed Dsh operations with sensitive headers and payload-free errors.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::dsh::*;
impl GatewayClient {
    pub async fn dsh_form_read(
        &self,
        id: &str,
        body: &DshTarget,
        token: &str,
    ) -> Result<DshFormView, Error> {
        self.dsh_request(id, "POST", "/form/read", body, token).await
    }
    pub async fn dsh_form_create(
        &self,
        id: &str,
        body: &DshFormWrite,
        token: &str,
    ) -> Result<DshResult, Error> {
        self.dsh_request(id, "POST", "/form", body, token).await
    }
    pub async fn dsh_form_update(
        &self,
        id: &str,
        body: &DshFormWrite,
        token: &str,
    ) -> Result<DshResult, Error> {
        self.dsh_request(id, "PATCH", "/form", body, token).await
    }

    pub async fn dsh_provider_remove_live(
        &self,
        id: &str,
        body: &DshTarget,
        token: &str,
    ) -> Result<DshResult, Error> {
        self.dsh_request(id, "POST", "/remove-live", body, token).await
    }

    pub async fn dsh_provider_save(
        &self,
        id: &str,
        body: &DshWrite,
        token: &str,
    ) -> Result<DshResult, Error> {
        self.dsh_request(id, "PUT", "/config", body, token).await
    }
    pub async fn dsh_provider_add(
        &self,
        id: &str,
        body: &DshTarget,
        token: &str,
    ) -> Result<DshResult, Error> {
        self.dsh_request(id, "POST", "/add", body, token).await
    }
    pub async fn dsh_state(&self, body: &DshTarget, token: &str) -> Result<DshState, Error> {
        self.dsh_request("config", "POST", "/state", body, token).await
    }
    async fn dsh_request<B: serde::Serialize, R: serde::de::DeserializeOwned>(
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
            Err(Error::Transport("Dsh requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let prefix = "/v1/app-providers/dsh";
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
                .map_err(|_| Error::Transport("Dsh request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                let body = if status.as_u16() == 422 {
                    response
                        .json::<DshError>()
                        .await
                        .ok()
                        .and_then(|e| serde_json::to_value(e).ok())
                        .unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                };
                return Err(Error::Http { status: status.as_u16(), body });
            }
            response.json().await.map_err(|_| Error::Decode("invalid Dsh response".into()))
        }
    }
}
