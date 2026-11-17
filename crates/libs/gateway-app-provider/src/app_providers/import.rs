//! Native import returns counts only; errors discard upstream payloads.
use gateway_app_read::AgentAppsReadError as Error;
use super::GatewayAppProviderClient;
use vw_api_app_provider_edit::import::*;
impl GatewayAppProviderClient {
    pub async fn provider_import(
        &self,
        app: vw_api_app_identity::AppId,
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
            let app = serde_json::to_value(app)
                .map_err(|_| Error::Transport("invalid application".into()))?;
            let app = app.as_str().ok_or_else(|| Error::Transport("invalid application".into()))?;
            let url = format!("{}/v1/app-providers/{app}/catalog/import", self.transport.endpoint().base_url());
            let request = self.transport.dynamic_host_client()
                .post(url)

                .json(body);
            let response = gateway_http::apply_request_context(request, self.transport.endpoint())
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
