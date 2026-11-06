//! Read-only editor projection. No baseline, digest or credentials leave the executor.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderPreviewRequest {
    pub target_host_id: String,
    pub directory_override: Option<String>,
    #[serde(default)]
    pub settings: Value,
    pub category: Option<String>,
    pub provider_id: Option<String>,
    pub meta: Option<Value>,
}
impl std::fmt::Debug for ProviderPreviewRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderPreviewRequest([redacted])")
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewUnavailable {
    UnsupportedApplication,
    OfficialLogin,
    Ownership,
    PendingTransaction,
    ManagedProvider,
    ModelCatalog,
    InvalidDraft,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewParseError {
    /// Logical role, never an absolute host path or source excerpt.
    pub source: String,
    pub line: usize,
    pub column: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewChangeKind {
    Added,
    Removed,
    Changed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewChange {
    pub path: Vec<String>,
    pub kind: PreviewChangeKind,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderPreview {
    /// All string leaves are concealed, including unknown plugins, commands and credentials.
    /// This display is never accepted as a three-way merge baseline.
    pub settings: Value,
    pub changes: Vec<PreviewChange>,
    pub inactive: Vec<Vec<String>>,
    pub unavailable: Option<PreviewUnavailable>,
    pub parse_error: Option<PreviewParseError>,
}
