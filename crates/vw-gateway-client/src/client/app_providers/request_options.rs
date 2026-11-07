use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::{agent_apps::AppId, app_providers::request_options::*};
impl GatewayClient {
    pub async fn app_provider_request_options_read(
        &self,
        app: AppId,
        id: &str,
        body: &RequestOptionsTarget,
        token: &str,
    ) -> Result<RequestOptionsView, Error> {
        self.request_options_request(app, id, "POST", "/request-options/read", body, token).await
    }
    pub async fn app_provider_request_options_save(
        &self,
        app: AppId,
        id: &str,
        body: &RequestOptionsWrite,
        token: &str,
    ) -> Result<RequestOptionsView, Error> {
        self.request_options_request(app, id, "PUT", "/request-options", body, token).await
    }
    pub(super) async fn request_options_request<
        B: serde::Serialize,
        R: serde::de::DeserializeOwned,
    >(
        &self,
        app: vw_api_types::agent_apps::AppId,
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
            let _ = (app, method, action, body, token);
            Err(Error::Transport("request options requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let path = super::provider_path(app, id).map_err(Error::Transport)?;
            let method = reqwest::Method::from_bytes(method.as_bytes())
                .map_err(|_| Error::Transport("invalid method".into()))?;
            let request = self
                .dynamic_host_client
                .request(method, format!("{}{path}{action}", self.endpoint.base_url()))
                .header("x-agent-apps-token", credential)
                .json(body);
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("request options request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid request options response".into()))
        }
    }
}
