//! Atomic native Codex detail rebuild. Archived accounting remains authoritative.
use rusqlite::{Connection, Transaction, TransactionBehavior, params};
use vw_api_types::{id::ProjectScope, usage::UsageSourceKind};

use crate::apps::lifecycle::operations::Operation;
use super::{
    codex_lineage::CodexLineage,
    cursors::UsageCursorKey,
    store::{StoredUsageRecord, UsageSaveOutcome, UsageSourceIdentity, save_usage_record_in_transaction},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexRebuildError {
    InvalidInput,
    Cancelled,
    StorageUnavailable,
    ConflictingSource,
    ReplayFailed,
}
impl std::fmt::Display for CodexRebuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for CodexRebuildError {}
fn storage(_: rusqlite::Error) -> CodexRebuildError {
    CodexRebuildError::StorageUnavailable
}
fn check(operation: &Operation) -> Result<(), CodexRebuildError> {
    if operation.is_cancelled() { Err(CodexRebuildError::Cancelled) } else { Ok(()) }
}

/// A narrow replay sink: host readers cannot accidentally write another source.
/// Archived identities are handled by the ledger's ordinary replay barrier.
pub struct CodexRebuildWriter<'a, 'conn> {
    transaction: &'a Transaction<'conn>,
    scope: &'a ProjectScope,
    host: &'a str,
    operation: &'a Operation,
}
impl CodexRebuildWriter<'_, '_> {
    pub fn save(
        &self,
        identity: &UsageSourceIdentity,
        record: &StoredUsageRecord,
    ) -> Result<UsageSaveOutcome, CodexRebuildError> {
        check(self.operation)?;
        if identity.target_host != self.host || identity.app != "codex"
            || identity.kind != UsageSourceKind::Native
        {
            return Err(CodexRebuildError::ConflictingSource);
        }
        save_usage_record_in_transaction(self.transaction, self.scope, identity, record)
            .map_err(|_| CodexRebuildError::ReplayFailed)
    }
}

/// Invoke inside the host's existing scan coordinator and operation worker.
/// SQLite IMMEDIATE serializes the whole reset/replay with incremental scans,
/// including scans using another connection. The callback must use the supplied
/// sink, not start a nested scanner transaction. It receives the cleared lineage
/// cache to project authorized rollout files again. Check cancellation while
/// reading files too; the sink checks every write and commit checks once more.
///
/// Daily archives and their receipts are deliberately retained: pruned details
/// cannot be safely reconstructed from possibly incomplete historical files.
/// Replaying archived identities therefore never adds their cost a second time.
/// Live native Codex details and cursor parser state are replaced atomically;
/// failure/cancellation restores both. Cursors remain absent after success and
/// the next ordinary scan repopulates them using stable replay identities.
/// Cache eviction is harmless even when the database transaction rolls back.
/// Wrap this worker with Operation::start_task to report terminal progress.
pub fn rebuild_codex_usage(
    connection: &mut Connection,
    scope: &ProjectScope,
    host: &str,
    lineage: &mut CodexLineage,
    operation: &Operation,
    replay: impl FnOnce(&CodexRebuildWriter<'_, '_>, &mut CodexLineage) -> Result<(), CodexRebuildError>,
) -> Result<u64, CodexRebuildError> {
    if host.trim().is_empty() || host.contains('\0') {
        return Err(CodexRebuildError::InvalidInput);
    }
    check(operation)?;
    let key = serde_json::to_string(&(scope, host)).map_err(|_| CodexRebuildError::InvalidInput)?;
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let mut statement = tx.prepare(
        "SELECT record_id,payload_json FROM usage_records WHERE scope_key=?1",
    ).map_err(storage)?;
    let mut rows = statement.query([&key]).map_err(storage)?;
    let mut ids = Vec::new();
    while let Some(row) = rows.next().map_err(storage)? {
        check(operation)?;
        let id: String = row.get(0).map_err(storage)?;
        let payload: String = row.get(1).map_err(storage)?;
        let stored: StoredUsageRecord = serde_json::from_str(&payload)
            .map_err(|_| CodexRebuildError::StorageUnavailable)?;
        let native_codex = |source: &vw_api_types::usage::UsageSourceRef| {
            source.app == "codex" && source.kind == UsageSourceKind::Native
        };
        if !stored.record.sources.iter().any(native_codex) { continue; }
        if stored.record.id != id || !stored.record.sources.iter().all(native_codex) {
            return Err(CodexRebuildError::ConflictingSource);
        }
        // Validate all associations before removing a canonical row.
        let mut identities = tx.prepare(
            "SELECT identity_key FROM usage_source_identities WHERE scope_key=?1 AND record_id=?2",
        ).map_err(storage)?;
        let keys = identities.query_map(params![key, id], |r| r.get::<_, String>(0)).map_err(storage)?;
        for identity in keys {
            let identity: UsageSourceIdentity = serde_json::from_str(&identity.map_err(storage)?)
                .map_err(|_| CodexRebuildError::StorageUnavailable)?;
            if identity.app != "codex" || identity.kind != UsageSourceKind::Native
                || identity.target_host != host
            {
                return Err(CodexRebuildError::ConflictingSource);
            }
        }
        ids.push(id);
    }
    drop(rows);
    drop(statement);
    for id in &ids {
        check(operation)?;
        tx.execute("DELETE FROM usage_source_identities WHERE scope_key=?1 AND record_id=?2", params![key, id]).map_err(storage)?;
        tx.execute("DELETE FROM usage_records WHERE scope_key=?1 AND record_id=?2", params![key, id]).map_err(storage)?;
    }
    let mut statement = tx.prepare("SELECT cursor_key FROM usage_scan_cursors").map_err(storage)?;
    let keys = statement.query_map([], |r| r.get::<_, String>(0)).map_err(storage)?
        .collect::<Result<Vec<_>, _>>().map_err(storage)?;
    drop(statement);
    for cursor_key in keys {
        check(operation)?;
        let (cursor_scope, source): (ProjectScope, UsageCursorKey) = serde_json::from_str(&cursor_key)
            .map_err(|_| CodexRebuildError::StorageUnavailable)?;
        if &cursor_scope == scope && source.target_host == host && source.app == "codex" {
            tx.execute("DELETE FROM usage_scan_cursors WHERE cursor_key=?1", [cursor_key]).map_err(storage)?;
        }
    }
    lineage.clear_cache();
    let writer = CodexRebuildWriter { transaction: &tx, scope, host, operation };
    replay(&writer, lineage)?;
    check(operation)?;
    super::source_settings::advance_statistics_revision(&tx, scope, host, "codex")
        .map_err(|_| CodexRebuildError::StorageUnavailable)?;
    tx.commit().map_err(storage)?;
    Ok(ids.len() as u64)
}
