use super::{GatewayAppProviderClient, provider_path};
use vw_api_app_provider_edit::icons::AppProviderIcon;
use vw_api_app_provider_edit::icons::AppProviderIconChange;
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
    pub async fn app_provider_icon(&self, app: AppId, id: &str) -> Result<AppProviderIcon, String> {
        let builder = self.transport.client().get(format!(
            "{}{}/icon",
            self.transport.endpoint().base_url(),
            provider_path(app, id)?
        ));
        let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
            .send()
            .await
            .map_err(|_| "provider icon transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("provider icon request failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid provider icon response".into())
    }
    pub async fn app_provider_icon_change(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderIconChange,
    ) -> Result<AppProviderIcon, String> {
        let builder = self.transport.client()
            .put(format!("{}{}/icon", self.transport.endpoint().base_url(), provider_path(app, id)?))
            .json(request);
        let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
            .send()
            .await
            .map_err(|_| "provider icon transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("provider icon request failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid provider icon response".into())
    }
}
