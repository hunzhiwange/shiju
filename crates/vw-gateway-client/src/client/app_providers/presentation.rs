use super::{GatewayClient, provider_collection_path, provider_path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::presentation::{AppProviderPresentation, AppProviderPresentationEntry},
};
impl GatewayClient {
    pub async fn app_provider_presentation_set(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderPresentation,
    ) -> Result<AppProviderPresentation, String> {
        let builder = self
            .client
            .put(format!("{}{}/presentation", self.endpoint.base_url(), provider_path(app, id)?))
            .json(request);
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|_| "application provider presentation transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("application provider presentation failed: {}", status.as_u16()));
        }
        response
            .json()
            .await
            .map_err(|_| "invalid application provider presentation response".into())
    }
    pub async fn app_provider_presentation_list(
        &self,
        app: AppId,
    ) -> Result<Vec<AppProviderPresentationEntry>, String> {
        let builder = self.client.get(format!(
            "{}{}/presentation/entries",
            self.endpoint.base_url(),
            provider_collection_path(app)
        ));
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|_| "application provider presentation transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("application provider presentation failed: {}", status.as_u16()));
        }
        response
            .json()
            .await
            .map_err(|_| "invalid application provider presentation response".into())
    }
}
