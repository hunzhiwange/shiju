//! Shared limits and cancellation for a workflow run and its parallel branches.
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

pub(crate) const MAX_PARALLEL: usize = 5;

#[derive(Clone, Debug)]
pub struct WorkflowRunBudget {
    limit: usize,
    permits: Arc<Semaphore>,
    cancellation: CancellationToken,
    steps: Arc<AtomicU32>,
}

impl Default for WorkflowRunBudget {
    fn default() -> Self {
        Self::new(MAX_PARALLEL)
    }
}

impl WorkflowRunBudget {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            limit,
            permits: Arc::new(Semaphore::new(limit)),
            cancellation: CancellationToken::new(),
            steps: Arc::new(AtomicU32::new(10_000)),
        }
    }

    pub fn with_cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = cancellation;
        self
    }

    pub(crate) fn limit(&self) -> usize {
        self.limit
    }

    #[cfg(test)]
    pub(crate) fn cancel(&self) {
        self.cancellation.cancel();
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    pub(crate) async fn cancelled(&self) {
        self.cancellation.cancelled().await;
    }

    pub(crate) async fn acquire(&self) -> Result<OwnedSemaphorePermit, String> {
        tokio::select! {
            biased;
            _ = self.cancelled() => Err("workflow run cancelled".into()),
            permit = self.permits.clone().acquire_owned() => {
                permit.map_err(|_| "workflow run budget closed".into())
            }
        }
    }

    pub(crate) fn set_steps(&self, steps: u32) {
        self.steps.store(steps, Ordering::SeqCst);
    }

    pub(crate) fn consume_step(&self) -> Result<(), String> {
        if self.is_cancelled() {
            return Err("workflow run cancelled".into());
        }
        self.steps
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |steps| steps.checked_sub(1))
            .map(|_| ())
            .map_err(|_| "workflow shared execution step budget exhausted".into())
    }
}
