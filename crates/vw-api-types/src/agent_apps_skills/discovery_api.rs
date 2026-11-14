//! Registered repository discovery; partial failures remain in successful reports.
use serde::{Deserialize, Serialize};
use super::repos_api::SkillReposQuery;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoverSkillsRequest {
    pub scope: SkillReposQuery,
    /// Optional owner:name identity from the registered repository list.
    pub repo_id: Option<String>,
    #[serde(default)]
    pub force_refresh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverSkillsResponse {
    pub version: u64,
    pub skills: Vec<DiscoverableSkill>,
    pub failures: Vec<SkillRepoFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverableSkill {
    pub key: String,
    pub name: String,
    pub description: String,
    /// Repository-relative source directory; the root skill uses the repository name.
    pub directory: String,
    pub readme_url: Option<String>,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
}

/// Carries no upstream body, transport URL, credential, or local filesystem path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscoveryError {
    InvalidRepository,
    Forbidden,
    NotFound,
    RateLimited,
    Timeout,
    Transport,
    InvalidResponse,
    InvalidArchive,
    UnreadableSkill,
    HttpStatus { status: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRepoFailure {
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub error: DiscoveryError,
}

