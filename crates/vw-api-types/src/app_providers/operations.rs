//! Host-owned provider operations reuse task identities and terminal statuses.
use crate::{agent_apps::AppId, id::TaskId, task::TaskStatus};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderOperationRequest {
    pub target_host_id: String,
    pub request_id: String,
    pub action: ProviderOperationAction,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ProviderOperationAction {
    Import { app: AppId, trigger: super::import::ImportTrigger },
    Project { id: String, apps: Vec<AppId> },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderOperationPhase {
    Preparing,
    Publishing,
    Settling,
    Terminal,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderOperationProgress {
    pub operation_id: TaskId,
    pub request_id: String,
    pub status: TaskStatus,
    pub phase: ProviderOperationPhase,
    pub completed: usize,
    pub total: usize,
    pub cancellation_requested: bool,
}
