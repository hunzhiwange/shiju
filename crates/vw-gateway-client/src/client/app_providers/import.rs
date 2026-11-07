//! Native import returns counts only; errors discard upstream payloads.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::app_providers::import::*;
impl GatewayClient {
    pub async fn provider_import(
        &self,
        app: vw_api_types::agent_apps::AppId,
        body: &ProviderImport,
        token: &str,
    ) -> Result<ProviderImportResult, Error> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (app, body, token);
            Err(Error::Transport("Provider import requires native transport".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut credential = reqwest::header::HeaderValue::from_str(token)
                .map_err(|_| Error::Transport("invalid application credential".into()))?;
            credential.set_sensitive(true);
            let app = serde_json::to_value(app)
                .map_err(|_| Error::Transport("invalid application".into()))?;
            let app = app.as_str().ok_or_else(|| Error::Transport("invalid application".into()))?;
            let url = format!("{}/v1/app-providers/{app}/catalog/import", self.endpoint.base_url());
            let request = self
                .dynamic_host_client
                .post(url)
                .header("x-agent-apps-token", credential)
                .json(body);
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|_| Error::Transport("Provider import request failed".into()))?;
            let status = response.status();
            if !status.is_success() {
                return Err(Error::Http { status: status.as_u16(), body: serde_json::Value::Null });
            }
            response
                .json()
                .await
                .map_err(|_| Error::Decode("invalid Provider import response".into()))
        }
    }
}
