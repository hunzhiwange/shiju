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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexModelMapping {
    pub slug: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexFormConfig {
    /// Only editable, credential-free fields. Opaque values and comments stay on the host.
    pub toml: String,
    pub has_saved_key: bool,
    pub inactive: Vec<Vec<String>>,
    /// None means the profile uses an external model catalog.
    pub models: Option<Vec<CodexModelMapping>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexFormView {
    pub fields: super::basic::ProviderBasic,
    /// None means that only basic metadata can be edited.
    pub config: Option<CodexFormConfig>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexFormWrite {
    pub target_host_id: String,
    pub fields: super::basic::ProviderBasic,
    pub toml: String,
    pub api_key: super::create::ProviderSecretWrite,
    pub models: Option<Vec<CodexModelMapping>>,
    pub apply: bool,
}
impl std::fmt::Debug for CodexFormWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodexFormWrite([redacted])")
    }
}
