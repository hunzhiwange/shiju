//! Authorized backup identifiers only; results never contain shell contents.
use super::env_diagnose::UserShellFile;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvRestoreRequest {
    pub target_host_id: String,
    pub backup_id: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvRestoreStatus {
    Restored,
    AlreadyRestored,
    NotCleaned,
    Conflict,
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvRestoreItem {
    pub file: UserShellFile,
    pub status: EnvRestoreStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvRestoreResult {
    pub items: Vec<EnvRestoreItem>,
}
