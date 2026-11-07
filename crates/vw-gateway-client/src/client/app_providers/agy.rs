//! Agy domain SDK; launch selection uses the existing agent-apps terminal endpoint.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::agy::*;
impl GatewayClient {
    pub async fn agy_form_delete(
        &self,
        id: &str,
        body: &AgyTarget,
        token: &str,
    ) -> Result<vw_api_types::app_providers::delete::AppProviderDeleted, Error> {
        self.agy_request(id, "POST", "/form/delete", body, token).await
    }

    pub async fn agy_form_read(
        &self,
        id: &str,
        body: &AgyTarget,
        token: &str,
    ) -> Result<AgyFormView, Error> {
        self.agy_request(id, "POST", "/form/read", body, token).await
    }
    pub async fn agy_form_create(
        &self,
        id: &str,
        body: &AgyFormWrite,
        token: &str,
    ) -> Result<AgySaved, Error> {
        self.agy_request(id, "POST", "/form", body, token).await
    }
    pub async fn agy_form_update(
        &self,
        id: &str,
        body: &AgyFormWrite,
        token: &str,
    ) -> Result<AgySaved, Error> {
        self.agy_request(id, "PATCH", "/form", body, token).await
    }

    pub async fn agy_provider_save(
        &self,
        id: &str,
        body: &AgyWrite,
        token: &str,
    ) -> Result<AgySaved, Error> {
        self.agy_request(id, "PUT", "/config", body, token).await
    }
    pub async fn agy_provider_switch(
        &self,
        id: &str,
        body: &AgyTarget,
        token: &str,
    ) -> Result<AgySaved, Error> {
        self.agy_request(id, "POST", "/switch", body, token).await
    }
    async fn agy_request<B: serde::Serialize, R: serde::de::DeserializeOwned>(
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
            Err(Error::Transport("Agy requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let prefix = "/v1/app-providers/agy";
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
                .map_err(|_| Error::Transport("Agy request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response.json().await.map_err(|_| Error::Decode("invalid Agy response".into()))
        }
    }
}
