//! Explicit mode operations; client paths, projections and runtime addresses are executor-owned.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModeApplyRequest {
    pub target_host_id: String,
    /// Project selector for proxy preparation. The gateway verifies its grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<crate::id::ProjectId>,
    pub action: ModeApplyAction,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ModeApplyAction {
    Proxy {
        #[serde(rename = "providerId")]
        provider_id: String,
    },
    Direct {},
    Detach {},
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TargetPublication {
    Published,
    Pending,
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModeApplyResult {
    pub configuration_changed: bool,
    pub attached: bool,
    pub target_publication: TargetPublication,
}
