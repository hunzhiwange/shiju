//! Backend-advertised ACP session controls; absence means unsupported or disconnected.
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpSessionOptions {
    #[serde(default)]
    pub acp_model_source: crate::acp_model_source::AcpModelSource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_source_capabilities: Vec<crate::acp_model_source::AcpModelSourceCapability>,
    pub connected: bool,
    pub options: Vec<AcpSessionOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpSessionSelection {
    pub kind: AcpOptionKind,
    pub value: String,
}
