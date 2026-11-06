use super::{GatewayClient, provider_path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, update::AppProviderUpdate},
};
impl GatewayClient {
    /// Partial catalog update without native activation.
    pub async fn app_provider_update(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderUpdate,
    ) -> Result<AppProviderSummary, String> {
        let path = provider_path(app, id)?;
        let builder =
            self.client.patch(format!("{}{}", self.endpoint.base_url(), path)).json(request);
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|_| "application provider update transport failed")?;
        let status = response.status();
        if !status.is_success() {
            // Never include a server's raw error body or the write-only request in diagnostics.
            return Err(format!("application provider update failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid application provider update response".into())
    }
}
