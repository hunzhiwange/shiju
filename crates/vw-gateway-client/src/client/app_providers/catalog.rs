use super::{GatewayClient, provider_collection_path};
use vw_api_types::{agent_apps::AppId, app_providers::AppProviderSummary};
impl GatewayClient {
    pub async fn app_provider_list(&self, app: AppId) -> Result<Vec<AppProviderSummary>, String> {
        self.get_json(&provider_collection_path(app), &[]).await
    }
}

impl GatewayClient {
    /// Native transport only; token authorizes the selected execution host independently.
    pub async fn app_provider_catalog(
        &self,
        app: AppId,
        host: &str,
        token: &str,
    ) -> Result<
        vw_api_types::app_providers::catalog::ProviderCatalog,
        super::super::agent_apps_read_api::AgentAppsReadError,
    > {
        use super::super::agent_apps_read_api::AgentAppsReadError as Error;
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (app, host, token);
            Err(Error::Transport("authorized application reads require native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let base = format!("{}/catalog/state", provider_collection_path(app));
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let request = self
                .dynamic_host_client
                .get(format!("{}{}", self.endpoint.base_url(), base))
                .query(&[("targetHostId", host)])
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
                .map_err(|_| Error::Decode("invalid application provider catalog".into()))
        }
    }
}
