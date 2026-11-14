//! Maintenance requests identify resources; paths and configuration remain host-owned.
use crate::{ProjectScope, task::TaskStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryMaintenanceTarget {
    pub scope: ProjectScope,
    pub target_host: String,
    pub instance_id: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryMaintenanceAction {
    Unify,
    Restore,
    LegacyOfficial,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryMaintenanceRequest {
    pub target: HistoryMaintenanceTarget,
    pub action: HistoryMaintenanceAction,
    /// Server-issued generation identity, required for retry and restore.
    pub generation_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryMaintenanceQuery {
    pub target: HistoryMaintenanceTarget,
    pub operation_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMaintenanceProgress {
    pub operation_id: String,
    pub generation_id: String,
    pub status: TaskStatus,
    pub phase: String,
    pub cancellation_requested: bool,
    pub completed: usize,
    pub total: usize,
    pub changed_rollouts: usize,
    pub changed_state_rows: usize,
    pub skipped_reason: Option<String>,
    /// A persisted operation whose in-process handle was lost on host restart.
    pub interrupted: bool,
}

/// Persisted intent only; saving never starts migration or restores history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryOptions {
    pub unify_enabled: bool,
    pub migrate_existing: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryOptionsUpdate {
    pub target: HistoryMaintenanceTarget,
    pub options: HistoryOptions,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryBackupSummary {
    pub generation_id: String,
    pub complete: bool,
    pub entries: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryBackups {
    pub generations: Vec<HistoryBackupSummary>,
}
