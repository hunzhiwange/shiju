//! Model catalog wire contracts shared by the gateway, SDK and clients.
use crate::{
    agent_apps::AppId,
    common::{PaginationRequest, TimestampMs},
    id::{ModelId, TaskId, TenantScope},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Deployment identity used in the existing saved Codex catalog key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogIdentity {
    pub target_host_id: String,
    pub app_id: AppId,
    pub provider_id: String,
    pub endpoint: String,
}
/// Already prepared native rows; no template guessing or network enrichment on publication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexNativeModel {
    pub slug: String,
    pub display_name: String,
    pub description: String,
    pub base_instructions: String,
    pub supports_reasoning_summaries: bool,
    pub supports_parallel_tool_calls: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedCodexCatalog {
    pub models: Vec<CodexNativeModel>,
}

/// Exact identity of one model in a tenant's deployment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScopedModelCatalogIdentity {
    /// The gateway must bind this to the authorized request scope.
    pub scope: TenantScope,
    pub deployment: ModelCatalogIdentity,
    /// Provider request ID, including any namespace or case used by the provider.
    pub model_id: ModelId,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapabilityState {
    #[default]
    Unknown,
    Supported,
    Unsupported,
}

/// Values may come from different sources. Only contributing fields have provenance.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelMetadataDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    #[serde(default)]
    pub reasoning: ModelCapabilityState,
    #[serde(default)]
    pub tool_calls: ModelCapabilityState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_efforts: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_modalities: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_modalities: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<ModelTokenCost>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub provenance: BTreeMap<ModelMetadataField, ModelMetadataProvenance>,
}

/// USD per million tokens, for the same deployment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelTokenCost {
    pub input: f64,
    pub output: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelMetadataField {
    ContextWindow,
    MaxOutputTokens,
    Reasoning,
    ToolCalls,
    ReasoningEfforts,
    InputModalities,
    OutputModalities,
    Cost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelMetadataSource {
    Manual,
    Preset,
    ModelsDev,
    Discovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelMetadataProvenance {
    pub source: ModelMetadataSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<TimestampMs>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogEntry {
    pub identity: ScopedModelCatalogIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_alias: Option<String>,
    pub metadata: ModelMetadataDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogQuery {
    pub deployment: ModelCatalogIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<PaginationRequest>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogReadResponse {
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model_id: Option<ModelId>,
    pub items: Vec<ModelCatalogEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogEditRequest {
    pub deployment: ModelCatalogIdentity,
    pub expected_revision: String,
    pub edits: Vec<ModelCatalogEdit>,
    /// Allows a batch to remove the current default and replace or clear it atomically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_change: Option<ModelCatalogDefaultChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ModelCatalogEdit {
    Upsert {
        model_id: ModelId,
        /// Omit to preserve the alias. A blank string clears it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_alias: Option<String>,
        metadata: Box<ModelMetadataDto>,
    },
    Remove {
        model_id: ModelId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogDefaultRequest {
    pub deployment: ModelCatalogIdentity,
    pub expected_revision: String,
    pub change: ModelCatalogDefaultChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ModelCatalogDefaultChange {
    Set { model_id: ModelId },
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogWriteResponse {
    pub revision: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogFillPreviewRequest {
    pub identity: ScopedModelCatalogIdentity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogFillPreviewResponse {
    pub identity: ScopedModelCatalogIdentity,
    pub revision: String,
    pub proposed: ModelMetadataDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelCatalogFillCommitRequest {
    pub identity: ScopedModelCatalogIdentity,
    pub expected_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelDiscoveryRequest {
    /// The provider ID references an existing host-owned provider and its saved credential.
    pub deployment: ModelCatalogIdentity,
    pub method: ModelDiscoveryMethod,
    /// Optional existing account identity for host-authorized discovery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelDiscoveryMethod {
    Http,
    #[serde(rename = "opencode")]
    OpenCode,
    Agy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelDiscoveryStatus {
    pub task_id: TaskId,
    pub deployment: ModelCatalogIdentity,
    pub state: ModelDiscoveryState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModelDiscoveryState {
    Queued,
    Discovering,
    Parsing,
    Completed { models: Vec<ModelCatalogEntry> },
    Failed { reason: ModelDiscoveryFailure },
    Cancelled,
}

/// Public failure classifications contain no execution output or credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelDiscoveryFailure {
    Unavailable,
    Unauthorized,
    TimedOut,
    InvalidResponse,
}
