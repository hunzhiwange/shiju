//! Fixed Codex supplier operations; no arbitrary file or auth payload.
use crate::model_catalog::SavedCodexCatalog;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexProfile {
    pub name: String,
    pub config: String,
    pub api_key: Option<String>,
    #[serde(default)]
    pub preserve_login: bool,
}
impl std::fmt::Debug for CodexProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodexProfile([redacted])")
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexWrite {
    pub target_host_id: String,
    pub profile: CodexProfile,
    /// None explicitly clears this deployment's saved catalog, without creating one.
    pub catalog: Option<SavedCodexCatalog>,
    #[serde(default)]
    pub activation: super::claude::ClaudeActivation,
}
impl std::fmt::Debug for CodexWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodexWrite([redacted])")
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexTarget {
    pub target_host_id: String,
}
pub use super::claude::{ClaudeLiveResult as CodexLiveResult, ClaudeResult as CodexResult};
