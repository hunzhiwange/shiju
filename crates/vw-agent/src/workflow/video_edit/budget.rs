//! One run's leaf-operation permits. Clones share cancellation and step accounting.
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicU32, AtomicUsize, Ordering},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct VideoEditRunBudget {
    active: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    progress: Arc<std::sync::Mutex<crate::workflow::video_edit::assets::VideoAssetProgress>>,
    limit: usize,
    permits: Arc<Semaphore>,
    cancellation: CancellationToken,
    steps: Arc<AtomicU32>,
    assets: Arc<std::sync::Mutex<std::collections::BTreeMap<String, VideoEditPermit>>>,
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
            active: Default::default(),
            peak: Default::default(),
            progress: Default::default(),
            limit,
            permits: Arc::new(Semaphore::new(limit)),
            cancellation: CancellationToken::new(),
            steps: Arc::new(AtomicU32::new(10_000)),
            assets: Default::default(),
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
    pub(crate) fn peak(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }
    fn track(&self, permit: OwnedSemaphorePermit) -> VideoEditPermit {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        VideoEditPermit { _permit: permit, active: self.active.clone() }
    }
    pub(crate) fn set_asset_progress(
        &self,
        progress: crate::workflow::video_edit::assets::VideoAssetProgress,
    ) {
        if let Ok(mut current) = self.progress.lock() {
            *current = progress;
        }
    }
    pub(crate) fn asset_progress(&self) -> crate::workflow::video_edit::assets::VideoAssetProgress {
        self.progress.lock().map(|p| p.clone()).unwrap_or_default()
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
    pub async fn acquire(&self) -> Result<VideoEditPermit, String> {
        tokio::select! {
            biased;
            _ = self.cancelled() => Err("video edit run cancelled".into()),
            permit = self.permits.clone().acquire_owned() => {
                let permit = permit.map_err(|_| "video edit budget closed".to_string())?;
                Ok(self.track(permit))
            },
        }
    }
    pub(crate) async fn reserve_asset(&self, key: &str) -> Result<(), String> {
        if self.assets.lock().map_err(|_| "asset budget poisoned")?.contains_key(key) {
            return Ok(());
        }
        let permit = self.acquire().await?;
        self.assets
            .lock()
            .map_err(|_| "asset budget poisoned")?
            .entry(key.into())
            .or_insert(permit);
        Ok(())
    }
    pub(crate) fn try_reserve_asset(&self, key: &str) -> Result<bool, String> {
        if self.is_cancelled() {
            return Err("video edit run cancelled".into());
        }
        let mut assets = self.assets.lock().map_err(|_| "asset budget poisoned")?;
        if assets.contains_key(key) {
            return Ok(true);
        }
        match self.permits.clone().try_acquire_owned() {
            Ok(permit) => {
                assets.insert(key.into(), self.track(permit));
                Ok(true)
            }
            Err(tokio::sync::TryAcquireError::NoPermits) => Ok(false),
            Err(_) => Err("video edit budget closed".into()),
        }
    }
    pub(crate) fn release_asset(&self, key: &str) {
        if let Ok(mut assets) = self.assets.lock() {
            assets.remove(key);
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

/// A leaf/remote-job lease keeps precise operation accounting until it is dropped.
#[derive(Debug)]
pub struct VideoEditPermit {
    _permit: OwnedSemaphorePermit,
    active: Arc<AtomicUsize>,
}
impl Drop for VideoEditPermit {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::SeqCst);
    }
}
