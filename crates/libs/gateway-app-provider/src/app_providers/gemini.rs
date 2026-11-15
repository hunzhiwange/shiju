//! Fixed Gemini operations with sensitive headers and payload-free errors.
use crate::AgentAppsReadError as Error;
use super::GatewayAppProviderClient;
use vw_api_app_provider_profiles::gemini::*;
impl GatewayAppProviderClient {
    pub async fn gemini_provider_save(
        &self,
        id: &str,
        body: &GeminiWrite,
        token: &str,
    ) -> Result<GeminiResult, Error> {
        self.gemini_request(id, "PUT", "/config", body, token).await
    }
    pub async fn gemini_provider_switch(
        &self,
        id: &str,
        body: &GeminiTarget,
        token: &str,
    ) -> Result<GeminiResult, Error> {
        self.gemini_request(id, "POST", "/switch", body, token).await
    }
    pub async fn gemini_provider_reapply(
        &self,
        id: &str,
        body: &GeminiTarget,
        token: &str,
    ) -> Result<GeminiResult, Error> {
        self.gemini_request(id, "POST", "/reapply", body, token).await
    }
    pub async fn gemini_form_read(
        &self,
        id: &str,
        body: &GeminiTarget,
        token: &str,
    ) -> Result<GeminiFormView, Error> {
        self.gemini_request(id, "POST", "/form/read", body, token).await
    }
    pub async fn gemini_form_create(
        &self,
        id: &str,
        body: &GeminiFormWrite,
        token: &str,
    ) -> Result<GeminiResult, Error> {
        self.gemini_request(id, "POST", "/form", body, token).await
    }
    pub async fn gemini_form_update(
        &self,
        id: &str,
        body: &GeminiFormWrite,
        token: &str,
    ) -> Result<GeminiResult, Error> {
        self.gemini_request(id, "PATCH", "/form", body, token).await
    }
    async fn gemini_request<B: serde::Serialize, R: serde::de::DeserializeOwned>(
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
            Err(Error::Transport("Gemini requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let prefix = "/v1/app-providers/gemini";
            let method = reqwest::Method::from_bytes(method.as_bytes())
                .map_err(|_| Error::Transport("invalid method".into()))?;
            let request = self.transport.dynamic_host_client()
                .request(method, format!("{}{prefix}/{id}{action}", self.transport.endpoint().base_url()))
                .header("x-agent-apps-token", credential)
                .json(body);
            let response = gateway_http::apply_request_context(request, self.transport.endpoint())
                .send()
                .await
                .map_err(|_| Error::Transport("Gemini request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response.json().await.map_err(|_| Error::Decode("invalid Gemini response".into()))
        }
    }
}
