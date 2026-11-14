//! Fixed Hermes dashboard actions; no command, environment or filesystem inputs.
use serde::{Deserialize, Serialize};
use crate::{agent_apps::AppOperationStatus, id::ProjectId};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DashboardScope {
    pub project_id: ProjectId,
    pub target_host_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DashboardProbe {
    Online { url: String },
    Offline,
}
pub type DashboardOperation = AppOperationStatus;
