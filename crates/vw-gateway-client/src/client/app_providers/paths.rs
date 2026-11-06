use super::{GatewayClient, provider_collection_path};
use vw_api_types::agent_apps::AppId;
impl GatewayClient {
    /// Native transport only; token authorizes the selected execution host independently.
    pub async fn app_provider_paths(
        &self,
        app: AppId,
        request_body: &vw_api_types::app_providers::paths::ProviderPathsRequest,
        token: &str,
    ) -> Result<
        vw_api_types::app_providers::paths::ProviderPaths,
        super::super::agent_apps_read_api::AgentAppsReadError,
    > {
        use super::super::agent_apps_read_api::AgentAppsReadError as Error;
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (app, request_body, token);
            Err(Error::Transport("authorized application reads require native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let base = format!("{}/config/paths", provider_collection_path(app));
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let request = self
                .dynamic_host_client
                .post(format!("{}{}", self.endpoint.base_url(), base))
                .json(request_body)
                .header("x-agent-apps-token", credential);
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("application provider read failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                // Configuration errors and permission denials must not expose raw payloads.
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid application provider paths".into()))
        }
    }
}
