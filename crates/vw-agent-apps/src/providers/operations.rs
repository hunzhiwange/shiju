//! Provider publication boundaries on the existing host operation executor.
use crate::apps::lifecycle::operations::Operation;
use std::sync::{Arc, Mutex};
use vw_api_types::{app_providers::operations::*, id::TaskId, task::TaskStatus};
#[derive(Clone)]
pub struct ProviderOperation {
    executor: Operation,
    state: Arc<Mutex<ProviderOperationProgress>>,
}
impl ProviderOperation {
    pub fn new(id: TaskId, request_id: String) -> Self {
        Self {
            executor: Operation::new(),
            state: Arc::new(Mutex::new(ProviderOperationProgress {
                operation_id: id,
                request_id,
                status: TaskStatus::Queued,
                phase: ProviderOperationPhase::Preparing,
                completed: 0,
                total: 0,
                cancellation_requested: false,
            })),
        }
    }
    pub fn snapshot(&self) -> ProviderOperationProgress {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    pub fn cancel(&self) -> ProviderOperationProgress {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(state.status, TaskStatus::Queued | TaskStatus::Running) {
            state.cancellation_requested = true;
            if state.phase == ProviderOperationPhase::Publishing {
                state.phase = ProviderOperationPhase::Settling;
            }
        }
        state.clone()
    }
    pub fn start(
        &self,
        work: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> Result<(), String> {
        let op = self.clone();
        self.executor.start_action(async move {
            tokio::task::spawn_blocking(move || op.run(work))
                .await
                .map_err(|_| "provider worker unavailable".to_string())?
        })
    }
    pub(super) fn run(&self, work: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
        CURRENT.with(|slot| {
            let previous = slot.replace(Some(self.clone()));
            struct Reset(Option<ProviderOperation>);
            impl Drop for Reset {
                fn drop(&mut self) {
                    CURRENT.with(|s| s.replace(self.0.take()));
                }
            }
            let _reset = Reset(previous);
            self.state.lock().unwrap_or_else(|e| e.into_inner()).status = TaskStatus::Running;
            let result = if self.snapshot().cancellation_requested {
                Err("operation cancelled".into())
            } else {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                    .unwrap_or_else(|_| Err("provider worker unavailable".into()))
            };
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let published = state.phase != ProviderOperationPhase::Preparing;
            state.status = if result.is_ok() {
                TaskStatus::Completed
            } else if !published && state.cancellation_requested {
                TaskStatus::Cancelled
            } else {
                TaskStatus::Failed
            };
            // An interrupted publication retains its existing recovery owner. Never
            // advertise a cancelled write after the irreversible claim.
            state.phase = if published && result.is_err() {
                ProviderOperationPhase::Settling
            } else {
                ProviderOperationPhase::Terminal
            };
            result
        })
    }
}
thread_local! { static CURRENT: std::cell::RefCell<Option<ProviderOperation>> = const { std::cell::RefCell::new(None) }; }
/// Called after planning and immediately before the first persistent mutation.
/// The state lock orders cancellation against the irreversible publication claim.
pub fn publication() -> Result<(), &'static str> {
    CURRENT.with(|slot| {
        if let Some(op) = slot.borrow().as_ref() {
            let mut state = op.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.phase == ProviderOperationPhase::Preparing {
                if state.cancellation_requested {
                    return Err("operation cancelled");
                }
                state.phase = ProviderOperationPhase::Publishing;
            }
        }
        Ok(())
    })
}
pub fn planned(total: usize) {
    CURRENT.with(|slot| {
        if let Some(op) = slot.borrow().as_ref() {
            op.state.lock().unwrap_or_else(|e| e.into_inner()).total = total;
        }
    });
}
pub fn advanced() {
    CURRENT.with(|slot| {
        if let Some(op) = slot.borrow().as_ref() {
            op.state.lock().unwrap_or_else(|e| e.into_inner()).completed += 1;
        }
    });
}
