//! Unified ledger persistence on a caller-owned SQLite connection.
//! Callers bind authorization before constructing identities; no paths are opened here.
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use vw_api_types::{
    id::ProjectScope,
    usage::{UsageRecord, UsageSourceKind},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageSourceIdentity {
    pub target_host: String,
    pub app: String,
    pub instance_id: String,
    pub kind: UsageSourceKind,
    /// Stable registry/file identity, never a client-supplied display path.
    pub file_identity: String,
    pub native_session_id: Option<String>,
    /// Stable request or event identity within the file/session.
    pub native_request_id: String,
}

/// Only fixed public messages reach storage. Raw upstream error bodies may contain secrets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageFailure {
    Upstream,
    Timeout,
    Cancelled,
    PermissionDenied,
}

impl UsageFailure {
    fn message(self) -> &'static str {
        match self {
            Self::Upstream => "Upstream request failed",
            Self::Timeout => "Request timed out",
            Self::Cancelled => "Request cancelled",
            Self::PermissionDenied => "Request permission denied",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUsageRecord {
    pub record: UsageRecord,
    pub status_code: Option<u16>,
    pub failure: Option<UsageFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageSaveOutcome {
    Inserted { record_id: String },
    Replayed { record_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageStoreError {
    InvalidInput,
    IdentityConflict,
    StorageUnavailable,
}

impl std::fmt::Display for UsageStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for UsageStoreError {}

fn storage_error(_: rusqlite::Error) -> UsageStoreError {
    UsageStoreError::StorageUnavailable
}

/// Called by the host's schema migration on its existing connection.
pub fn initialize_usage_store(connection: &mut Connection) -> Result<(), UsageStoreError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_records (
            scope_key TEXT NOT NULL,
            record_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            error_message TEXT,
            PRIMARY KEY (scope_key, record_id)
        );
        CREATE TABLE IF NOT EXISTS usage_source_identities (
            scope_key TEXT NOT NULL,
            identity_key TEXT NOT NULL,
            record_id TEXT NOT NULL,
            PRIMARY KEY (scope_key, identity_key),
            FOREIGN KEY (scope_key, record_id) REFERENCES usage_records(scope_key, record_id)
        );",
        )
        .map_err(storage_error)?;
    super::retention::initialize_retention(&transaction)?;
    super::exchange_store::initialize_exchange_store(&transaction)
        .map_err(|_| UsageStoreError::StorageUnavailable)?;
    super::source_settings::initialize_source_settings(&transaction)
        .map_err(|_| UsageStoreError::StorageUnavailable)?;
    super::pricing::initialize_pricing_store(&transaction)
        .map_err(|_| UsageStoreError::StorageUnavailable)?;
    transaction.commit().map_err(storage_error)
}

/// Owns the commit boundary. Any record/source failure rolls back both writes.
/// A source replay returns the original canonical ID without replacing its payload.
pub fn save_usage_record(
    connection: &mut Connection,
    scope: &ProjectScope,
    identity: &UsageSourceIdentity,
    value: &StoredUsageRecord,
) -> Result<UsageSaveOutcome, UsageStoreError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let outcome = save_usage_record_in_transaction(&transaction, scope, identity, value)?;
    transaction.commit().map_err(storage_error)?;
    Ok(outcome)
}

/// For the host's existing transaction facility. The caller MUST propagate errors
/// and commit only after success, allowing future scan cursors to share this transaction.
pub fn save_usage_record_in_transaction(
    transaction: &Transaction<'_>,
    scope: &ProjectScope,
    identity: &UsageSourceIdentity,
    value: &StoredUsageRecord,
) -> Result<UsageSaveOutcome, UsageStoreError> {
    let valid = |s: &str| !s.trim().is_empty() && !s.contains('\0');
    if [
        &identity.target_host,
        &identity.app,
        &identity.instance_id,
        &identity.file_identity,
        &identity.native_request_id,
        &value.record.id,
    ]
    .into_iter()
    .any(|s| !valid(s))
        || identity.native_session_id.as_deref().is_some_and(|s| !valid(s))
        || value.record.sources.len() != 1
    {
        return Err(UsageStoreError::InvalidInput);
    }
    let source = &value.record.sources[0];
    if !valid(&source.source_id)
        || source.kind != identity.kind
        || source.app != identity.app
        || source.native_request_id.as_deref() != Some(identity.native_request_id.as_str())
        || source.native_session_id != identity.native_session_id
    {
        return Err(UsageStoreError::InvalidInput);
    }
    // Structured encoding avoids collisions caused by delimiters in native IDs.
    let scope_key = serde_json::to_string(&(scope, &identity.target_host))
        .map_err(|_| UsageStoreError::InvalidInput)?;
    let identity_key =
        serde_json::to_string(identity).map_err(|_| UsageStoreError::InvalidInput)?;
    let existing: Option<String> = transaction
        .query_row(
            "SELECT record_id FROM usage_source_identities WHERE scope_key=?1 AND identity_key=?2 UNION ALL SELECT record_id FROM usage_archived_identities WHERE scope_key=?1 AND identity_key=?2 LIMIT 1",
            params![scope_key, identity_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_error)?;
    if let Some(record_id) = existing {
        return Ok(UsageSaveOutcome::Replayed { record_id });
    }
    // Different sources are merged only by the later evidence-based reconciler.
    // A caller-chosen colliding canonical ID must never silently attach a source.
    let exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM usage_records WHERE scope_key=?1 AND record_id=?2 UNION ALL SELECT 1 FROM usage_archived_records WHERE scope_key=?1 AND record_id=?2)",
            params![scope_key, value.record.id],
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    if exists {
        return Err(UsageStoreError::IdentityConflict);
    }
    let payload = serde_json::to_string(value).map_err(|_| UsageStoreError::InvalidInput)?;
    transaction.execute(
        "INSERT INTO usage_records(scope_key,record_id,payload_json,error_message) VALUES (?1,?2,?3,?4)",
        params![scope_key, value.record.id, payload, value.failure.map(UsageFailure::message)],
    ).map_err(storage_error)?;
    transaction.execute(
        "INSERT INTO usage_source_identities(scope_key,identity_key,record_id) VALUES (?1,?2,?3)",
        params![scope_key, identity_key, value.record.id],
    ).map_err(storage_error)?;
    super::source_settings::advance_statistics_revision(
        transaction, scope, &identity.target_host, &identity.app,
    )?;
    Ok(UsageSaveOutcome::Inserted { record_id: value.record.id.clone() })
}

