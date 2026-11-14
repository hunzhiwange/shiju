//! Device-sync wire contracts. These DTOs grant no authorization or execution.
//! The gateway binds the target to the authenticated tenant/principal and host;
//! task reads and cancellation must reauthorize the same complete target.

use crate::agent_apps::DataManagementTarget;
use crate::device_sync_config::DeviceSyncTransport;
use crate::id::TaskId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    Upload,
    Download,
}

/// Manifest digest identifies the inspected remote revision, including metadata;
/// snapshot_id identifies artifact content. ETag is an optional additional CAS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncRemoteSnapshot {
    pub snapshot_id: String,
    pub manifest_sha256: String,
    pub etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SyncExpectedRemote {
    Empty,
    Snapshot { snapshot: SyncRemoteSnapshot },
}

/// Remote inspection failures are errors, never Empty. Display metadata must be
/// normalized by the server; no raw manifest, URL or transport payload is exposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SyncRemoteInfo {
    Empty,
    Compatible {
        snapshot: SyncRemoteSnapshot,
        device_name: String,
        created_at: String,
        data_compat_version: u32,
        total_bytes: u64,
    },
    Incompatible { reason: SyncIncompatibility },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncIncompatibility {
    Format,
    ProtocolVersion,
    DataCompatVersion,
}

/// Requests cannot carry progress, origin, warnings or a server terminal state.
/// request_id is idempotent only within the complete authorized target/parameters.
/// None means no precondition was supplied, never an assertion of an empty remote.
/// Handlers must require an explicit precondition before destructive replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncStartRequest {
    pub target: DataManagementTarget,
    pub transport: DeviceSyncTransport,
    pub direction: SyncDirection,
    pub request_id: String,
    pub expected_remote_snapshot: Option<SyncExpectedRemote>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTaskOrigin {
    Manual,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTaskPhase {
    WaitingForLock,
    InspectingRemote,
    Exporting,
    Transferring,
    Verifying,
    Previewing,
    Committing,
    Projecting,
    RollingBack,
    PersistingStatus,
}

/// No arbitrary transport error strings: map failures to safe domain codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncErrorCode {
    Unauthorized,
    UnknownHost,
    HostOffline,
    RemoteExecutionUnsupported,
    NotConfigured,
    InvalidRequest,
    IdempotencyConflict,
    RemoteEmpty,
    Incompatible,
    Conflict,
    Integrity,
    UnsafeArchive,
    SizeLimit,
    Transport,
    Timeout,
    Cancelled,
    TooLate,
    SnapshotUnavailable,
    ApplyFailed,
    RollbackFailed,
    StatePersistenceFailed,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncError {
    pub code: SyncErrorCode,
    pub incompatibility: Option<SyncIncompatibility>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTaskWarning {
    ProjectionFailed,
    StatePersistenceFailed,
}

/// A failed rollback may leave data committed. Preserve that fact independently
/// of failure; cancellation is only terminal before the commit critical section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SyncTaskState {
    Queued,
    Running,
    Succeeded { snapshot: SyncRemoteSnapshot },
    Failed { error: SyncError, committed: bool },
    Cancelled,
    Interrupted { committed: bool },
}

impl SyncTaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded { .. }
                | Self::Failed { .. }
                | Self::Cancelled
                | Self::Interrupted { .. }
        )
    }
}

/// Server response/persisted record, never a mutation request. Bytes are monotonic
/// transferred bytes; total may be unknown. Warnings never erase a commit result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncTaskStatus {
    pub task_id: TaskId,
    pub target: DataManagementTarget,
    pub transport: DeviceSyncTransport,
    pub direction: SyncDirection,
    pub origin: SyncTaskOrigin,
    pub phase: SyncTaskPhase,
    pub bytes_transferred: u64,
    pub total_bytes: Option<u64>,
    pub state: SyncTaskState,
    pub warnings: Vec<SyncTaskWarning>,
}

/// Cancellation is a request: accepted running work must still be polled.
/// TooLate preserves the actual state, including commits still in flight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SyncTaskCancelResponse {
    Accepted { status: SyncTaskStatus },
    TooLate { status: SyncTaskStatus },
}
