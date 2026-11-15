use vw_api_transport::id::ProjectId;
use serde::{Deserialize, Serialize};
use crate::*;

/// OMO writes accept references to saved provider profiles, never raw configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmoApplyRequest {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub provider_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmoDisableRequest {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmoState {
    pub revision: String,
    pub current_provider_id: Option<String>,
    pub agents: Option<serde_json::Map<String, serde_json::Value>>,
    pub categories: Option<serde_json::Map<String, serde_json::Value>>,
    pub other_fields: Option<serde_json::Map<String, serde_json::Value>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmoSlimState {
    pub revision: String,
    pub current_provider_id: Option<String>,
    pub agents: Option<serde_json::Map<String, serde_json::Value>>,
    pub other_fields: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Independent Claude Desktop resource, never a Claude Code projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopInstallation {
    Installed,
    NotInstalled,
    Unsupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopProfileMode {
    Official,
    DirectGateway,
    Routing,
    Aggregation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProfileStatus {
    pub revision: String,
    pub installation: DesktopInstallation,
    pub configured: bool,
    pub mode: DesktopProfileMode,
    pub applied_provider_id: Option<String>,
    pub notices: Vec<AggregationNotice>,
    pub recovery: RecoveryState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopMappingChange {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub mapping: vw_api_app_provider_profiles::desktop_mapping::DesktopMapping,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopMappingView {
    pub revision: String,
    pub form: vw_api_app_provider_profiles::desktop_mapping::DesktopFormView,
}

/// Optional scope for existing provider-write routes. Active aggregation requires
/// both fields; legacy unbound catalog-only edits retain their existing shape.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregationWriteQuery {
    pub target_host_id: Option<String>,
    pub project_id: Option<ProjectId>,
    pub expected_revision: Option<String>,
}

/// The active native profile is selected by the execution host, never the request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmpDirectoryState {
    pub revision: String,
    pub profile: Option<String>,
    pub members: Vec<String>,
    pub default_provider_id: Option<String>,
    pub default_model_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OmpDirectoryChange {
    Insert { profile: vw_api_app_provider_terminal::omp::OmpProfile },
    Replace { profile: vw_api_app_provider_terminal::omp::OmpProfile },
    Remove {},
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmpDirectoryWrite {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub change: OmpDirectoryChange,
}

/// Public directory metadata only; native login and active settings remain Pi-owned.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiDirectoryState {
    pub revision: String,
    pub members: Vec<String>,
    pub default_provider_id: Option<String>,
    pub default_model_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PiDirectoryChange {
    Insert { profile: vw_api_app_provider_terminal::pi::PiProfile },
    Replace { profile: vw_api_app_provider_terminal::pi::PiProfile },
    Remove {},
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiDirectoryWrite {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub change: PiDirectoryChange,
}
/// Compatibility keeps the existing tenant-only read separate from the scoped view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PiDirectoryRead {
    Scoped(AggregationQuery),
    Legacy(vw_api_app_provider_terminal::pi::PiTarget),
}
