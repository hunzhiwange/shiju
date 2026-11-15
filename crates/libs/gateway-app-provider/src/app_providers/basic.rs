use super::{GatewayAppProviderClient, provider_path};
use vw_api_app_provider_base::basic::ProviderBasic;
use vw_api_app_provider_base::basic::ProviderBasicWrite;
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
    pub async fn app_provider_basic(&self, app: AppId, id: &str) -> Result<ProviderBasic, String> {
        self.transport.get_json(&format!("{}/basic", provider_path(app, id)?), &[]).await
    }
    pub async fn app_provider_basic_write(
        &self,
        app: AppId,
        id: &str,
        value: &ProviderBasicWrite,
    ) -> Result<ProviderBasic, String> {
        let request = self.transport.client()
            .put(format!("{}{}/basic", self.transport.endpoint().base_url(), provider_path(app, id)?))
            .json(value);
        let response = gateway_http::apply_request_context(request, self.transport.endpoint())
            .send()
            .await
            .map_err(|_| "provider basic write transport failed")?;
        if !response.status().is_success() {
            return Err(format!("provider basic write failed: {}", response.status().as_u16()));
        }
        response.json().await.map_err(|_| "invalid provider basic response".into())
    }
}
