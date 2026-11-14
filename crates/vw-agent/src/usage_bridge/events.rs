//! Scope-local invalidations over the gateway's existing SSE transport.
//! Durable ledger revisions are read after commit; no mutation hooks can emit
//! rolled-back data. A new subscription always sends a recovery snapshot.
use std::{collections::BTreeMap, sync::OnceLock};
use vw_api_types::ProjectScope;
use vw_agent_apps::usage::cache::UsageCacheKey;

#[derive(Clone)]
pub(crate) struct CacheChange {
    pub scope: ProjectScope,
    pub principal: String,
    pub host: String,
    pub app: String,
    pub provider: String,
    pub revision: String,
}

pub(crate) fn cache_channel() -> &'static tokio::sync::broadcast::Sender<CacheChange> {
    static CHANNEL: OnceLock<tokio::sync::broadcast::Sender<CacheChange>> = OnceLock::new();
    CHANNEL.get_or_init(|| tokio::sync::broadcast::channel(256).0)
}

pub(crate) fn cache_updated(key: &UsageCacheKey, revision: uuid::Uuid) {
    let Ok(serde_json::Value::String(app)) = serde_json::to_value(key.app) else {
        tracing::warn!("usage cache event has invalid application identity");
        return;
    };
    let _ = cache_channel().send(CacheChange {
        scope: key.project.clone(), principal: key.principal.clone(), host: key.host.clone(),
        app, provider: key.provider.clone(), revision: revision.to_string(),
    });
}

pub(crate) fn revisions(scope: &ProjectScope, host: &str) -> anyhow::Result<BTreeMap<String, u64>> {
    let database = super::context::database()?;
    let connection = rusqlite::Connection::open_with_flags(database.sqlite_path()?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    // Read one committed SQLite snapshot. Structured keys prevent delimiter collisions.
    let mut statement = connection.prepare("SELECT scope_key, revision FROM usage_statistics_revisions")?;
    let mut rows = statement.query([])?;
    let mut result = BTreeMap::new();
    while let Some(row) = rows.next()? {
        let key: String = row.get(0)?;
        let (owner, target, app): (ProjectScope, String, String) = serde_json::from_str(&key)?;
        if owner == *scope && target == host { result.insert(app, row.get(1)?); }
    }
    Ok(result)
}
