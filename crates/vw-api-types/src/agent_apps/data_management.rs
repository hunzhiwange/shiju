//! Data-management transport contracts, not execution or authorization.
//!
//! IDs are opaque server references. Handlers must bind every read, mutation and
//! task lookup to the authenticated tenant, principal, project and execution host.
//! No request accepts SQL, a filesystem path, credentials or native configuration.
use crate::id::{ProjectScope, TaskId};
use crate::task::TaskStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DataTransferId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DataPreviewId(pub String);

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

/// Portable artifacts always exclude credentials, host paths and integrations.
/// Local rollback artifacts are host-bound and must not become download handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataArtifactKind {
    PortableExport,
    LocalRollback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataArchiveFormat {
    Sql,
    NativePackage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceKind {
    CcSwitch,
    Shiju,
}

/// Digest and opaque references come from inspected server-owned resources;
/// source IDs, filenames and raw source metadata are not safe display text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataSourceSummary {
    pub transfer_id: DataTransferId,
    pub kind: DataSourceKind,
    pub application_version: String,
    pub schema_version: u32,
    pub format_version: u32,
    pub format: DataArchiveFormat,
    pub digest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentifyDataSourceRequest {
    pub target: DataManagementTarget,
    pub transfer_id: DataTransferId,
}

/// Empty selections must be rejected by the owner. Entry IDs are preview-issued
/// opaque handles, never source labels, paths or target storage namespace keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataSelection {
    pub category: DataCategory,
    pub entries: DataEntrySelection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataEntrySelection {
    All,
    Entries { ids: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewDataImportRequest {
    pub target: DataManagementTarget,
    pub transfer_id: DataTransferId,
    pub selections: Vec<DataSelection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataConflictDecision {
    Skip,
    KeepBoth,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataConflictResolution {
    pub conflict_id: String,
    pub decision: DataConflictDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataConflictSummary {
    pub conflict_id: String,
    pub category: DataCategory,
    pub entry_id: String,
    pub allowed_decisions: Vec<DataConflictDecision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataExclusionReason {
    Credentials,
    AbsolutePath,
    SystemIntegration,
    UnsupportedField,
    UnsupportedCategory,
    DuplicateUsage,
}

/// Counts instead of source payloads ensure previews do not serialize secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataExclusionSummary {
    pub category: DataCategory,
    pub reason: DataExclusionReason,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataCategorySummary {
    pub category: DataCategory,
    pub selected: u64,
    pub eligible: u64,
    pub excluded: u64,
}

/// Revision binds source digest, selections, conflicts and observed target state.
/// Preview issuance grants no write authority; execution must revalidate it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataImportPreview {
    pub target: DataManagementTarget,
    pub preview_id: DataPreviewId,
    pub revision: String,
    pub source: DataSourceSummary,
    pub categories: Vec<DataCategorySummary>,
    pub conflicts: Vec<DataConflictSummary>,
    pub exclusions: Vec<DataExclusionSummary>,
}

/// Same idempotency key with different inputs must conflict, not start a new job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecuteDataImportRequest {
    pub target: DataManagementTarget,
    pub preview_id: DataPreviewId,
    pub expected_revision: String,
    pub conflicts: Vec<DataConflictResolution>,
    pub idempotency_key: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportDataRequest {
    pub target: DataManagementTarget,
    pub selections: Vec<DataSelection>,
    pub format: DataArchiveFormat,
    pub idempotency_key: String,
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

/// Closed reporting vocabulary, not an arbitrary command dispatch interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataOperation {
    Import,
    Export,
    CreateBackup,
    RestoreBackup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataOperationPhase {
    Queued,
    Reading,
    Parsing,
    Previewing,
    PreparingRollback,
    Committing,
    Projecting,
    RollingBack,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataRollbackState {
    NotRequired,
    Prepared,
    InProgress,
    Restored,
    Conflict,
    Failed,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataItemOutcome {
    Applied,
    Skipped,
    Failed { error: DataManagementErrorCode },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataItemResult {
    pub category: DataCategory,
    pub entry_id: String,
    pub outcome: DataItemOutcome,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataOperationProgress {
    pub target: DataManagementTarget,
    pub operation_id: TaskId,
    pub operation: DataOperation,
    pub status: TaskStatus,
    pub phase: DataOperationPhase,
    pub completed_items: u64,
    pub total_items: Option<u64>,
    pub cancel_requested: bool,
    /// True after the data commit, including when projection subsequently fails.
    pub committed: bool,
    pub rollback: DataRollbackState,
    pub rollback_backup_id: Option<DataBackupId>,
    pub items: Vec<DataItemResult>,
    pub result: Option<DataOperationResult>,
    pub created_backup: Option<DataBackupSummary>,
    /// Only portable exports may produce a downloadable transfer reference.
    pub exported_transfer_id: Option<DataTransferId>,
}

/// Both query and cancellation use the original scope, not just a global task ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDataOperationRequest {
    pub target: DataManagementTarget,
    pub operation_id: TaskId,
}

/// A cancellation request is acknowledgement of intent. During commit the owner
/// must finish or roll back before publishing a terminal cancelled result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelDataOperationRequest {
    pub target: DataManagementTarget,
    pub operation_id: TaskId,
}
