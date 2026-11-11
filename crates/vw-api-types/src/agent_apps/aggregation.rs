//! Read-only aggregation resources. No paths, credentials or publication inputs.
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
