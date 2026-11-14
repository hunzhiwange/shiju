//! Conservative cross-source reconciliation. Fingerprints alone never erase usage.
use rusqlite::{Connection, OptionalExtension, params};
use vw_api_types::usage::UsageSourceKind;
use super::store::{StoredUsageRecord, UsageSourceIdentity, UsageStoreError};

fn storage(_: rusqlite::Error) -> UsageStoreError { UsageStoreError::StorageUnavailable }

pub(super) fn initialize(connection: &Connection) -> Result<(), UsageStoreError> {
    // Receipts survive detail pruning, preventing late sources from billing twice.
    connection.execute_batch("CREATE TABLE IF NOT EXISTS usage_dedup_receipts (
        scope_key TEXT NOT NULL, identity_key TEXT NOT NULL, record_id TEXT NOT NULL,
        payload_json TEXT NOT NULL, PRIMARY KEY(scope_key, identity_key));")
        .map_err(storage)
}

fn namespace(app: &str) -> &str {
    match app { "claude-desktop" => "claude", other => other }
}

fn same_semantics(a: &StoredUsageRecord, b: &StoredUsageRecord) -> bool {
    let a = &a.record;
    let b = &b.record;
    a.provider == b.provider && a.tokens == b.tokens && a.tokens.is_some()
        && a.models.request_model == b.models.request_model
        && a.models.response_model == b.models.response_model
        && a.status == b.status && a.billable == b.billable
        && a.started_at_unix_seconds.0.abs_diff(b.started_at_unix_seconds.0) <= 600
}

pub(super) struct DedupMatch {
    pub canonical: Option<String>,
    pub suspected: Option<String>,
}

/// Reads only the already authorized scope/host. Same-kind retries remain distinct.
pub(super) fn find(
    connection: &Connection, scope_key: &str, identity: &UsageSourceIdentity,
    value: &StoredUsageRecord,
) -> Result<DedupMatch, UsageStoreError> {
    let mut statement = connection.prepare("SELECT i.identity_key,r.record_id,r.payload_json
        FROM usage_source_identities i JOIN usage_records r
        ON r.scope_key=i.scope_key AND r.record_id=i.record_id WHERE i.scope_key=?1
        UNION SELECT identity_key,record_id,payload_json FROM usage_dedup_receipts WHERE scope_key=?1
        ORDER BY record_id,identity_key").map_err(storage)?;
    let mut rows = statement.query([scope_key]).map_err(storage)?;
    let mut result = DedupMatch { canonical: None, suspected: None };
    while let Some(row) = rows.next().map_err(storage)? {
        let key: String = row.get(0).map_err(storage)?;
        let previous: UsageSourceIdentity = serde_json::from_str(&key)
            .map_err(|_| UsageStoreError::StorageUnavailable)?;
        if namespace(&previous.app) != namespace(&identity.app)
            || previous.kind == identity.kind
            || !matches!((previous.kind, identity.kind),
                (UsageSourceKind::Native, UsageSourceKind::Relay) |
                (UsageSourceKind::Relay, UsageSourceKind::Native) |
                (UsageSourceKind::Shiju, UsageSourceKind::Native) |
                (UsageSourceKind::Native, UsageSourceKind::Shiju) |
                (UsageSourceKind::Shiju, UsageSourceKind::Relay) |
                (UsageSourceKind::Relay, UsageSourceKind::Shiju)) { continue; }
        let payload: String = row.get(2).map_err(storage)?;
        let stored: StoredUsageRecord = serde_json::from_str(&payload)
            .map_err(|_| UsageStoreError::StorageUnavailable)?;
        if !same_semantics(&stored, value) { continue; }
        let id: String = row.get(1).map_err(storage)?;
        if previous.native_request_id == identity.native_request_id {
            // Ambiguous stable IDs must not silently pick one request.
            if result.canonical.as_ref().is_some_and(|existing| existing != &id) {
                return Ok(DedupMatch { canonical: None, suspected: Some(id) });
            }
            result.canonical = Some(id);
        } else if result.suspected.is_none() { result.suspected = Some(id); }
    }
    Ok(result)
}

pub(super) fn receipt(
    connection: &Connection, scope: &str, identity: &str, id: &str, payload: &str,
) -> Result<(), UsageStoreError> {
    connection.execute("INSERT OR IGNORE INTO usage_dedup_receipts VALUES (?1,?2,?3,?4)",
        params![scope, identity, id, payload]).map_err(storage)?;
    Ok(())
}

pub(super) fn attach(
    connection: &Connection, scope: &str, id: &str, incoming: &StoredUsageRecord,
) -> Result<(), UsageStoreError> {
    let payload: Option<String> = connection.query_row(
        "SELECT payload_json FROM usage_records WHERE scope_key=?1 AND record_id=?2",
        params![scope,id], |row| row.get(0)).optional().map_err(storage)?;
    if let Some(payload) = payload {
        let mut stored: StoredUsageRecord = serde_json::from_str(&payload)
            .map_err(|_| UsageStoreError::StorageUnavailable)?;
        for source in &incoming.record.sources {
            if !stored.record.sources.contains(source) { stored.record.sources.push(source.clone()); }
        }
        let payload = serde_json::to_string(&stored).map_err(|_| UsageStoreError::InvalidInput)?;
        connection.execute("UPDATE usage_records SET payload_json=?3 WHERE scope_key=?1 AND record_id=?2",
            params![scope,id,payload]).map_err(storage)?;
    }
    Ok(())
}
