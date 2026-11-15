//! Local execution identity; never derived from model options or HTTP payloads.
pub(crate) fn database() -> anyhow::Result<agent_database::GlobalDatabase> {
    agent_usage::context::database(&crate::global::paths().home)
}

pub(crate) use agent_usage::context::local_host;
