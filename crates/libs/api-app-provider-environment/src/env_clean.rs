//! Server-issued selections only; no client-selected filesystem coordinates.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvCleanSelection {
    pub diagnostic_id: String,
    pub base_digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvCleanRequest {
    pub target_host_id: String,
    pub selections: Vec<EnvCleanSelection>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvCleanResult {
    /// Opaque local backup identifier, never a caller-controlled path.
    pub backup_id: String,
    pub removed_definitions: usize,
}
