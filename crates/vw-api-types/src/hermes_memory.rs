//! Hermes memory wire contract. Authorization and host selection belong to the gateway.

use serde::{Deserialize, Serialize};

/// Only these two memory resources may be addressed; never accept a file path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HermesMemoryKind {
    Memory,
    User,
}

/// An absent file is distinct from an existing empty file and from a read error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesMemoryResponse {
    pub kind: HermesMemoryKind,
    pub content: String,
    pub exists: bool,
    /// Authorized executor path; never synthesized by desktop clients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// The resource kind is supplied in the route, not repeated in the body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesMemoryWriteRequest {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesMemoryEnabledRequest {
    pub enabled: bool,
}

/// Read-only budgets measured in UTF-16 code units (JavaScript string length).
/// Budgets are informational and must not truncate the user's memory text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesMemoryLimits {
    pub memory: u64,
    pub user: u64,
    pub memory_enabled: bool,
    pub user_enabled: bool,
}

/// Successful native writes can leave a backup that callers should surface.
/// Conflicts and IO failures remain errors, never successful empty outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesWriteOutcome {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
}

/// Allowlisted read-only projection of the provider domain's model configuration.
/// An absent section is represented by `Option<HermesModelSnapshot>` in responses.
/// Never copy raw configuration: credentials, arbitrary extensions and endpoint
/// URLs (which may embed credentials) are deliberately excluded. Model writes
/// continue to use the existing provider domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HermesModelSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
}
