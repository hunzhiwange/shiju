//! Storage choices refer exclusively to roots on the execution host.
use serde::{Deserialize, Serialize};
use super::{bindings_api::{SkillBindingMode, SkillBindingTarget}, repos_api::SkillReposQuery};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillStorageLocation { Runtime, Agents }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillStorageResponse {
    pub scope: SkillReposQuery,
    pub version: u64,
    pub root: String,
    pub mode: SkillBindingMode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillStoragePatch {
    pub scope: SkillReposQuery,
    pub expected_version: u64,
    pub mode: SkillBindingMode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillStorageMoveRequest {
    pub scope: SkillReposQuery,
    pub expected_version: u64,
    pub location: SkillStorageLocation,
    pub targets: Vec<SkillBindingTarget>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillStorageMoveResponse {
    pub storage: SkillStorageResponse,
    pub task_id: crate::id::TaskId,
    pub migrated: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
    pub switched: bool,
}
