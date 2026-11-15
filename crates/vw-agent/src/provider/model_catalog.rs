//! Host storage adapter for the provider catalog.
use crate::config::{Config, Error};
use std::collections::BTreeMap;
pub use agent_provider::model_catalog::*;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn read(config: &Config) -> Result<DeploymentCatalogs, Error> {
    use crate::{
        app_providers::AppProviderStore,
        global_database::{self, GlobalDatabase},
        session::ui_store::current_tenant_scope,
    };
    use serde_json::Value;
    use vw_api_model_catalog::ModelCatalogIdentity;

    let mut bindings = Vec::new();
    for (provider_id, provider) in &config.providers {
        let Some(binding) = provider.get("model_catalog") else { continue };
        vw_agent_apps::providers::validate_identity(provider_id).map_err(Error::Invalid)?;
        let deployment: ModelCatalogIdentity = serde_json::from_value(binding.clone())
            .map_err(|_| Error::Invalid("invalid runtime model catalog binding".into()))?;
        if matches!(provider_id.as_str(), "shiju" | "acp")
            || provider.get("api").and_then(Value::as_str) != Some(deployment.endpoint.as_str())
        {
            return Err(Error::Invalid(
                "runtime model catalog requires the same configured endpoint on a custom provider"
                    .into(),
            ));
        }
        let adapter = provider.get("adapter").and_then(Value::as_str).unwrap_or_default();
        if !matches!(adapter, "openai" | "openai-compatible" | "openai-responses" | "anthropic") {
            return Err(Error::Invalid("unsupported runtime model catalog adapter".into()));
        }
        bindings.push((provider_id.clone(), adapter.to_string(), deployment));
    }
    if bindings.is_empty() {
        return Ok(BTreeMap::new());
    }

    let tenant = current_tenant_scope();
    global_database::run_isolated(move || {
        let database = GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| Error::Invalid("application provider storage unavailable".into()))?;
        bindings
            .into_iter()
            .map(|(id, adapter, deployment)| {
                // Read tenant-owned metadata only. Runtime routing and authentication
                // remain on the configured runtime provider.
                let record = AppProviderStore::preview_record(
                    &database,
                    &tenant,
                    deployment.app_id,
                    &deployment.provider_id,
                )
                .map_err(Error::Invalid)?;
                let catalog = vw_agent_apps::models::edits::read(&tenant, &deployment, &record)
                    .map_err(Error::Invalid)?;
                Ok((id, DeploymentCatalog { adapter, catalog }))
            })
            .collect()
    })
    .await
    .map_err(|_| Error::Invalid("application provider storage unavailable".into()))?
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn read(_: &Config) -> Result<DeploymentCatalogs, Error> {
    Ok(BTreeMap::new())
}
