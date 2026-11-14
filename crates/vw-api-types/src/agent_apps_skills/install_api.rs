//! Installation sources are host-owned IDs, never client filesystem paths.
use serde::{Deserialize, Serialize};
use super::repos_api::SkillReposQuery;
use crate::id::TaskId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum SkillInstallSource {
    Local { file_id: String, directory: String },
    Git { repo_id: String, selection: String, directory: String },
    Archive { file_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallSkillRequest {
    pub scope: SkillReposQuery,
    pub expected_version: u64,
    pub source: SkillInstallSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillInstallStatus { Pending, Installed, Skipped, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillInstallItem {
    pub target_host_id: String,
    pub directory: String,
    pub status: SkillInstallStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallSkillResponse {
    pub scope: SkillReposQuery,
    pub task_id: TaskId,
    pub results: Vec<SkillInstallItem>,
}
