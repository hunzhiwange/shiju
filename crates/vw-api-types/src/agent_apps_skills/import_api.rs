//! Import uses server-issued scan selections, never client paths.
use serde::{Deserialize, Serialize};
use crate::agent_apps::AppId;
use super::repos_api::SkillReposQuery;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnmanagedSkillsQuery {
    pub project_id: crate::id::ProjectId,
    pub target_host_id: String,
    /// Comma-separated application IDs; each source requires explicit selection.
    pub apps: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnmanagedSkillItem {
    pub selection_id: Option<String>,
    pub directory: String,
    pub name: String,
    pub content_hash: Option<String>,
    pub apps: Vec<AppId>,
    pub managed: bool,
    pub conflict: bool,
    pub external_link: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnmanagedSkillsResponse {
    pub scope: SkillReposQuery,
    pub version: u64,
    pub skills: Vec<UnmanagedSkillItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportSkillRequest {
    pub scope: SkillReposQuery,
    pub expected_version: u64,
    pub selection_id: String,
    pub apps: Vec<AppId>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportSkillResponse {
    pub scope: SkillReposQuery,
    pub version: u64,
    pub directory: String,
}
