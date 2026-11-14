//! Host-owned operation progress. Callers retain handles in their authorized task
//! store; neither this module nor an operation ID grants execution permission.
use tokio::sync::watch;
use vw_api_types::{agent_apps::AppId, task::TaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Queued,
    Preflight,
    Executing,
    Postflight,
    Terminal,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub phase: Phase,
    pub status: TaskStatus,
}
pub enum OperationFailure {
    Cancelled,
    Failed,
}

#[derive(Clone)]
pub struct Operation {
    cancel: watch::Sender<bool>,
    progress: watch::Sender<Progress>,
}
impl Default for Operation {
    fn default() -> Self {
        Self::new()
    }
}
impl Operation {
    pub fn new() -> Self {
        let (progress, _) =
            watch::channel(Progress { phase: Phase::Queued, status: TaskStatus::Queued });
        Self { cancel: watch::Sender::new(false), progress }
    }
    pub fn snapshot(&self) -> Progress {
        self.progress.borrow().clone()
    }
    pub fn subscribe(&self) -> watch::Receiver<Progress> {
        self.progress.subscribe()
    }
    pub fn cancel(&self) {
        self.cancel.send_replace(true);
    }
    /// Cancellation is acknowledged only after the worker and its children finish.
    pub async fn cancel_and_wait(&self) -> Progress {
        self.cancel();
        self.wait().await
    }
    pub async fn wait(&self) -> Progress {
        let mut events = self.subscribe();
        loop {
            let progress = events.borrow().clone();
            if progress.phase == Phase::Terminal {
                return progress;
            }
            if events.changed().await.is_err() {
                return self.snapshot();
            }
        }
    }
    pub fn is_cancelled(&self) -> bool {
        *self.cancel.borrow()
    }
    /// Wait until cancellation is requested, including requests made before subscribing.
    pub async fn cancelled(&self) {
        let mut signal = self.cancel.subscribe();
        let _ = signal.wait_for(|&cancelled| cancelled).await;
    }
    fn set(&self, phase: Phase, status: TaskStatus) {
        self.progress.send_replace(Progress { phase, status });
    }
    /// Runs an existing install/upgrade/uninstall future. The coordinator carries
    /// this context into its blocking worker, so all existing recipes share it.
    pub fn start_action<F>(&self, action: F) -> Result<(), String>
    where
        F: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        self.spawn(async move {
            action.await.map_err(|error| {
                if error == "operation cancelled" {
                    OperationFailure::Cancelled
                } else {
                    OperationFailure::Failed
                }
            })
        })
    }

    /// Run an asynchronous task that reports a typed terminal outcome.
    pub fn start_task<F>(&self, task: F) -> Result<(), String>
    where
        F: std::future::Future<Output = Result<(), OperationFailure>> + Send + 'static,
    {
        let operation = self.clone();
        self.spawn(async move {
            operation.set(Phase::Executing, TaskStatus::Running);
            task.await
        })
    }

    fn spawn<F>(&self, task: F) -> Result<(), String>
    where
        F: std::future::Future<Output = Result<(), OperationFailure>> + Send + 'static,
    {
        self.claim()?;
        let operation = self.clone();
        tokio::spawn(async move {
            let status = match ACTIVE.scope(operation.clone(), task).await {
                Ok(()) => TaskStatus::Completed,
                Err(OperationFailure::Cancelled) => TaskStatus::Cancelled,
                Err(OperationFailure::Failed) => TaskStatus::Failed,
            };
            operation.set(Phase::Terminal, status);
        });
        Ok(())
    }
    fn claim(&self) -> Result<(), String> {
        self.progress
            .send_if_modified(|p| {
                if p.phase != Phase::Queued {
                    return false;
                }
                p.phase = Phase::Preflight;
                true
            })
            .then_some(())
            .ok_or_else(|| "operation already submitted".into())
    }
    /// Begin exactly once, before returning the handle to an HTTP subscriber.
    /// The owned task survives subscribers dropping and never resubmits a write.
    pub fn start<F>(&self, tools: Vec<AppId>, work: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<(), String> + Send + 'static,
    {
        self.start_action(async move {
            super::coordinator::ToolLifecycleCoordinator.run(tools, work).await
        })
    }
}
thread_local! { static CURRENT: std::cell::RefCell<Option<Operation>> = const { std::cell::RefCell::new(None) }; }
pub(super) fn current() -> Option<Operation> {
    CURRENT.with(|slot| slot.borrow().clone())
}
pub(super) fn phase(phase: Phase) {
    if let Some(op) = current() {
        op.set(phase, TaskStatus::Running);
    }
}

tokio::task_local! { static ACTIVE: Operation; }
pub(super) fn active() -> Option<Operation> {
    ACTIVE.try_with(Clone::clone).ok()
}
pub(super) fn with_context<T>(
    operation: Option<Operation>,
    work: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    if operation.as_ref().is_some_and(Operation::is_cancelled) {
        return Err("operation cancelled".into());
    }
    CURRENT.with(|slot| {
        let previous = slot.replace(operation);
        struct Reset(Option<Operation>);
        impl Drop for Reset {
            fn drop(&mut self) {
                CURRENT.with(|s| s.replace(self.0.take()));
            }
        }
        let _reset = Reset(previous);
        phase(Phase::Preflight);
        work()
    })
}
