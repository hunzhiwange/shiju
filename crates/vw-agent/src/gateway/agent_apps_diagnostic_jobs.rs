//! Execution-host lifecycle for application diagnostics.

use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    future::Future,
    io::{ErrorKind, Read, Write},
    panic::AssertUnwindSafe,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

use futures_util::{FutureExt, StreamExt, stream};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::{sync::Semaphore, time::Instant};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use vw_agent_apps::apps::lifecycle::operations::{Operation, OperationFailure};
use vw_api_types::{
    agent_apps_diagnostics::{
        DiagnosticFailure, DiagnosticFailureCode, DiagnosticJobIdentity, DiagnosticJobSnapshot,
        DiagnosticJobState, DiagnosticOutcome, DiagnosticPhase, DiagnosticProbeKind,
        DiagnosticProgress, DiagnosticResult, DiagnosticScope,
    },
    id::TaskId,
};

const MAX_JOBS: usize = 128;
const MAX_PROBES_PER_JOB: usize = 32;
const MAX_CONCURRENT_PROBES: usize = 3;
const MAX_TIMINGS_PER_RESULT: usize = 8;
const MAX_ID_BYTES: usize = 1024;
const MAX_CHECKPOINT_BYTES: u64 = 8 * 1024 * 1024;
const RETENTION: Duration = Duration::from_secs(600);
const INVALID_RESULT: DiagnosticFailure = DiagnosticFailure {
    phase: DiagnosticPhase::Cleanup,
    code: DiagnosticFailureCode::InvalidResponse,
};

/// Identity resolved by the gateway's authorization boundary for both starts and reads.
#[derive(Clone, PartialEq, Eq)]
pub struct DiagnosticJobOwner {
    pub scope: DiagnosticScope,
    pub principal: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DiagnosticJobError {
    InvalidInput,
    CapacityExceeded,
    Unavailable,
    NotFound,
}

/// A worker returns a public failure code or reports that cancellation cleanup is complete.
pub enum DiagnosticProbeError {
    Failed(DiagnosticFailure),
    Cancelled,
}

/// Share one registry per execution host across all diagnostic kinds and callers.
#[derive(Clone)]
pub struct DiagnosticJobs(Arc<Registry>);

struct Registry {
    entries: Mutex<HashMap<TaskId, Job>>,
    permits: Semaphore,
    checkpoint: PathBuf,
    _lease: File,
}

struct Job {
    metadata: JobMetadata,
    results: Option<Vec<DiagnosticResult>>,
    expires: Option<Instant>,
    cancellation: Option<CancellationToken>,
}

// The bounded checkpoint retains only the fields needed to query an interrupted job.
#[derive(Serialize, Deserialize)]
struct JobMetadata {
    identity: DiagnosticJobIdentity,
    principal: String,
    state: DiagnosticJobState,
    progress: DiagnosticProgress,
    expires_at: Option<SystemTime>,
}

impl DiagnosticJobs {
    /// Open a registry in an agent-owned private directory on the execution host.
    /// Call on a blocking thread. Unfinished entries are recovered without replaying probes.
    pub fn open(directory: &Path) -> Result<Self, DiagnosticJobError> {
        Self::open_registry(directory).map_err(|_| DiagnosticJobError::Unavailable)
    }

    fn open_registry(directory: &Path) -> anyhow::Result<Self> {
        let lease = private_file()
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("diagnostics.lock"))?;
        lease.try_lock().map_err(|_| anyhow::anyhow!("diagnostic registry is already open"))?;
        let checkpoint = directory.join("diagnostics.json");
        let mut entries = HashMap::new();
        match private_file().open(&checkpoint) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(MAX_CHECKPOINT_BYTES + 1).read_to_end(&mut bytes)?;
                anyhow::ensure!(
                    bytes.len() as u64 <= MAX_CHECKPOINT_BYTES,
                    "diagnostic checkpoint is too large"
                );
                let jobs: Vec<JobMetadata> = serde_json::from_slice(&bytes)?;
                anyhow::ensure!(jobs.len() <= MAX_JOBS, "too many diagnostic jobs in checkpoint");
                let now = SystemTime::now();
                let restored_at = Instant::now();
                for mut metadata in jobs {
                    if metadata.expires_at.is_none() {
                        metadata.state = DiagnosticJobState::Interrupted {};
                        metadata.expires_at = Some(now + RETENTION);
                    }
                    let Some(remaining) = metadata
                        .expires_at
                        .and_then(|expires| expires.duration_since(now).ok())
                        .filter(|time| !time.is_zero())
                    else {
                        continue;
                    };
                    let id = metadata.identity.task_id.clone();
                    let job = Job {
                        metadata,
                        results: None,
                        expires: Some(restored_at + remaining.min(RETENTION)),
                        cancellation: None,
                    };
                    anyhow::ensure!(entries.insert(id, job).is_none(), "duplicate diagnostic job");
                }
                save(&checkpoint, &entries)?;
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(Self(Arc::new(Registry {
            entries: Mutex::new(entries),
            permits: Semaphore::new(MAX_CONCURRENT_PROBES),
            checkpoint,
            _lease: lease,
        })))
    }

    /// Submit server-prepared work from a blocking thread within the host runtime.
    /// The caller resolves authorized configuration before submission. Probe closures
    /// start I/O only after obtaining a host permit. Each receives a child token
    /// to stop I/O and retry waits, and must close connections and reap children
    /// before returning. The host permit is held until the probe returns.
    pub fn start<F, Fut>(
        &self,
        owner: DiagnosticJobOwner,
        kind: DiagnosticProbeKind,
        probes: Vec<F>,
    ) -> Result<DiagnosticJobIdentity, DiagnosticJobError>
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = Result<DiagnosticResult, DiagnosticProbeError>> + Send + 'static,
    {
        let project = &owner.scope.project;
        let ids = [
            owner.principal.as_str(),
            owner.scope.target_host_id.as_str(),
            project.platform_id().as_ref(),
            project.company_id().as_ref(),
            project.project_id().as_ref(),
        ];
        if probes.is_empty() || probes.len() > MAX_PROBES_PER_JOB || !ids.into_iter().all(valid_id)
        {
            return Err(DiagnosticJobError::InvalidInput);
        }
        tokio::runtime::Handle::try_current().map_err(|_| DiagnosticJobError::Unavailable)?;
        let mut entries = self.0.entries.lock();
        entries.retain(|_, job| !job.expired());
        let evicted = if entries.len() >= MAX_JOBS {
            let oldest = entries
                .iter()
                .filter_map(|(id, job)| job.expires.map(|time| (id, time)))
                .min_by_key(|(_, time)| *time)
                .map(|(id, _)| id.clone())
                .ok_or(DiagnosticJobError::CapacityExceeded)?;
            entries.remove_entry(&oldest)
        } else {
            None
        };
        let identity = DiagnosticJobIdentity {
            task_id: TaskId::from(Uuid::new_v4().to_string()),
            correlation_id: Uuid::new_v4().to_string(),
            scope: owner.scope,
            kind,
        };
        let metadata = JobMetadata {
            identity: identity.clone(),
            principal: owner.principal,
            state: DiagnosticJobState::Queued {},
            progress: DiagnosticProgress { completed: 0, total: Some(probes.len() as u32) },
            expires_at: None,
        };
        let cancellation = CancellationToken::new();
        entries.insert(
            identity.task_id.clone(),
            Job {
                metadata,
                results: Some(Vec::new()),
                expires: None,
                cancellation: Some(cancellation.clone()),
            },
        );
        if save(&self.0.checkpoint, &entries).is_err() {
            entries.remove(&identity.task_id);
            entries.extend(evicted);
            return Err(DiagnosticJobError::Unavailable);
        }

        let registry = self.0.clone();
        let task_id = identity.task_id.clone();
        Operation::new()
            .start_task(async move { registry.run(task_id, kind, probes, cancellation).await })
            .expect("a fresh operation can always start");
        Ok(identity)
    }

    /// Unknown, expired and differently owned tasks have the same lookup result.
    pub fn snapshot(
        &self,
        owner: &DiagnosticJobOwner,
        id: &TaskId,
    ) -> Result<DiagnosticJobSnapshot, DiagnosticJobError> {
        let entries = self.0.entries.lock();
        entries
            .get(id)
            .filter(|job| job.visible_to(owner))
            .map(Job::snapshot)
            .ok_or(DiagnosticJobError::NotFound)
    }

    /// Request cancellation for the authorized owner. Cleanup may still be running.
    pub fn cancel(
        &self,
        owner: &DiagnosticJobOwner,
        id: &TaskId,
    ) -> Result<DiagnosticJobSnapshot, DiagnosticJobError> {
        let mut entries = self.0.entries.lock();
        let job = entries
            .get_mut(id)
            .filter(|job| job.visible_to(owner))
            .ok_or(DiagnosticJobError::NotFound)?;
        if matches!(
            job.metadata.state,
            DiagnosticJobState::Queued {} | DiagnosticJobState::Running {}
        ) {
            job.metadata.state = DiagnosticJobState::Cancelling {};
            job.cancellation.as_ref().expect("active job has a cancellation token").cancel();
        }
        Ok(job.snapshot())
    }
}

impl Job {
    fn expired(&self) -> bool {
        self.expires.is_some_and(|expires| Instant::now() >= expires)
    }

    fn visible_to(&self, owner: &DiagnosticJobOwner) -> bool {
        !self.expired()
            && self.metadata.principal == owner.principal
            && self.metadata.identity.scope == owner.scope
    }

    fn snapshot(&self) -> DiagnosticJobSnapshot {
        DiagnosticJobSnapshot {
            job: self.metadata.identity.clone(),
            state: self.metadata.state,
            progress: self.metadata.progress,
            results: self.results.clone(),
        }
    }
}

impl Registry {
    async fn run<F, Fut>(
        self: Arc<Self>,
        id: TaskId,
        kind: DiagnosticProbeKind,
        probes: Vec<F>,
        cancellation: CancellationToken,
    ) -> Result<(), OperationFailure>
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = Result<DiagnosticResult, DiagnosticProbeError>> + Send + 'static,
    {
        let mut results = stream::iter(probes)
            .map(|probe| {
                let (registry, id, cancellation) = (&self, &id, &cancellation);
                async move {
                    let _permit = tokio::select! {
                        biased;
                        () = cancellation.cancelled() => return None,
                        permit = registry.permits.acquire() => {
                            permit.expect("diagnostic semaphore stays open")
                        }
                    };
                    {
                        let mut entries = registry.entries.lock();
                        let job = entries.get_mut(id).expect("active job is retained");
                        if job.metadata.state == (DiagnosticJobState::Cancelling {}) {
                            return None;
                        }
                        job.metadata.state = DiagnosticJobState::Running {};
                    }
                    // Let the worker finish asynchronous cleanup before dropping its permit.
                    let result =
                        AssertUnwindSafe(async { probe(cancellation.child_token()).await })
                            .catch_unwind()
                            .await
                            .unwrap_or(Err(DiagnosticProbeError::Failed(INVALID_RESULT)));
                    Some(result)
                }
            })
            .buffer_unordered(MAX_CONCURRENT_PROBES);

        let mut failure = None;
        let mut cancelled = false;
        let mut checkpoint_failed = false;
        while let Some(probe) = results.next().await {
            let Some(result) = probe else { continue };
            let result = match result {
                Ok(result) if valid_result(kind, &result) => Some(result),
                Ok(_) => {
                    failure.get_or_insert(INVALID_RESULT);
                    None
                }
                Err(DiagnosticProbeError::Failed(error)) => {
                    failure.get_or_insert(error);
                    None
                }
                Err(DiagnosticProbeError::Cancelled) => {
                    cancelled = true;
                    None
                }
            };
            if !checkpoint_failed {
                let (registry, id) = (self.clone(), id.clone());
                checkpoint_failed =
                    !tokio::task::spawn_blocking(move || registry.record(&id, result))
                        .await
                        .unwrap_or(false);
            }
        }

        let (state, outcome) = if checkpoint_failed {
            (DiagnosticJobState::Interrupted {}, Err(OperationFailure::Failed))
        } else if cancelled {
            (DiagnosticJobState::Cancelled {}, Err(OperationFailure::Cancelled))
        } else if let Some(error) = failure {
            (DiagnosticJobState::Failed { error }, Err(OperationFailure::Failed))
        } else {
            (DiagnosticJobState::Succeeded {}, Ok(()))
        };
        let (registry, id) = (self.clone(), id.clone());
        tokio::task::spawn_blocking(move || registry.finish(&id, state, outcome))
            .await
            .unwrap_or(Err(OperationFailure::Failed))
    }

    fn record(&self, id: &TaskId, result: Option<DiagnosticResult>) -> bool {
        let mut entries = self.entries.lock();
        let job = entries.get_mut(id).expect("active job is retained");
        if job.metadata.state == (DiagnosticJobState::Cancelling {}) {
            return true;
        }
        job.metadata.progress.completed += 1;
        let saved = save(&self.checkpoint, &entries).is_ok();
        let job = entries.get_mut(id).expect("active job is retained");
        if saved {
            job.results.as_mut().expect("active job retains results").extend(result);
        } else {
            job.metadata.progress.completed -= 1;
        }
        saved
    }

    fn finish(
        &self,
        id: &TaskId,
        state: DiagnosticJobState,
        outcome: Result<(), OperationFailure>,
    ) -> Result<(), OperationFailure> {
        let mut entries = self.entries.lock();
        let job = entries.get_mut(id).expect("active job is retained");
        let (state, outcome) = if job.metadata.state == (DiagnosticJobState::Cancelling {}) {
            (DiagnosticJobState::Cancelled {}, Err(OperationFailure::Cancelled))
        } else {
            (state, outcome)
        };
        job.metadata.state = state;
        job.cancellation = None;
        job.metadata.expires_at = Some(SystemTime::now() + RETENTION);
        job.expires = Some(Instant::now() + RETENTION);
        if save(&self.checkpoint, &entries).is_err() {
            // A subsequent process can only prove that this job was interrupted.
            entries.get_mut(id).expect("job is retained").metadata.state =
                DiagnosticJobState::Interrupted {};
            return Err(OperationFailure::Failed);
        }
        outcome
    }
}

fn private_file() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options
}

fn save(checkpoint: &Path, entries: &HashMap<TaskId, Job>) -> anyhow::Result<()> {
    let temporary = checkpoint.with_extension("tmp");
    let metadata: Vec<_> = entries.values().map(|job| &job.metadata).collect();
    let mut file = private_file().write(true).create(true).truncate(true).open(&temporary)?;
    file.write_all(&serde_json::to_vec(&metadata)?)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, checkpoint)?;
    Ok(())
}

fn valid_id(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= MAX_ID_BYTES
}

fn valid_result(kind: DiagnosticProbeKind, result: &DiagnosticResult) -> bool {
    let (result_kind, timings, ids_valid) = match result {
        DiagnosticResult::Reachability { provider_id, report } => {
            (DiagnosticProbeKind::Reachability, &report.timings, valid_id(provider_id.as_ref()))
        }
        DiagnosticResult::Speed { endpoint_id, report } => {
            (DiagnosticProbeKind::Speed, &report.timings, valid_id(&endpoint_id.0))
        }
        DiagnosticResult::Model { provider_id, model_id, report, .. } => {
            let actual_valid = match &report.outcome {
                DiagnosticOutcome::Succeeded { evidence } => {
                    valid_id(evidence.actual_model_id.as_ref())
                }
                _ => true,
            };
            (
                DiagnosticProbeKind::Model,
                &report.timings,
                valid_id(provider_id.as_ref()) && valid_id(model_id.as_ref()) && actual_valid,
            )
        }
        DiagnosticResult::Streaming { provider_id, model_id, report, .. } => {
            let actual_valid = match &report.outcome {
                DiagnosticOutcome::Succeeded { evidence } => {
                    valid_id(evidence.actual_model_id.as_ref())
                }
                _ => true,
            };
            (
                DiagnosticProbeKind::Streaming,
                &report.timings,
                valid_id(provider_id.as_ref()) && valid_id(model_id.as_ref()) && actual_valid,
            )
        }
        DiagnosticResult::Mcp { server_id, report } => {
            (DiagnosticProbeKind::Mcp, &report.timings, valid_id(&server_id.0))
        }
    };
    result_kind == kind && timings.len() <= MAX_TIMINGS_PER_RESULT && ids_valid
}

#[cfg(test)]
#[path = "agent_apps_diagnostic_jobs_tests.rs"]
mod tests;
