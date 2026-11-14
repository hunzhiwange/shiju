//! Local execution identity; never derived from model options or HTTP payloads.
use crate::global_database::GlobalDatabase;

pub(crate) fn database() -> anyhow::Result<GlobalDatabase> {
    GlobalDatabase::for_home(&crate::global::paths().home)
}

pub(crate) fn local_host() -> anyhow::Result<String> {
    Ok(vw_shared::local_apps::HOST_ID.to_owned())
}
