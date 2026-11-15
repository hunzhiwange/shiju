use super::{GatewayAppProviderClient, provider_path};
use vw_api_app_provider_base::AppProviderSummary;
use vw_api_app_provider_base::update::AppProviderUpdate;
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
    /// Partial catalog update without native activation.
    pub async fn app_provider_update(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderUpdate,
    ) -> Result<AppProviderSummary, String> {
        let path = provider_path(app, id)?;
        let builder =
            self.transport.client().patch(format!("{}{}", self.transport.endpoint().base_url(), path)).json(request);
        let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
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
