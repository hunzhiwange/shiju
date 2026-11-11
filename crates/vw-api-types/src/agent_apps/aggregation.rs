//! Aggregation and native directory resources scoped to a project.
use crate::id::ProjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregationQuery {
    pub target_host_id: String,
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogPolicy {
    #[default]
    Preserve,
    UseManaged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregationPreviewRequest {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub default_provider_id: Option<String>,
    #[serde(default)]
    pub catalog_policy: CatalogPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationMode {
    Direct,
    Routing,
    Aggregation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryState {
    None,
    Pending,
    Restored,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationNotice {
    ModelReselect,
    NewSession,
    RestartDesktop,
    RestartEditor,
    CodexRouteOwnsCatalog,
    CodexOfficialCatalogBundled,
    CodexOfficialCatalogUnavailable,
    CodexOfficialCatalogOutdated,
    EncryptedTaskCrossProvider,
    MultiAgentV2Forced,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationModel {
    pub id: String,
    pub label: String,
    #[serde(rename = "supports1m")]
    pub supports_1m: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationMember {
    pub provider_id: String,
    pub name: String,
    pub models: Vec<AggregationModel>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationState {
    pub revision: String,
    pub mode: AggregationMode,
    pub attached: bool,
    pub default_provider_id: Option<String>,
    pub direct_provider_id: Option<String>,
    pub members: Vec<AggregationMember>,
    pub notices: Vec<AggregationNotice>,
    pub recovery: RecoveryState,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewIssueCode {
    ModelCollision,
    InvalidMember,
    MissingDefault,
    InvalidDefault,
    FailoverEnabled,
    UnrestorableDirect,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewIssue {
    pub code: PreviewIssueCode,
    pub blocking: bool,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationPreview {
    pub revision: String,
    pub preview_token: Option<String>,
    pub default_provider_id: Option<String>,
    pub members: Vec<AggregationMember>,
    pub issues: Vec<PreviewIssue>,
    pub notices: Vec<AggregationNotice>,
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
    Insert { profile: crate::app_providers::omp::OmpProfile },
    Replace { profile: crate::app_providers::omp::OmpProfile },
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
    Insert { profile: crate::app_providers::pi::PiProfile },
    Replace { profile: crate::app_providers::pi::PiProfile },
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
    Legacy(crate::app_providers::pi::PiTarget),
}

/// Save a membership target at the exact observed scoped revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregationMemberRequest {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregationDefaultRequest {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub provider_id: String,
}

/// Requested intent is retained when a published member disappears.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexSettingsValues {
    pub subagent_model_id: Option<String>,
    pub subagent_effort: Option<crate::ReasoningEffortDto>,
    pub auto_review_model_id: Option<String>,
    pub classic_tools: bool,
}
impl Default for CodexSettingsValues {
    fn default() -> Self {
        Self {
            subagent_model_id: None,
            subagent_effort: None,
            auto_review_model_id: None,
            classic_tools: true,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSettingsChoice {
    pub model_id: String,
    pub label: String,
    pub efforts: Vec<crate::ReasoningEffortDto>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexAggregationSettings {
    pub revision: String,
    pub requested: CodexSettingsValues,
    pub effective: CodexSettingsValues,
    pub choices: Vec<CodexSettingsChoice>,
    pub notices: Vec<AggregationNotice>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexSettingsRequest {
    pub target_host_id: String,
    pub project_id: ProjectId,
    pub expected_revision: String,
    pub requested: CodexSettingsValues,
}

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

/// Optional scope for existing provider-write routes. Active aggregation requires
/// both fields; legacy unbound catalog-only edits retain their existing shape.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregationWriteQuery {
    pub target_host_id: Option<String>,
    pub project_id: Option<ProjectId>,
    pub expected_revision: Option<String>,
}
