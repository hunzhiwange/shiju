//! Cancellation responses retain the real state, including a too-late commit.
use vw_api_types::{agent_apps::DataManagementTarget, id::TaskId,
    device_sync_task::{SyncErrorCode, SyncTaskStatus}};

/// Accepted running work stays Running until its resources have been dropped.
/// A commit already in flight is never interrupted by this request.
pub enum CancelOutcome {
    Accepted(SyncTaskStatus),
    TooLate(SyncTaskStatus),
}

/// The caller must reauthorize the complete target before requesting cancellation.
pub fn cancel(target: &DataManagementTarget, id: &TaskId)
    -> Result<CancelOutcome, SyncErrorCode> {
    super::tasks::request_cancel(target, id)
}
