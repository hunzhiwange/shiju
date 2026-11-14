//! Scoped import progress on the shared host executor. The task owner must hold
//! its target/restore exclusion lock until run returns and persist each update
//! through `save`; a failed save stops further writes. No payloads are reported.
use std::{collections::BTreeSet, path::Path, sync::{Arc, Mutex}};
use vw_api_types::{agent_apps::*, task::TaskStatus};
use crate::apps::lifecycle::operations::Operation;
use super::{rollback::{apply_with_rollback, RollbackError}, safety_snapshot::{OwnerPreimage, SafetySnapshotReceipt, SnapshotTarget}, source::{SourceCategory, SourceScope, SourceSnapshot, SourceSummary}};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationError { Forbidden, CommitCriticalSection, InvalidState, Persistence, Execution, RecoveryRequired }

#[derive(Clone)]
pub struct ImportOperation {
    pub executor: Operation,
    state: Arc<Mutex<DataOperationProgress>>,
}
impl ImportOperation {
    /// The authorized task store owns uniqueness, persistence and exclusion with
    /// restore operations. Rehydrated terminal progress remains queryable.
    pub fn new(progress: DataOperationProgress) -> Result<Self, OperationError> {
        if progress.operation != DataOperation::Import { return Err(OperationError::InvalidState); }
        Ok(Self { executor: Operation::new(), state: Arc::new(Mutex::new(progress)) })
    }
    pub fn snapshot(&self, target: &DataManagementTarget) -> Result<DataOperationProgress, OperationError> {
        let state = self.state.lock().map_err(|_| OperationError::InvalidState)?;
        if &state.target != target { return Err(OperationError::Forbidden); }
        Ok(state.clone())
    }
    pub fn cancel(&self, target: &DataManagementTarget, save: impl FnOnce(&DataOperationProgress) -> Result<(), ()>) -> Result<DataOperationProgress, OperationError> {
        let mut state = self.state.lock().map_err(|_| OperationError::InvalidState)?;
        if &state.target != target { return Err(OperationError::Forbidden); }
        if state.phase == DataOperationPhase::Terminal || state.cancel_requested { return Ok(state.clone()); }
        if state.total_items == Some(state.completed_items) && state.status == TaskStatus::Running { return Err(OperationError::CommitCriticalSection); }
        if matches!(state.phase, DataOperationPhase::Committing | DataOperationPhase::Projecting) { return Err(OperationError::CommitCriticalSection); }
        let mut next = state.clone();
        next.cancel_requested = true;
        save(&next).map_err(|_| OperationError::Persistence)?;
        *state = next;
        Ok(state.clone())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn run<E, R>(
        &self, root: &Path, receipt: &SafetySnapshotReceipt, scope: &SourceScope,
        source: &SourceSnapshot, confirmed: &SourceSummary, selected: &[SourceCategory],
        targets: &[SnapshotTarget], mut commit: impl FnMut(SourceCategory) -> Result<(), E>,
        mut restore: impl FnMut(SourceCategory, &[&OwnerPreimage]) -> Result<(), R>,
        save: impl Fn(&DataOperationProgress) -> Result<(), ()>,
    ) -> Result<(), OperationError> {
        {
            let mut state = self.state.lock().map_err(|_| OperationError::InvalidState)?;
            if state.status != TaskStatus::Queued || state.target.scope != *scope.project()
                || state.target.target_host_id != scope.host_id() || state.target.principal_id != scope.principal_id() {
                return Err(OperationError::InvalidState);
            }
            let mut next = state.clone();
            next.status = TaskStatus::Running;
            next.phase = DataOperationPhase::PreparingRollback;
            next.total_items = Some(selected.len() as u64);
            save(&next).map_err(|_| OperationError::Persistence)?;
            *state = next;
        }
        let touched = std::cell::RefCell::new(BTreeSet::new());
        let report = apply_with_rollback(root, receipt, scope, source, confirmed, selected, targets,
            |category| {
                {
                    let mut state = self.state.lock().map_err(|_| OperationError::InvalidState)?;
                    if state.cancel_requested { return Err(OperationError::Execution); }
                    state.phase = DataOperationPhase::Committing;
                    save(&state).map_err(|_| OperationError::Persistence)?;
                }
                touched.borrow_mut().insert(category);
                let result = commit(category).map_err(|_| OperationError::Execution);
                let mut state = self.state.lock().map_err(|_| OperationError::InvalidState)?;
                // Errors may follow a partial commit: retain the write fact.
                state.committed = true;
                if result.is_ok() { state.completed_items += 1; }
                state.phase = DataOperationPhase::PreparingRollback;
                save(&state).map_err(|_| OperationError::Persistence)?;
                result
            },
            |category, images| {
                if !touched.borrow().contains(&category) { return Ok(()); }
                {
                    let mut state = self.state.lock().map_err(|_| OperationError::InvalidState)?;
                    state.phase = DataOperationPhase::RollingBack;
                    state.rollback = DataRollbackState::InProgress;
                    // Recovery must continue even when progress storage is unavailable.
                    let _ = save(&state);
                }
                restore(category, images).map_err(|_| OperationError::RecoveryRequired)
            });
        let mut state = self.state.lock().map_err(|_| OperationError::InvalidState)?;
        let (result, error) = match report {
            Ok(report) if !report.recovery_required.is_empty() => {
                state.rollback = DataRollbackState::Failed;
                (DataOperationResult::Partial { errors: vec![DataManagementError { code: DataManagementErrorCode::RecoveryFailed, category: None, entry_id: None }] }, Some(OperationError::RecoveryRequired))
            }
            Ok(report) if report.apply.failed.is_some() => {
                state.rollback = if touched.borrow().is_empty() { DataRollbackState::NotRequired } else { DataRollbackState::Restored };
                if state.cancel_requested { (DataOperationResult::Cancelled, None) }
                else { (failed(), Some(OperationError::Execution)) }
            }
            Ok(_) => (DataOperationResult::Succeeded, None),
            Err(RollbackError::Snapshot(_) | RollbackError::Apply(_) | RollbackError::InvalidSelection) => (failed(), Some(OperationError::Execution)),
        };
        let mut next = state.clone();
        next.status = match &result { DataOperationResult::Succeeded => TaskStatus::Completed, DataOperationResult::Cancelled => TaskStatus::Cancelled, _ => TaskStatus::Failed };
        next.phase = DataOperationPhase::Terminal;
        next.result = Some(result);
        save(&next).map_err(|_| OperationError::Persistence)?;
        *state = next;
        error.map_or(Ok(()), Err)
    }
}
fn failed() -> DataOperationResult {
    DataOperationResult::Failed { error: DataManagementError { code: DataManagementErrorCode::StorageUnavailable, category: None, entry_id: None } }
}
