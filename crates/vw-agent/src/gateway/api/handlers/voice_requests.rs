//! Bounded owner-scoped transcription cancellation and deduplication.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use vw_api_types::{
    id::TenantScope,
    voice::{TranscriptionResponse, VoiceErrorCode as Code},
};
pub(super) type Outcome = Result<TranscriptionResponse, Code>;
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct Owner {
    pub(super) tenant: TenantScope,
    pub(super) principal: [u8; 32],
}
struct Record {
    digest: Option<[u8; 32]>,
    cancel: CancellationToken,
    result: watch::Sender<Option<Outcome>>,
    completed: Option<Instant>,
}
pub(super) struct Requests(
    Mutex<HashMap<Owner, HashMap<String, Record>>>,
    Arc<tokio::sync::Semaphore>,
);
impl Default for Requests {
    fn default() -> Self {
        // Bound concurrent HTTP buffers and upstream operations across owners.
        Self(Mutex::new(HashMap::new()), Arc::new(tokio::sync::Semaphore::new(4)))
    }
}
impl Requests {
    pub(super) fn upload_slot(&self) -> Result<tokio::sync::OwnedSemaphorePermit, Code> {
        self.1.clone().try_acquire_owned().map_err(|_| Code::Backpressure)
    }
}

// Dropping the operation future cancels upstream and wakes every waiter.
// Retained records contain no audio or credentials.
pub(super) struct Pending {
    requests: Arc<Requests>,
    owner: Owner,
    id: String,
    pub(super) cancel: CancellationToken,
}
impl Pending {
    pub(super) fn finish(&self, outcome: Outcome) {
        let mut all = self.requests.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(record) = all.get_mut(&self.owner).and_then(|records| records.get_mut(&self.id))
            && record.completed.is_none()
        {
            record.result.send_replace(Some(outcome));
            record.completed = Some(Instant::now());
        }
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.finish(Err(Code::Cancelled));
    }
}
pub(super) enum Start {
    Run(Pending),
    Wait(watch::Receiver<Option<Outcome>>),
}
impl Requests {
    pub(super) fn start(
        self: &Arc<Self>,
        owner: Owner,
        id: &str,
        digest: [u8; 32],
    ) -> Result<Start, Code> {
        let mut all = self.0.lock().unwrap_or_else(|e| e.into_inner());
        all.retain(|_, records| {
            records.retain(|_, record| {
                record.completed.is_none_or(|at| at.elapsed() < Duration::from_secs(300))
            });
            !records.is_empty()
        });
        let records = all.entry(owner.clone()).or_default();
        if let Some(record) = records.get(id) {
            if record.digest.is_some_and(|previous| previous != digest) {
                return Err(Code::Conflict);
            }
            return Ok(Start::Wait(record.result.subscribe()));
        }
        if records.len() >= 128 || records.values().any(|r| r.completed.is_none()) {
            return Err(Code::Conflict);
        }
        let cancel = CancellationToken::new();
        let (result, _) = watch::channel(None);
        records.insert(
            id.to_owned(),
            Record { digest: Some(digest), cancel: cancel.clone(), result, completed: None },
        );
        Ok(Start::Run(Pending { requests: self.clone(), owner, id: id.to_owned(), cancel }))
    }
}

impl Requests {
    pub(super) fn cancel(&self, owner: Owner, id: String) {
        let mut all = self.0.lock().unwrap_or_else(|e| e.into_inner());
        all.retain(|_, records| {
            records.retain(|_, r| {
                r.completed.is_none_or(|at| at.elapsed() < Duration::from_secs(300))
            });
            !records.is_empty()
        });
        let records = all.entry(owner).or_default();
        if let Some(record) = records.get(&id) {
            record.cancel.cancel();
        } else if records.len() < 128 {
            let (result, _) = watch::channel(Some(Err(Code::Cancelled)));
            records.insert(
                id,
                Record {
                    digest: None,
                    cancel: CancellationToken::new(),
                    result,
                    completed: Some(Instant::now()),
                },
            );
        }
    }
}
#[cfg(test)]
#[path = "voice_requests_tests.rs"]
mod tests;
