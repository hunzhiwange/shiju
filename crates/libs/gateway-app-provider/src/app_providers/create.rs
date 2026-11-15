use super::{GatewayAppProviderClient, provider_path};
use vw_api_app_provider_base::AppProviderSummary;
use vw_api_app_provider_base::create::AppProviderCreate;
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
    /// Explicit create-only operation. Never defaults to live activation or upsert.
    pub async fn app_provider_create(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderCreate,
    ) -> Result<AppProviderSummary, String> {
        let path = provider_path(app, id)?;
        let builder =
            self.transport.client().post(format!("{}{}", self.transport.endpoint().base_url(), path)).json(request);
        let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
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
