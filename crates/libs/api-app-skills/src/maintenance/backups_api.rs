//! Backup identities are opaque names on the authorized execution host.
use serde::{Deserialize, Serialize};
use super::{repos_api::SkillReposQuery, bindings_api::SkillBindingTarget};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillBackup {
    pub backup_id: String,
    pub directory: String,
    pub name: String,
    pub created_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillBackupsResponse { pub scope: SkillReposQuery, pub backups: Vec<SkillBackup> }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteSkillBackupRequest { pub scope: SkillReposQuery }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSkillBackupResponse { pub scope: SkillReposQuery, pub backup_id: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreSkillBackupRequest {
    pub scope: SkillReposQuery,
    pub expected_version: u64,
    pub apps: Vec<SkillBindingTarget>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSkillBackupResponse {
    pub scope: SkillReposQuery,
    pub backup_id: String,
    pub directory: String,
    pub version: u64,
}
