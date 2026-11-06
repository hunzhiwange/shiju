use super::{GatewayClient, provider_path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, create::AppProviderCreate},
};
impl GatewayClient {
    /// Explicit create-only operation. Never defaults to live activation or upsert.
    pub async fn app_provider_create(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderCreate,
    ) -> Result<AppProviderSummary, String> {
        let path = provider_path(app, id)?;
        let builder =
            self.client.post(format!("{}{}", self.endpoint.base_url(), path)).json(request);
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|_| "application provider create transport failed")?;
        let status = response.status();
        if !status.is_success() {
            // Never include a server's raw error body or the write-only request in diagnostics.
            return Err(format!("application provider create failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid application provider create response".into())
    }
}
