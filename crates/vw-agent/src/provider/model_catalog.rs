//! Host lifecycle and tenant-owned storage port for the provider catalog.
use crate::config::{Config, Error};
pub use agent_provider::model_catalog::*;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn read(config: &Config) -> Result<DeploymentCatalogs, Error> {
    let config = config.clone();
    let tenant = crate::session::ui_store::current_tenant_scope();
    crate::global_database::run_isolated(move || {
        // Open storage lazily: unbound configurations must not require a database.
        let mut database = None;
        read_with(&config, |deployment| {
            if database.is_none() {
                database = Some(crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| Error::Invalid("application provider storage unavailable".into()))?);
            }
            let record = crate::app_providers::AppProviderStore::preview_record(
                database.as_ref().expect("opened catalog storage"), &tenant, deployment.app_id, &deployment.provider_id,
            ).map_err(Error::Invalid)?;
            vw_agent_apps::models::edits::read(&tenant, deployment, &record).map_err(Error::Invalid)
        })
    }).await.map_err(|_| Error::Invalid("application provider storage unavailable".into()))?
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn read(_: &Config) -> Result<DeploymentCatalogs, Error> {
    Ok(std::collections::BTreeMap::new())
}
