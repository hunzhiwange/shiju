//! Review-only lifecycle contracts; execution recipes belong to vw-agent-apps.
use super::{AppId, AppInstallationEnvironment, AppInstallationId, AppInstallationSource};
use crate::{id::TaskId, task::TaskStatus};
use serde::{Deserialize, Serialize};

/// Closed action vocabulary. Unknown actions must never become shell commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppLifecycleAction {
    Install,
    Update,
    Uninstall,
}

/// An installation is selected by server identity, never a client-supplied path.
/// Environment selection is for a new installation. The domain must reject
/// incompatible action/target combinations and resolve IDs in authorized scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AppActionTarget {
    Environment {
        environment: AppInstallationEnvironment,
    },
    Installation {
        #[serde(rename = "installationId")]
        installation_id: AppInstallationId,
    },
}

/// Host/tenant identity comes from authenticated gateway context, not this body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppActionPreviewRequest {
    pub app_id: AppId,
    pub action: AppLifecycleAction,
    pub target: AppActionTarget,
}

/// Opaque server-issued reference to a stored preview, not an authorization token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppActionPreviewId(pub String);

/// Review information only. Display text is never accepted as an execution input.
/// Unknown source does not authorize fallback commands. The domain determines
/// support and confirmation policy before issuing a preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppActionPreview {
    pub preview_id: AppActionPreviewId,
    pub request: AppActionPreviewRequest,
    pub source: AppInstallationSource,
    pub confirmation_required: bool,
    /// Safe review summary; must not contain credentials or raw process output.
    pub summary: String,
    pub display_command: Option<String>,
}

/// Submit only a preview reference and explicit confirmation decision.
/// The handler must reauthorize, check expiry, enforce confirmation and revalidate
/// the stored plan under the execution lock. Client DTOs never grant authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppActionSubmitRequest {
    pub preview_id: AppActionPreviewId,
    pub confirmed: bool,
}

/// Reference to the existing task contract, not a second scheduler/status model.
/// Acceptance/queued is not installation success. Cancelled may only be reported
/// after the actual process stops; full recovery semantics belong to the executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppActionExecutionResult {
    pub operation_id: TaskId,
    pub status: TaskStatus,
}

/// Safe current progress snapshot. Polling never resubmits an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppOperationStatus {
    pub operation_id: TaskId,
    pub status: TaskStatus,
    pub phase: AppOperationPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppOperationPhase {
    Queued,
    Preflight,
    Executing,
    Postflight,
    Terminal,
}
