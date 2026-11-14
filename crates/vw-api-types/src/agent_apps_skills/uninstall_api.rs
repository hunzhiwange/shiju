//! Uninstall paths describe the authorized execution host, never the client.
use serde::{Deserialize, Serialize};
use super::repos_api::SkillReposQuery;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UninstallSkillRequest {
    pub scope: SkillReposQuery,
    pub expected_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallSkillResponse {
    pub scope: SkillReposQuery,
    pub version: u64,
    pub backup_id: String,
    pub backup_path: std::path::PathBuf,
    pub preserved_paths: Vec<std::path::PathBuf>,
    pub cleanup_incomplete: bool,
}
