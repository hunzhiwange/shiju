//! Host-scoped distribution identities; filesystem roots are never accepted.
use serde::{Deserialize, Serialize};
use crate::agent_apps::AppId;
use super::repos_api::SkillReposQuery;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillBindingTarget { pub installation_id: String, pub app_id: AppId }
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillBindingMode { Auto, Symlink, Copy }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillBindingsRequest {
    pub scope: SkillReposQuery,
    pub targets: Vec<SkillBindingTarget>,
    pub enabled: bool,
    pub mode: SkillBindingMode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillResyncRequest { pub scope: SkillReposQuery, pub targets: Vec<SkillBindingTarget> }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillBindingStatus { Enabled, Disabled, Failed }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillBindingItem {
    pub target: SkillBindingTarget,
    pub directory: String,
    pub status: SkillBindingStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillBindingsResponse { pub scope: SkillReposQuery, pub results: Vec<SkillBindingItem> }

/// Execution-host capability projection, including unsupported registered apps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillBindingCapability { pub app_id: AppId, pub supported: bool, pub reason: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillBindingSnapshot {
    pub scope: SkillReposQuery,
    pub version: u64,
    pub targets: Vec<SkillBindingTarget>,
    pub capabilities: Vec<SkillBindingCapability>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillBindingPreferences {
    pub scope: SkillReposQuery,
    pub expected_version: u64,
    pub targets: Vec<SkillBindingTarget>,
}
