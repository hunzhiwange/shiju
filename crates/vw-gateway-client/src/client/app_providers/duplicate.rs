use super::{GatewayClient, provider_path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, duplicate::AppProviderDuplicate},
};
impl GatewayClient {
    /// Copy a catalog record within its current tenant/application, without activation.
    pub async fn app_provider_duplicate(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderDuplicate,
    ) -> Result<AppProviderSummary, String> {
        let path = format!("{}/duplicate", provider_path(app, id)?);
        let builder =
            self.client.post(format!("{}{}", self.endpoint.base_url(), path)).json(request);
        let response = crate::http::apply_request_context(builder, &self.endpoint)
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
