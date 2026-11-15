use super::{GatewayAppProviderClient, provider_path};
use vw_api_app_provider_base::AppProviderSummary;
use vw_api_app_provider_edit::duplicate::AppProviderDuplicate;
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
    /// Copy a catalog record within its current tenant/application, without activation.
    pub async fn app_provider_duplicate(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderDuplicate,
    ) -> Result<AppProviderSummary, String> {
        let path = format!("{}/duplicate", provider_path(app, id)?);
        let builder =
            self.transport.client().post(format!("{}{}", self.transport.endpoint().base_url(), path)).json(request);
        let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
            .send()
            .await
            .map_err(|_| "application provider duplicate transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("application provider duplicate failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid application provider duplicate response".into())
    }
}
