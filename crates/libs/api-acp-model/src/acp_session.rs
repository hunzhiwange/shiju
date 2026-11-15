//! Native ACP controls and non-secret client model-source state; reads never start a CLI.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpChoice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AcpOptionKind {
    Model,
    Mode,
    Config { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpSessionOption {
    pub kind: AcpOptionKind,
    pub name: String,
    pub current: String,
    pub choices: Vec<AcpChoice>,
}

/// Observed tool permission boundary, separate from the user's requested grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcpAccessSupport {
    Native,
    ClientOnly,
    Partial,
    Unverified,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpAccessStatus {
    pub requested: bool,
    pub backend: String,
    /// Advertised handshake version; this is not real-provider acceptance evidence.
    pub version: Option<String>,
    pub support: AcpAccessSupport,
    /// Fixed client explanation, never provider errors, command payloads or environment.
    pub explanation: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpSessionOptions {
    #[serde(default)]
    pub acp_model_source: crate::acp_model_source::AcpModelSource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_source_capabilities: Vec<crate::acp_model_source::AcpModelSourceCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_model: Option<crate::acp_model_source::AcpSessionModelSelection>,
    /// The exact release with real model-request evidence; other releases are unverified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_cli_release: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_status: Option<AcpAccessStatus>,
    pub connected: bool,
    pub options: Vec<AcpSessionOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpSessionSelection {
    pub kind: AcpOptionKind,
    pub value: String,
}
