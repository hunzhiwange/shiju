//! Prompt resource contracts. Tenant and execution host are supplied by the
//! authenticated gateway context, never inferred from a desktop filesystem.
use super::{AgentAppsErrorCode, AppId, AppOperationStatus};
use crate::id::ProjectId;
use serde::{Deserialize, Serialize};

/// Closed ownership vocabulary; classification does not grant editing rights.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PromptResource {
    Application { #[serde(rename = "appId")] app_id: AppId },
    ProjectRules { #[serde(rename = "projectId")] project_id: ProjectId },
    ShijuSystem { #[serde(rename = "agentId")] agent_id: String },
    PiNative { resource: PiPromptResource },
}

/// Pi's native filenames are selected by kind, never supplied as paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PiPromptResource {
    System,
    AppendSystem,
    Template { slug: String },
}

/// Opaque revision produced by the owning service from the exact content.
/// Missing files and empty files must have distinct revisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PromptRevision(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptEntry {
    pub id: String,
    pub app_id: AppId,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub enabled: bool,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub revision: PromptRevision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptResourceRequest {
    pub resource: PromptResource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptContentSnapshot {
    pub resource: PromptResource,
    /// None means absent; Some("") means an existing empty file.
    pub content: Option<String>,
    pub revision: PromptRevision,
}

/// Output only. Display paths are never accepted as mutation inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptTargetPreview {
    pub resource: PromptResource,
    pub display_path: Option<String>,
    pub writable: bool,
    pub snapshot: PromptContentSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpsertPromptRequest {
    pub app_id: AppId,
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    /// None is create-only; updates must compare the existing revision.
    pub expected_revision: Option<PromptRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptEntryRequest {
    pub app_id: AppId,
    pub id: String,
    pub expected_revision: PromptRevision,
}

/// Shared text is inert data. It cannot carry paths, commands, host identity,
/// tenant identity, or authority to activate imported content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptSharePayload {
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub source_app: AppId,
    pub activation_suggested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PromptActivationChoice {
    KeepDisabled,
    /// An explicit user decision, independent of share metadata.
    Activate { #[serde(rename = "expectedTargetRevision")] expected_target_revision: PromptRevision },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportPromptShareRequest {
    pub payload: PromptSharePayload,
    pub target_app: AppId,
    pub confirmed: bool,
    pub activation: PromptActivationChoice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActivatePromptRequest {
    pub entry: PromptEntryRequest,
    pub expected_target_revision: PromptRevision,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptMutationResult {
    pub entry: PromptEntry,
    pub operation: AppOperationStatus,
}

impl PromptResource {
    /// Validate the ownership boundary before dispatch. Actual authorization,
    /// slug validation and filesystem resolution remain with the owner.
    pub fn validate_application_access(&self) -> Result<(), AgentAppsErrorCode> {
        match self {
            Self::Application { app_id: AppId::ClaudeDesktop | AppId::OpenClaw }
            | Self::ProjectRules { .. }
            | Self::ShijuSystem { .. } => Err(AgentAppsErrorCode::InvalidInput),
            Self::Application { .. } | Self::PiNative { .. } => Ok(()),
        }
    }
}

impl ImportPromptShareRequest {
    /// Confirmation is required even when importing a disabled copy. A shared
    /// suggestion never changes the separately chosen activation decision.
    pub fn validate(&self) -> Result<(), AgentAppsErrorCode> {
        if !self.confirmed {
            return Err(AgentAppsErrorCode::InvalidInput);
        }
        PromptResource::Application { app_id: self.target_app }.validate_application_access()
    }
}

/// Scoped transport wrapper; share metadata never selects tenant or host.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportPromptShareApiRequest {
    pub project_id: ProjectId,
    pub target_host_id: String,
    pub revision: u64,
    pub import: ImportPromptShareRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImportPromptShareResult {
    ImportedDisabled { entry: PromptEntry },
    Activated { entry: PromptEntry },
    ImportedActivationFailed { entry: PromptEntry, error: PromptShareActivationError },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptShareActivationError {
    pub status: u16,
    pub code: String,
}
