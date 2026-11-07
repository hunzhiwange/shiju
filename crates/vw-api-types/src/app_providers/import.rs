//! Native imports store catalog rows only; they never activate an application.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportTrigger {
    #[default]
    Manual,
    Startup,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderImport {
    pub target_host_id: String,
    #[serde(default)]
    pub trigger: ImportTrigger,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderImportResult {
    pub imported: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderImportErrorCode {
    UseClaudeCompatibleImport,
    DshNativeProfileImportUnsupported,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderImportError {
    pub code: ProviderImportErrorCode,
}
