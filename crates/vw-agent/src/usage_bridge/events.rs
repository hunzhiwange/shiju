//! Scope-local usage invalidations.
pub(crate) use agent_usage::events::{CacheChange, cache_channel, cache_updated};

pub(crate) fn revisions(scope: &vw_api_transport::ProjectScope, host: &str) -> anyhow::Result<std::collections::BTreeMap<String, u64>> {
    agent_usage::events::revisions(&super::context::database()?, scope, host)
}
