//! Upstream admission for a trusted application route.

use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore},
    time::timeout,
};

const MAX_ACTIVE: usize = 2;
const MAX_WAITING: usize = 1;
const WAIT_LIMIT: Duration = Duration::from_secs(2);

pub(super) struct RouteCapacity {
    active: Arc<Semaphore>,
    waiting: Semaphore,
}

impl Default for RouteCapacity {
    fn default() -> Self {
        Self { active: Arc::new(Semaphore::new(MAX_ACTIVE)), waiting: Semaphore::new(MAX_WAITING) }
    }
}

impl RouteCapacity {
    pub(super) async fn acquire(&self, buffer: OwnedSemaphorePermit) -> Option<RequestPermit> {
        let active = match Arc::clone(&self.active).try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                let _waiter = self.waiting.try_acquire().ok()?;
                timeout(WAIT_LIMIT, Arc::clone(&self.active).acquire_owned()).await.ok()?.ok()?
            }
        };
        Some(RequestPermit { _buffer: buffer, _active: active })
    }
}

/// Own both reservations until the request or its response body is dropped.
pub(super) struct RequestPermit {
    _buffer: OwnedSemaphorePermit,
    _active: OwnedSemaphorePermit,
}
