//! ACP execution chooses its model independently of the CLI key and tool permissions.
use serde::{Deserialize, Serialize};

/// Wire default; new sessions choose the client model only when the CLI supports it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcpModelSource {
    #[default]
    SessionModel,
    /// Configuration on the host executing the CLI, never the desktop host implicitly.
    CliLocal,
}

/// Persist only model identity; credentials remain in the existing Provider store.
/// Source changes require a new ACP session; same-source model changes require an idle rebuild.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcpSessionModelSelection {
    #[serde(default)]
    pub source: AcpModelSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// Preserve the complete provider/model reference, including any model path segments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Public refusal categories carry no credentials or upstream payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcpModelSourceRejectionReason {
    UnsupportedCli,
    UnsupportedProtocol,
    UnsupportedModelCapability,
    ModelUnavailable,
    MissingCredentials,
}

/// Advertise only verified support; omission never implies support or account fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AcpModelSourceCapability {
    Supported { source: AcpModelSource },
    Unsupported { source: AcpModelSource, reason: AcpModelSourceRejectionReason },
}

/// Non-secret desktop selection; tool permissions and agent binding remain unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcpModelSourceUpdate {
    pub agent: String,
    pub selection: AcpSessionModelSelection,
}
