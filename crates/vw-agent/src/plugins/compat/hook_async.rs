//! Turn-owned asynchronous work. Dropping the owner aborts children and their process guards.
use std::sync::Mutex;
use tokio::task::JoinSet;

pub(crate) struct AsyncHooks {
    tasks: Mutex<JoinSet<()>>,
    closed: std::sync::atomic::AtomicBool,
    permits: std::sync::Arc<tokio::sync::Semaphore>,
}
impl Default for AsyncHooks {
    fn default() -> Self {
        Self {
            tasks: Mutex::default(),
            closed: Default::default(),
            permits: std::sync::Arc::new(tokio::sync::Semaphore::new(8)),
        }
    }
}
impl AsyncHooks {
    pub(crate) fn is_closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub(crate) async fn shutdown(&self) {
        self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
        let mut tasks = match self.tasks.lock() {
            Ok(mut tasks) => std::mem::take(&mut *tasks),
            Err(_) => return,
        };
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
    }

    pub(crate) fn schedule(
        &self,
        task: impl Future<Output = ()> + Send + 'static,
    ) -> anyhow::Result<()> {
        let mut tasks =
            self.tasks.lock().map_err(|_| anyhow::anyhow!("async hook lock poisoned"))?;
        anyhow::ensure!(
            !self.closed.load(std::sync::atomic::Ordering::SeqCst),
            "async hook owner is closed"
        );
        while tasks.try_join_next().is_some() {}
        let permits = self.permits.clone();
        tasks.spawn(async move {
            let Ok(_permit) = permits.acquire_owned().await else {
                return;
            };
            task.await;
        });
        Ok(())
    }
}

/// A turn boundary closes work even when tool-context clones outlive the request.
pub(crate) struct AsyncHookTurn(std::sync::Arc<AsyncHooks>);
impl AsyncHooks {
    pub(crate) fn turn_guard(self: &std::sync::Arc<Self>) -> AsyncHookTurn {
        AsyncHookTurn(self.clone())
    }
}
impl Drop for AsyncHookTurn {
    fn drop(&mut self) {
        self.0.closed.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut tasks) = self.0.tasks.lock() {
            tasks.abort_all();
        }
    }
}
