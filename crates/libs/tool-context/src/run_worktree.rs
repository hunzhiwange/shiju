//! Scheduler-owned worktree operations. The host retains the scheduler store and claims.
use crate::context::WorktreeBindingState;
use std::sync::Arc;

pub trait RunWorktreePort: Send + Sync {
    fn move_workflow_step(&self, return_to_previous: bool) -> anyhow::Result<serde_json::Value>;
    fn validate(&self) -> anyhow::Result<()>;
    fn claim_directory(&self, directory: &str) -> anyhow::Result<()>;
    fn retain_unconfirmed(&self) -> anyhow::Result<()>;
    fn release_directories(&self) -> anyhow::Result<()>;
    fn record(&self, binding: &WorktreeBindingState, action: &str) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct RunWorktreeOwner(Arc<dyn RunWorktreePort>);
impl RunWorktreeOwner {
    pub fn new(owner: impl RunWorktreePort + 'static) -> Self { Self(Arc::new(owner)) }
    pub fn move_workflow_step(&self, return_to_previous: bool) -> anyhow::Result<serde_json::Value> { self.0.move_workflow_step(return_to_previous) }
    pub fn validate(&self) -> anyhow::Result<()> { self.0.validate() }
    pub fn claim_directory(&self, directory: &str) -> anyhow::Result<()> { self.0.claim_directory(directory) }
    pub fn retain_unconfirmed(&self) -> anyhow::Result<()> { self.0.retain_unconfirmed() }
    pub fn release_directories(&self) -> anyhow::Result<()> { self.0.release_directories() }
    pub fn record(&self, binding: &WorktreeBindingState, action: &str) -> anyhow::Result<()> { self.0.record(binding, action) }
}
