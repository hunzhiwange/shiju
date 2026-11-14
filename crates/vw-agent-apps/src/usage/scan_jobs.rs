//! Scope-local scan coordination shared by manual and page refresh callers.
//!
//! The execution host supplies authorized file discovery and adapter dispatch.
//! A step commits at most one complete file using the existing transactional
//! scanners. Cancellation never drops a scanner halfway through its transaction.
use std::{collections::HashMap, sync::{Arc, Mutex}, time::SystemTime};
use serde::{Deserialize, Serialize};
use vw_api_types::id::{ProjectScope, TaskId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanAdapter { Claude, Codex, Gemini, OpenCode, GrokBuild, Pi, Omp, MCode, Agy, Kimi, Dsh }

pub const SCAN_ADAPTERS: [ScanAdapter; 11] = [
    ScanAdapter::Claude, ScanAdapter::Codex, ScanAdapter::Gemini,
    ScanAdapter::OpenCode, ScanAdapter::GrokBuild, ScanAdapter::Pi,
    ScanAdapter::Omp, ScanAdapter::MCode, ScanAdapter::Agy,
    ScanAdapter::Kimi, ScanAdapter::Dsh,
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanCounts {
    pub imported: u64,
    pub skipped: u64,
    pub files_scanned: u64,
    pub suspected_duplicates: u64,
    pub deferred_files: u64,
}
impl ScanCounts {
    fn merge(&mut self, other: Self) {
        self.imported = self.imported.saturating_add(other.imported);
        self.skipped = self.skipped.saturating_add(other.skipped);
        self.files_scanned = self.files_scanned.saturating_add(other.files_scanned);
        self.suspected_duplicates = self.suspected_duplicates.saturating_add(other.suspected_duplicates);
        self.deferred_files = self.deferred_files.saturating_add(other.deferred_files);
    }
}

/// Closed error codes prevent native paths or raw log contents reaching clients.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ScanFailure { PermissionDenied, SourceUnavailable, InvalidSource, StorageUnavailable }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanPhase { Queued, Running, Completed, Cancelled, Failed }
impl ScanPhase {
    fn terminal(self) -> bool { matches!(self, Self::Completed | Self::Cancelled | Self::Failed) }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJob {
    pub task_id: TaskId,
    pub phase: ScanPhase,
    pub cancel_requested: bool,
    pub current_adapter: Option<ScanAdapter>,
    pub completed_adapters: usize,
    pub counts: ScanCounts,
    pub errors: Vec<(ScanAdapter, ScanFailure)>,
    pub last_complete_scan: Option<SystemTime>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScanOwner {
    pub scope: ProjectScope,
    pub target_host: String,
    pub principal: String,
}
struct Entry { owner: ScanOwner, snapshot: ScanJob }
#[derive(Default)]
struct State {
    jobs: HashMap<TaskId, Entry>,
    last: HashMap<ScanOwner, SystemTime>,
}
/// Keep one instance for the host lifecycle, shared by all scan entry points.
#[derive(Clone, Default)]
pub struct ScanJobs(Arc<Mutex<State>>);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanJobError { InvalidOwner, NotFound, Unavailable, CapacityExceeded }

/// Adapter dispatch must advance its host-private file iterator even on a file
/// failure. End ends that adapter; Failed ends it with a public diagnostic.
/// Other adapters always continue. No raw source paths belong in this interface.
pub enum ScanStep {
    Committed(ScanCounts),
    End,
    Failed(ScanFailure),
}
impl ScanJobs {
    /// Returns the same active ID to concurrent manual/page requests. The host
    /// may call execute for either result: only one executor can claim the ID.
    pub fn start(&self, owner: &ScanOwner) -> Result<ScanJob, ScanJobError> {
        if [&owner.target_host, &owner.principal].iter()
            .any(|value| value.trim().is_empty() || value.contains('\0')) {
            return Err(ScanJobError::InvalidOwner);
        }
        let mut state = self.0.lock().map_err(|_| ScanJobError::Unavailable)?;
        if let Some(entry) = state.jobs.values().find(|entry| entry.owner == *owner && !entry.snapshot.phase.terminal()) {
            return Ok(entry.snapshot.clone());
        }
        if state.jobs.len() >= 128 {
            let terminal = state.jobs.iter().find(|(_, entry)| entry.snapshot.phase.terminal())
                .map(|(id, _)| id.clone());
            if let Some(id) = terminal { state.jobs.remove(&id); }
            else { return Err(ScanJobError::CapacityExceeded); }
        }
        let snapshot = ScanJob {
            task_id: TaskId::from(uuid::Uuid::new_v4().to_string()),
            phase: ScanPhase::Queued, cancel_requested: false,
            current_adapter: None, completed_adapters: 0, counts: ScanCounts::default(),
            errors: Vec::new(), last_complete_scan: state.last.get(owner).copied(),
        };
        state.jobs.insert(snapshot.task_id.clone(), Entry { owner: owner.clone(), snapshot: snapshot.clone() });
        Ok(snapshot)
    }
    pub fn get(&self, owner: &ScanOwner, id: &TaskId) -> Result<ScanJob, ScanJobError> {
        let state = self.0.lock().map_err(|_| ScanJobError::Unavailable)?;
        Ok(owned(&state, owner, id)?.snapshot.clone())
    }
    pub fn cancel(&self, owner: &ScanOwner, id: &TaskId) -> Result<ScanJob, ScanJobError> {
        let mut state = self.0.lock().map_err(|_| ScanJobError::Unavailable)?;
        owned(&state, owner, id)?;
        let entry = state.jobs.get_mut(id).ok_or(ScanJobError::NotFound)?;
        if !entry.snapshot.phase.terminal() {
            entry.snapshot.cancel_requested = true;
            if entry.snapshot.phase == ScanPhase::Queued { entry.snapshot.phase = ScanPhase::Cancelled; }
        }
        Ok(entry.snapshot.clone())
    }
    pub fn last_complete_scan(&self, owner: &ScanOwner) -> Result<Option<SystemTime>, ScanJobError> {
        let state = self.0.lock().map_err(|_| ScanJobError::Unavailable)?;
        Ok(state.last.get(owner).copied())
    }
    /// Synchronous: run on the host's blocking worker, never the UI thread.
    /// The callback owns adapter-specific iterators and uses existing authorized
    /// scanners. Disabled sources must return End without opening their files.
    pub fn execute(
        &self, owner: &ScanOwner, id: &TaskId,
        mut step: impl FnMut(ScanAdapter) -> ScanStep,
    ) -> Result<ScanJob, ScanJobError> {
        {
            let mut state = self.0.lock().map_err(|_| ScanJobError::Unavailable)?;
            owned(&state, owner, id)?;
            let entry = state.jobs.get_mut(id).ok_or(ScanJobError::NotFound)?;
            if entry.snapshot.phase != ScanPhase::Queued { return Ok(entry.snapshot.clone()); }
            entry.snapshot.phase = ScanPhase::Running;
        }
        // A panicking host adapter must not leave a permanently active job.
        let _guard = ExecutionGuard { jobs: self.clone(), id: id.clone() };
        for adapter in SCAN_ADAPTERS {
            loop {
                {
                    let mut state = self.0.lock().map_err(|_| ScanJobError::Unavailable)?;
                    let job = &mut state.jobs.get_mut(id).ok_or(ScanJobError::NotFound)?.snapshot;
                    if job.cancel_requested {
                        job.phase = ScanPhase::Cancelled;
                        job.current_adapter = None;
                        return Ok(job.clone());
                    }
                    job.current_adapter = Some(adapter);
                }
                let result = step(adapter);
                let mut state = self.0.lock().map_err(|_| ScanJobError::Unavailable)?;
                let job = &mut state.jobs.get_mut(id).ok_or(ScanJobError::NotFound)?.snapshot;
                match result {
                    ScanStep::Committed(counts) => job.counts.merge(counts),
                    ScanStep::End => { job.completed_adapters += 1; break; }
                    ScanStep::Failed(error) => {
                        job.errors.push((adapter, error));
                        job.completed_adapters += 1;
                        break;
                    }
                }
            }
        }
        let mut state = self.0.lock().map_err(|_| ScanJobError::Unavailable)?;
        let job = &mut state.jobs.get_mut(id).ok_or(ScanJobError::NotFound)?.snapshot;
        job.current_adapter = None;
        job.phase = if job.cancel_requested { ScanPhase::Cancelled }
            else if job.errors.is_empty() { ScanPhase::Completed } else { ScanPhase::Failed };
        if job.phase == ScanPhase::Completed { job.last_complete_scan = Some(SystemTime::now()); }
        let result = job.clone();
        if result.phase == ScanPhase::Completed {
            state.last.insert(owner.clone(), result.last_complete_scan.ok_or(ScanJobError::Unavailable)?);
        }
        Ok(result)
    }
}
fn owned<'a>(state: &'a State, owner: &ScanOwner, id: &TaskId) -> Result<&'a Entry, ScanJobError> {
    state.jobs.get(id).filter(|entry| entry.owner == *owner).ok_or(ScanJobError::NotFound)
}
struct ExecutionGuard { jobs: ScanJobs, id: TaskId }
impl Drop for ExecutionGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.jobs.0.lock() {
            if let Some(entry) = state.jobs.get_mut(&self.id) {
                if entry.snapshot.phase == ScanPhase::Running {
                    entry.snapshot.phase = ScanPhase::Failed;
                    entry.snapshot.current_adapter = None;
                }
            }
        }
    }
}
