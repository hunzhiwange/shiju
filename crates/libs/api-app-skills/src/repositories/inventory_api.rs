//! Installed skill identities are scoped to project and execution host.
use serde::{Deserialize, Serialize};
use super::repos_api::SkillReposQuery;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInventoryResponse {
    pub version: u64,
    pub scope: SkillReposQuery,
    pub skills: Vec<InstalledSkill>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSkill {
    /// Persisted installation directory identity, never an arbitrary file path.
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub present: bool,
    pub content_hash: Option<String>,
    pub version: Option<String>,
    /// Safe repository provenance; excludes host filesystem paths.
    pub source: Option<String>,
    pub markdown: Option<String>,
    pub manifest: Option<String>,
    /// Relative resources only; no host-local provenance or root paths.
    pub resources: Vec<String>,
}
