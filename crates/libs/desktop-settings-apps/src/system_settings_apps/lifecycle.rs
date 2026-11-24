use vw_api_app_operations::{AppActionPreview, AppActionPreviewRequest, AppOperationPhase, AppOperationStatus};
// Desktop interaction state; plans, preflight and execution belong to the gateway.
use vw_api_task::TaskStatus;

#[derive(Clone, Debug, Default)]
pub struct LifecycleState {
    pub batch: Option<BatchUpgrade>,
    pub target_label: String,
    pub request: Option<AppActionPreviewRequest>,
    pub preview: Option<AppActionPreview>,
    pub operation: Option<AppOperationStatus>,
    pub busy: bool,
    pub uncertain: bool,
    /// A cancellation request is in flight or has been acknowledged; only SDK status ends it.
    pub cancel_requested: bool,
    pub error: Option<String>,
    pub generation: u64,
}
impl LifecycleState {
    pub fn invalidate(&mut self) {
        let generation = self.generation.wrapping_add(1);
        *self = Self { generation, ..Default::default() };
    }
    pub fn accept_status(&mut self, status: AppOperationStatus) -> bool {
        if let Some(current) = &self.operation
            && (current.operation_id != status.operation_id
                || current.phase == AppOperationPhase::Terminal)
            {
                return false;
            }
        if status.phase == AppOperationPhase::Terminal {
            self.cancel_requested = false;
        }
        self.operation = Some(status);
        true
    }
    pub fn active(&self) -> bool {
        self.busy
            || self.uncertain
            || self
                .operation
                .as_ref()
                .is_some_and(|s| matches!(s.status, TaskStatus::Queued | TaskStatus::Running))
    }
}

/// A fixed, explicitly confirmed selection; never recompute candidates while running.
#[derive(Clone, Debug, Default)]
pub struct BatchUpgrade {
    pub items: Vec<vw_api_app_identity::AppInstallation>,
    pub catalog_generation: Option<u64>,
    pub confirmed: bool,
    pub stopped: bool,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}
impl BatchUpgrade {
    pub fn pending(&self) -> usize {
        self.items.len().saturating_sub(self.completed + self.failed + self.cancelled)
    }
}
