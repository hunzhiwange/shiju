//! Persisted skill task snapshots. IDs are opaque; storage must authorize scope
//! and execution host before returning any snapshot. No operation is replayed.
use serde::{Deserialize, Serialize};
use vw_api_types::id::TaskId;

pub const MAX_RETAINED_JOBS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillJobPhase {
    Started,
    Staged,
    Committing,
    Completed,
    Failed,
    Interrupted,
}

impl SkillJobPhase {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Interrupted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillJob {
    /// Immutable update result retained across later installations.
    #[serde(default)]
    pub update_version: Option<u64>,
    #[serde(default)]
    pub content_hash: Option<String>,
    pub task_id: TaskId,
    pub phase: SkillJobPhase,
    /// Only present after the installation transaction committed.
    pub installed_directory: Option<String>,
    /// A commit may have happened before interruption; inspect inventory first.
    pub requires_reconciliation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSkillJob {
    pub snapshot: SkillJob,
    /// Execution process identity, never a credential or client supplied value.
    pub execution_epoch: String,
    #[serde(default)]
    pub cancel_requested: bool,
    /// Legacy snapshots without ownership are never exposed by the jobs API.
    #[serde(default)]
    pub owner_principal: Option<String>,
    #[serde(default)]
    pub resource_root: Option<std::path::PathBuf>,
}

impl SavedSkillJob {
    pub fn new(execution_epoch: String) -> Self {
        Self {
            snapshot: SkillJob {
                update_version: None,
                content_hash: None,
                task_id: TaskId::from(uuid::Uuid::new_v4().to_string()),
                phase: SkillJobPhase::Started,
                installed_directory: None,
                requires_reconciliation: false,
            },
            execution_epoch,
            cancel_requested: false,
            owner_principal: None,
            resource_root: None,
        }
    }

    pub fn interrupt_previous_execution(&mut self, epoch: &str) {
        if self.execution_epoch != epoch && !self.snapshot.phase.is_terminal() {
            self.snapshot.requires_reconciliation = self.snapshot.phase == SkillJobPhase::Committing;
            self.snapshot.phase = SkillJobPhase::Interrupted;
        }
    }
}
