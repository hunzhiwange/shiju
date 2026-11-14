//! Bounded, read-only market discovery wire contract.
use serde::{Deserialize, Serialize};
use crate::id::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketSearchQuery {
    pub project_id: ProjectId,
    pub target_host_id: String,
    pub query: String,
    pub limit: usize,
    pub offset: usize,
}
impl MarketSearchQuery {
    pub fn is_valid(&self) -> bool {
        !self.query.trim().is_empty() && self.query.len() <= 512
            && !self.query.chars().any(char::is_control)
            && (1..=100).contains(&self.limit) && self.offset <= 10_000
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSearchResponse {
    pub skills: Vec<MarketSkill>,
    pub total_count: usize,
    pub query: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSkill {
    pub key: String,
    pub name: String,
    pub directory: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub installs: u64,
    pub readme_url: Option<String>,
}

/// No upstream body, URL, credential or transport diagnostic is exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketSearchFailure {
    InvalidInput,
    Unauthenticated,
    Forbidden,
    NotFound,
    Unavailable,
    UnsupportedTarget,
    RateLimited,
    Timeout,
    Transport,
    InvalidResponse,
    HttpStatus { status: u16 },
}
