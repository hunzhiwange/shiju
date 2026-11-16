//! Domain tests migrated from the agent host; storage remains real SQLite.
mod provider;
mod catalog;

tokio::task_local! { static TENANT: vw_api_transport::id::TenantScope; }
fn tenant() -> vw_api_transport::id::TenantScope {
    TENANT.try_with(Clone::clone).unwrap_or_default()
}
async fn with_tenant_scope<T>(tenant: vw_api_transport::id::TenantScope, future: impl std::future::Future<Output = T>) -> T {
    TENANT.scope(tenant, future).await
}
fn global_async_env_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    &LOCK
}
fn ensure_host() {
    crate::provider::install_host(crate::provider::Host {
        current_tenant_scope: tenant,
        read_catalogs: |config| Box::pin(async move {
            crate::model_catalog::read_with(&config, |deployment| {
                let database = agent_database::GlobalDatabase::for_home(&agent_paths::paths().home)
                    .map_err(|_| agent_config::runtime::Error::Invalid("application provider storage unavailable".into()))?;
                let tenant = tenant();
                let record = app_provider_services::AppProviderStore::preview_record(
                    &database, &tenant, deployment.app_id, &deployment.provider_id,
                ).map_err(agent_config::runtime::Error::Invalid)?;
                app_models::edits::read(&tenant, deployment, &record)
                    .map_err(agent_config::runtime::Error::Invalid)
            })
        }),
    });
}
