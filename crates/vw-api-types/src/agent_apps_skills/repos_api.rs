//! Project repository configuration; no client filesystem paths are accepted.
use serde::{Deserialize, Serialize};
use crate::id::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillReposQuery {
    pub project_id: ProjectId,
    pub target_host_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillRepoEntry {
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveSkillRepoRequest {
    pub scope: SkillReposQuery,
    pub expected_version: u64,
    pub repo: SkillRepoEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveSkillRepoRequest {
    pub scope: SkillReposQuery,
    pub expected_version: u64,
}

/// Version is also the discovery partition revision. Readers must discard
/// discovery results from older revisions after any configuration mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillReposResponse {
    pub scope: SkillReposQuery,
    pub version: u64,
    pub repos: Vec<SkillRepoEntry>,
}
