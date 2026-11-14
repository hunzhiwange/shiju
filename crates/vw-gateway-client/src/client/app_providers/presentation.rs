use super::{GatewayClient, provider_collection_path, provider_path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::presentation::{AppProviderPresentation, AppProviderPresentationEntry},
};
impl GatewayClient {
    /// Read persisted presentation and apply its category/index/identity ordering.
    /// Callers render these rows directly, including after a refresh or save.
    pub async fn app_provider_presented_catalog(
        &self,
        app: AppId,
        host: &str,
        token: &str,
    ) -> Result<(vw_api_types::app_providers::catalog::ProviderCatalog, Vec<AppProviderPresentationEntry>), String> {
        let mut catalog = self.app_provider_catalog(app, host, token).await?;
        let presentation = self.app_provider_presentation_list(app).await?;
        catalog.rows.sort_by_key(|row| {
            let value = presentation.iter().find(|entry| entry.id == row.provider.id);
            (
                value.and_then(|entry| entry.presentation.category.clone()).unwrap_or_default(),
                value.and_then(|entry| entry.presentation.sort_index).unwrap_or(u32::MAX),
                row.provider.id.clone(),
            )
        });
        Ok((catalog, presentation))
    }

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
