//! Durable observation after disconnect; cancellation remains cooperative.
use serde::{Deserialize, Serialize};
use crate::id::TaskId;
use super::repos_api::SkillReposQuery;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillJobPhase { Started, Staged, Committing, Completed, Failed, Interrupted }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillJobResponse {
    pub scope: SkillReposQuery,
    pub task_id: TaskId,
    pub phase: SkillJobPhase,
    pub installed_directory: Option<String>,
    pub requires_reconciliation: bool,
}
