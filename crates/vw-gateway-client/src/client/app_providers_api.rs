//! Tenant application-provider records; no relationship to model-provider auth APIs.
use super::GatewayClient;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, AppProviderWrite},
};
impl GatewayClient {
    pub async fn app_provider_save(
        &self,
        app: AppId,
        id: &str,
        profile: &AppProviderWrite,
    ) -> Result<AppProviderSummary, String> {
        self.put_json(&provider_path(app, id)?, &[], profile).await
    }
    pub async fn app_provider_get(
        &self,
        app: AppId,
        id: &str,
    ) -> Result<AppProviderSummary, String> {
        self.get_json(&provider_path(app, id)?, &[]).await
    }
}
fn provider_path(app: AppId, id: &str) -> Result<String, String> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err("invalid application provider identity".into());
    }
    Ok(format!("{}/{id}", provider_collection_path(app)))
}

#[path = "app_providers/catalog.rs"]
mod catalog;
#[cfg(test)]
#[path = "app_providers/catalog_tests.rs"]
mod catalog_tests;

fn provider_collection_path(app: AppId) -> String {
    let app = serde_json::to_string(&app).expect("AppId serialization");
    format!("/v1/app-providers/{}", app.trim_matches('"'))
}

#[path = "app_providers/create.rs"]
mod create;
#[cfg(test)]
#[path = "app_providers/create_tests.rs"]
mod create_tests;

#[path = "app_providers/update.rs"]
mod update;
#[cfg(test)]
#[path = "app_providers/update_tests.rs"]
mod update_tests;

#[path = "app_providers/delete.rs"]
mod delete;
#[cfg(test)]
#[path = "app_providers/delete_tests.rs"]
mod delete_tests;

#[path = "app_providers/duplicate.rs"]
mod duplicate;
#[cfg(test)]
#[path = "app_providers/duplicate_tests.rs"]
mod duplicate_tests;

#[path = "app_providers/presentation.rs"]
mod presentation;
#[cfg(test)]
#[path = "app_providers/presentation_tests.rs"]
mod presentation_tests;

#[path = "app_providers/endpoints.rs"]
mod endpoints;
#[cfg(test)]
#[path = "app_providers/endpoints_tests.rs"]
mod endpoints_tests;

#[path = "app_providers/icons.rs"]
mod icons;
#[cfg(test)]
#[path = "app_providers/icons_tests.rs"]
mod icons_tests;
