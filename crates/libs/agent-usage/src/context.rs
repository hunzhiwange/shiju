//! Local execution identity; never derived from model options or HTTP payloads.
use agent_database::GlobalDatabase;

pub fn database(home: &std::path::Path) -> anyhow::Result<GlobalDatabase> {
    GlobalDatabase::for_home(home)
}

pub fn local_host() -> anyhow::Result<String> {
    Ok(local_app_auth::HOST_ID.to_owned())
}
