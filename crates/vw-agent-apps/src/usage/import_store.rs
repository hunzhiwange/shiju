//! Ledger-owned CC migration transaction operations.
use rusqlite::{Connection, Transaction, OptionalExtension, params};
use vw_api_types::id::ProjectScope;
use super::store::{StoredUsageRecord, UsageSourceIdentity, UsageStoreError, save_usage_record_in_transaction};
fn storage_error(_: rusqlite::Error) -> UsageStoreError { UsageStoreError::StorageUnavailable }

/// Internal CC migration reader. Archived identities remain replay barriers and
/// cannot be replaced: retention owns their aggregate accounting.
pub(crate) fn load_imported_usage(
    connection: &Connection, scope: &ProjectScope, identity: &UsageSourceIdentity,
) -> Result<Option<StoredUsageRecord>, UsageStoreError> {
    let scope_key = serde_json::to_string(&(scope, &identity.target_host)).map_err(|_| UsageStoreError::InvalidInput)?;
    let identity_key = serde_json::to_string(identity).map_err(|_| UsageStoreError::InvalidInput)?;
    let archived: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM usage_archived_identities WHERE scope_key=?1 AND identity_key=?2)",
        params![scope_key,identity_key], |r| r.get(0)).map_err(storage_error)?;
    if archived { return Err(UsageStoreError::IdentityConflict); }
    let payload: Option<String> = connection.query_row(
        "SELECT r.payload_json FROM usage_source_identities i JOIN usage_records r ON r.scope_key=i.scope_key AND r.record_id=i.record_id WHERE i.scope_key=?1 AND i.identity_key=?2",
        params![scope_key,identity_key], |r| r.get(0)).optional().map_err(storage_error)?;
    payload.map(|p| serde_json::from_str(&p).map_err(|_| UsageStoreError::InvalidInput)).transpose()
}

/// Replace only this importer-owned, unarchived request. Never changes account
/// ownership or canonical identity. All SQL stays inside the ledger owner.
pub(crate) fn replace_imported_usage_in_transaction(
    tx: &Transaction<'_>, scope: &ProjectScope, identity: &UsageSourceIdentity,
    value: &StoredUsageRecord,
) -> Result<(), UsageStoreError> {
    if !identity.file_identity.starts_with("cc-import:") { return Err(UsageStoreError::InvalidInput); }
    let previous = load_imported_usage(tx, scope, identity)?;
    if let Some(previous) = previous {
        if previous.record.id != value.record.id || previous.record.provider != value.record.provider
            || previous.record.sources != value.record.sources {
            return Err(UsageStoreError::IdentityConflict);
        }
        let scope_key = serde_json::to_string(&(scope, &identity.target_host)).map_err(|_| UsageStoreError::InvalidInput)?;
        let payload = serde_json::to_string(value).map_err(|_| UsageStoreError::InvalidInput)?;
        tx.execute("UPDATE usage_records SET payload_json=?3,error_message=?4 WHERE scope_key=?1 AND record_id=?2",
            params![scope_key,value.record.id,payload,value.failure.map(|_| "Upstream request failed")]).map_err(storage_error)?;
        crate::usage::source_settings::advance_statistics_revision(tx,scope,&identity.target_host,&identity.app)?;
    } else {
        save_usage_record_in_transaction(tx,scope,identity,value)?;
    }
    Ok(())
}
