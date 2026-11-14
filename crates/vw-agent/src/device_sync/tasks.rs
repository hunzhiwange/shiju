//! Process-lifetime device-sync execution records. Authorization remains with the caller.
use std::{future::Future, sync::{Mutex, OnceLock}};
use vw_api_types::{agent_apps::DataManagementTarget, id::TaskId, device_sync_task::{
    SyncStartRequest, SyncTaskOrigin, SyncTaskStatus, SyncTaskPhase, SyncTaskState,
    SyncError, SyncErrorCode, SyncRemoteSnapshot, SyncTaskWarning,
}};

type Result<T> = std::result::Result<T, SyncErrorCode>;
struct Record {
    request: SyncStartRequest,
    status: SyncTaskStatus,
    claimed: bool,
    cancel_requested: bool,
    cancellation: tokio_util::sync::CancellationToken,
    commit_started: bool,
    committed: bool,
}
fn records() -> &'static Mutex<Vec<Record>> {
    static RECORDS: OnceLock<Mutex<Vec<Record>>> = OnceLock::new();
    RECORDS.get_or_init(Default::default)
}

/// Accept only after resolving and authorizing the complete canonical target.
/// Reusing an ID with different parameters is rejected, including after completion.
/// Records are intentionally retained for this process lifetime to preserve deduplication.
pub fn accept(request: SyncStartRequest, origin: SyncTaskOrigin) -> Result<SyncTaskStatus> {
    if request.request_id.trim().is_empty() || request.request_id.len() > 256 {
        return Err(SyncErrorCode::InvalidRequest);
    }
    let mut records = records().lock().map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    if let Some(record) = records.iter().find(|r| r.request.target == request.target
        && r.request.request_id == request.request_id) {
        return if record.request == request && record.status.origin == origin {
            Ok(record.status.clone())
        } else { Err(SyncErrorCode::IdempotencyConflict) };
    }
    let status = SyncTaskStatus {
        task_id: TaskId::from(uuid::Uuid::new_v4().to_string()),
        target: request.target.clone(), transport: request.transport,
        direction: request.direction, origin, phase: SyncTaskPhase::WaitingForLock,
        bytes_transferred: 0, total_bytes: None, state: SyncTaskState::Queued,
        warnings: Vec::new(),
    };
    records.push(Record { request, status: status.clone(), claimed: false,
        cancel_requested: false, cancellation: tokio_util::sync::CancellationToken::new(), commit_started: false, committed: false });
    Ok(status)
}
fn with_record<T>(target: &DataManagementTarget, id: &TaskId,
    operation: impl FnOnce(&mut Record) -> Result<T>) -> Result<T> {
    let mut records = records().lock().map_err(|_| SyncErrorCode::StatePersistenceFailed)?;
    let record = records.iter_mut().find(|r| &r.status.task_id == id && &r.status.target == target)
        .ok_or(SyncErrorCode::Unauthorized)?;
    operation(record)
}
/// Reauthorize the complete target on every read/cancel; IDs alone confer no access.
pub fn status(target: &DataManagementTarget, id: &TaskId) -> Result<SyncTaskStatus> {
    with_record(target, id, |r| Ok(r.status.clone()))
}
pub fn cancel(target: &DataManagementTarget, id: &TaskId) -> Result<SyncTaskStatus> {
    match super::cancel::cancel(target, id)? {
        super::cancel::CancelOutcome::Accepted(status) => Ok(status),
        super::cancel::CancelOutcome::TooLate(_) => Err(SyncErrorCode::TooLate),
    }
}

pub(super) fn request_cancel(target: &DataManagementTarget, id: &TaskId)
    -> Result<super::cancel::CancelOutcome> {
    with_record(target, id, |r| {
        if r.commit_started || (r.status.state.is_terminal()
            && r.status.state != SyncTaskState::Cancelled) {
            return Ok(super::cancel::CancelOutcome::TooLate(r.status.clone()));
        }
        if !r.status.state.is_terminal() {
            r.cancel_requested = true;
            r.cancellation.cancel();
            if !r.claimed { r.status.state = SyncTaskState::Cancelled; }
        }
        Ok(super::cancel::CancelOutcome::Accepted(r.status.clone()))
    })
}

/// Exclusive execution lease; dropping the worker records interruption, never success.
/// Callers await transfer/apply inline under run_with_sync_lock. Never race a
/// commit future against cancellation. Start the commit boundary before manifest
/// publication or local application, and mark committed after publication succeeds.
pub struct SyncTaskExecution { target: DataManagementTarget, id: TaskId, owns_execution: bool }
impl SyncTaskExecution {
    pub fn checkpoint(&self, phase: SyncTaskPhase, bytes: u64, total: Option<u64>) -> Result<()> {
        with_record(&self.target, &self.id, |r| {
            if r.status.state.is_terminal() { return Err(SyncErrorCode::TooLate); }
            if r.cancel_requested && !r.commit_started { return Err(SyncErrorCode::Cancelled); }
            if rank(phase) < rank(r.status.phase) || bytes < r.status.bytes_transferred
                || total.is_some_and(|n| n < bytes)
                || r.status.total_bytes.is_some_and(|old| total.is_none_or(|n| n < old)) {
                return Err(SyncErrorCode::InvalidRequest);
            }
            if matches!(phase, SyncTaskPhase::Committing | SyncTaskPhase::Projecting
                | SyncTaskPhase::RollingBack | SyncTaskPhase::PersistingStatus) && !r.commit_started {
                return Err(SyncErrorCode::InvalidRequest);
            }
            r.status.phase = phase;
            r.status.bytes_transferred = bytes;
            r.status.total_bytes = total;
            Ok(())
        })
    }
    /// Serialized with cancellation: an accepted cancellation prevents publication.
    pub fn begin_commit(&self) -> Result<()> {
        with_record(&self.target, &self.id, |r| {
            if r.cancel_requested { return Err(SyncErrorCode::Cancelled); }
            if r.status.state.is_terminal() { return Err(SyncErrorCode::TooLate); }
            r.commit_started = true;
            Ok(())
        })
    }
    pub fn mark_committed(&self) -> Result<()> {
        with_record(&self.target, &self.id, |r| {
            if !r.commit_started || r.status.state.is_terminal() { return Err(SyncErrorCode::InvalidRequest); }
            r.committed = true;
            Ok(())
        })
    }
    pub fn warn(&self, warning: SyncTaskWarning) -> Result<()> {
        with_record(&self.target, &self.id, |r| {
            if r.status.state.is_terminal() { return Err(SyncErrorCode::TooLate); }
            if !r.status.warnings.contains(&warning) { r.status.warnings.push(warning); }
            Ok(())
        })
    }
}
impl Drop for SyncTaskExecution {
    fn drop(&mut self) {
        if !self.owns_execution { return; }
        let _ = with_record(&self.target, &self.id, |r| {
            if !r.status.state.is_terminal() {
                r.status.state = SyncTaskState::Interrupted { committed: r.committed };
            }
            Ok(())
        });
    }
}

/// Execute an accepted task at most once. The closure owns domain transfer work;
/// safe error codes cross this boundary, never credentials or transport errors.
pub async fn execute<F, Fut>(target: &DataManagementTarget, id: &TaskId, operation: F)
    -> Result<SyncTaskStatus>
where F: FnOnce(SyncTaskExecution) -> Fut,
    Fut: Future<Output = std::result::Result<SyncRemoteSnapshot, SyncErrorCode>>,
{
    with_record(target, id, |r| {
        if r.claimed || r.status.state.is_terminal() { return Err(SyncErrorCode::TooLate); }
        r.claimed = true;
        r.status.state = SyncTaskState::Running;
        Ok(())
    })?;
    // Only this retained guard finalizes interruption; reporting handles do not.
    let guard = SyncTaskExecution { target: target.clone(), id: id.clone(), owns_execution: true };
    let cancellation = with_record(target, id, |r| Ok(r.cancellation.clone()))?;
    let mut work = Box::pin(operation(SyncTaskExecution {
        target: target.clone(), id: id.clone(), owns_execution: false,
    }));
    let result = tokio::select! {
        biased;
        () = cancellation.cancelled() => Err(SyncErrorCode::Cancelled),
        result = &mut work => result,
    };
    // Drop requests, staging directories and lock waiters before publishing Cancelled.
    drop(work);
    with_record(target, id, |r| {
        r.status.state = match result {
            Ok(_) if r.cancel_requested && !r.commit_started => SyncTaskState::Cancelled,
            Ok(snapshot) => SyncTaskState::Succeeded { snapshot },
            Err(_) if r.cancel_requested && !r.commit_started => SyncTaskState::Cancelled,
            Err(code) => SyncTaskState::Failed {
                error: SyncError { code, incompatibility: None }, committed: r.committed },
        };
        Ok(r.status.clone())
    }).map(|status| { drop(guard); status })
}
fn rank(phase: SyncTaskPhase) -> u8 {
    match phase {
        SyncTaskPhase::WaitingForLock => 0, SyncTaskPhase::InspectingRemote => 1,
        SyncTaskPhase::Exporting => 2, SyncTaskPhase::Transferring => 3,
        SyncTaskPhase::Verifying => 4, SyncTaskPhase::Previewing => 5,
        SyncTaskPhase::Committing => 6, SyncTaskPhase::Projecting => 7,
        SyncTaskPhase::RollingBack => 8, SyncTaskPhase::PersistingStatus => 9,
    }
}
