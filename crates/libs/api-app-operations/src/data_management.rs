//! Data-management transport contracts, not execution or authorization.
//!
//! IDs are opaque server references. Handlers must bind every read, mutation and
//! task lookup to the authenticated tenant, principal, project and execution host.
//! No request accepts SQL, a filesystem path, credentials or native configuration.
use vw_api_transport::{ProjectScope, TaskId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DataBackupId(pub String);

/// Includes the principal because preferences may be user-owned. Supplied scope
/// is a claim to reauthorize, never permission to impersonate another principal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataManagementTarget {
    pub scope: ProjectScope,
    pub target_host_id: String,
    pub principal_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataCategory {
    Providers,
    Mcp,
    Skills,
    Rules,
    Profiles,
    Preferences,
    Usage,
}

/// Local rollback artifacts are host-bound and must not become download handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataArtifactKind {
    LocalRollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateDataBackupRequest {
    pub target: DataManagementTarget,
    pub categories: Vec<DataCategory>,
    pub kind: DataArtifactKind,
    pub name: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataBackupSummary {
    pub backup_id: DataBackupId,
    pub target: DataManagementTarget,
    /// User-authored backup label, never populated from source configuration.
    pub name: String,
    pub revision: String,
    pub kind: DataArtifactKind,
    pub format_version: u32,
    pub digest_sha256: String,
    pub created_at_ms: i64,
    pub size_bytes: u64,
    pub categories: Vec<DataCategory>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListDataBackupsRequest {
    pub target: DataManagementTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameDataBackupRequest {
    pub target: DataManagementTarget,
    pub backup_id: DataBackupId,
    pub expected_revision: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteDataBackupRequest {
    pub target: DataManagementTarget,
    pub backup_id: DataBackupId,
    pub expected_revision: String,
    pub confirmed: bool,
}

/// Owner must check archive compatibility/integrity and create a new rollback
/// point before restoring. A local rollback is usable only on its original host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreDataBackupRequest {
    pub target: DataManagementTarget,
    pub backup_id: DataBackupId,
    pub expected_revision: String,
    pub categories: Vec<DataCategory>,
    pub idempotency_key: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataManagementErrorCode {
    InvalidInput,
    UnsupportedVersion,
    Forbidden,
    NotFound,
    RevisionConflict,
    IdempotencyConflict,
    IntegrityMismatch,
    StorageUnavailable,
    HostUnavailable,
    Cancelled,
    RecoveryFailed,
    ProjectionFailed,
}

/// Stable codes only: no raw errors, SQL, paths, tokens or diagnostic payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataManagementError {
    pub code: DataManagementErrorCode,
    pub category: Option<DataCategory>,
    pub entry_id: Option<String>,
}

/// Completed task status alone does not establish full success. Partial records
/// retain the fact of committed writes even when projection or rollback failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataOperationResult {
    Succeeded,
    Partial { errors: Vec<DataManagementError> },
    Failed { error: DataManagementError },
    Cancelled,
}

/// Restore status lookup uses the original scope and task ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDataOperationRequest {
    pub target: DataManagementTarget,
    pub operation_id: TaskId,
}
