//! Updates consume server-owned provenance; requests cannot replace download URLs.
use serde::{Deserialize, Serialize};
use super::{repos_api::SkillReposQuery, discovery_api::SkillRepoFailure};
use crate::id::TaskId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckSkillUpdatesRequest { pub scope: SkillReposQuery }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSkillRequest {
    pub scope: SkillReposQuery,
    pub expected_version: u64,
    pub expected_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckSkillUpdatesResponse {
    pub scope: SkillReposQuery,
    pub version: u64,
    pub updates: Vec<SkillUpdateInfo>,
    pub failures: Vec<SkillRepoFailure>,
    pub skill_failures: Vec<SkillCheckFailure>,
    pub without_repository: Vec<String>,
    pub unchanged: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpdateInfo {
    pub directory: String, pub name: String,
    pub current_hash: Option<String>, pub remote_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCheckError { LocalUnreadable, RemoteSkillMissing, RemoteUnreadable }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCheckFailure { pub directory: String, pub error: SkillCheckError }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillUpdatePhase { Started, Staged, Committing, Completed, Failed, Interrupted }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateTask {
    pub scope: SkillReposQuery,
    pub task_id: TaskId,
    pub phase: SkillUpdatePhase,
    pub directory: Option<String>,
    pub requires_reconciliation: bool,
    pub version: u64,
    pub content_hash: Option<String>,
}
