//! Fixed Claude control actions and credential-free projections.
use serde::{Deserialize, Serialize};
use crate::id::ProjectId;

#[derive(Clone, Copy, Debug)]
pub enum ClaudeConfigPermission { Read, Marker, Onboarding }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeConfigQuery {
    pub project_id: ProjectId,
    pub target_host_id: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeConfigAction { Enable, Disable }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeConfigRequest {
    pub scope: ClaudeConfigQuery,
    pub action: ClaudeConfigAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClaudeConfigStatus {
    Missing,
    Valid {
        #[serde(rename = "primaryApiKeyPresent")]
        primary_api_key_present: bool,
        managed: bool,
    },
    Invalid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeConfigResponse {
    pub scope: ClaudeConfigQuery,
    pub status: ClaudeConfigStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeConfigChangeResponse {
    pub scope: ClaudeConfigQuery,
    pub changed: bool,
}
