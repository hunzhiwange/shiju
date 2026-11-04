//! One run's leaf-operation permits. Clones share cancellation and step accounting.
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct VideoEditRunBudget {
    limit: usize,
    permits: Arc<Semaphore>,
    cancellation: CancellationToken,
    steps: Arc<AtomicU32>,
}
impl Default for VideoEditRunBudget {
    fn default() -> Self {
        Self::new(5).expect("fixed valid limit")
    }
}
impl VideoEditRunBudget {
    pub fn new(limit: usize) -> Result<Self, String> {
        if !(1..=5).contains(&limit) {
            return Err("video edit concurrency must be an integer in 1..=5".into());
        }
        Ok(Self {
            limit,
            permits: Arc::new(Semaphore::new(limit)),
            cancellation: CancellationToken::new(),
            steps: Arc::new(AtomicU32::new(10_000)),
        })
    }
    pub fn from_value(value: Option<&Value>) -> Result<Self, String> {
        let limit = match value {
            None => 5,
            Some(value) => value
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| "video edit concurrency must be an integer in 1..=5".to_string())?,
        };
        Self::new(limit)
    }
    pub fn with_cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = cancellation;
        self
    }
    pub fn limit(&self) -> usize {
        self.limit
    }
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
    pub async fn cancelled(&self) {
        self.cancellation.cancelled().await;
    }
    pub async fn acquire(&self) -> Result<OwnedSemaphorePermit, String> {
        tokio::select! {
            biased;
            _ = self.cancelled() => Err("video edit run cancelled".into()),
            permit = self.permits.clone().acquire_owned() => permit.map_err(|_| "video edit budget closed".into()),
        }
    }
    pub(crate) fn set_steps(&self, steps: u32) {
        self.steps.store(steps, Ordering::SeqCst);
    }
    pub(crate) fn consume_step(&self) -> Result<(), String> {
        if self.is_cancelled() {
            return Err("video edit run cancelled".into());
        }
        self.steps
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .map(|_| ())
            .map_err(|_| "workflow shared execution step budget exhausted".into())
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
