//! Local execution identity; never derived from model options or HTTP payloads.
use agent_database::GlobalDatabase;
use vw_api_transport::{ProjectScope, TenantScope};

pub fn database(home: &std::path::Path) -> anyhow::Result<GlobalDatabase> {
    GlobalDatabase::for_home(home)
}

pub fn local_host() -> anyhow::Result<String> {
    Ok(local_app_auth::HOST_ID.to_owned())
}

/// Use the existing local ledger key so upgrades keep reading local history.
pub fn local_scope(tenant: TenantScope) -> ProjectScope {
    ProjectScope::new(tenant, vw_api_transport::local_project_id())
        .expect("the built-in local usage key is valid")
}
