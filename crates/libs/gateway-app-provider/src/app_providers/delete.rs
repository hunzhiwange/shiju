use super::{GatewayAppProviderClient, provider_path};
use vw_api_app_provider_edit::delete::AppProviderDelete;
use vw_api_app_provider_edit::delete::AppProviderDeleted;
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
    /// Deletes the catalog row only; requires independent host configuration read authorization.
    pub async fn app_provider_delete(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderDelete,
        token: &str,
    ) -> Result<AppProviderDeleted, String> {
        let path = provider_path(app, id)?;
        let mut credential = reqwest::header::HeaderValue::from_str(token)
            .map_err(|_| "invalid application credential")?;
        credential.set_sensitive(true);
        let builder = self.transport.client()
            .delete(format!("{}{}", self.transport.endpoint().base_url(), path))
            .header("x-agent-apps-token", credential)
            .json(request);
        let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
            .send()
            .await
            .map_err(|_| "application provider delete transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("application provider delete failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid application provider delete response".into())
    }
}
