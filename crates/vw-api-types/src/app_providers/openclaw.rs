//! OpenClaw member operations; execution paths and process commands are never accepted.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawProfile {
    pub name: String,
    pub declaration: Value,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenClawActivation {
    #[default]
    CatalogOnly,
    Add,
    IfMember,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawWrite {
    pub target_host_id: String,
    pub profile: OpenClawProfile,
    #[serde(default)]
    pub activation: OpenClawActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawTarget {
    pub target_host_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawDefaultModel {
    pub target_host_id: String,
    pub model: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawState {
    pub members: Vec<String>,
    pub model: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawImportResult {
    pub imported: usize,
}
pub use super::claude::{ClaudeLiveResult as OpenClawLiveResult, ClaudeResult as OpenClawResult};
macro_rules! concealed { ($($t:ty),*) => {$(impl std::fmt::Debug for $t { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("OpenClaw([redacted])") } })*}; }
concealed!(OpenClawProfile, OpenClawWrite, OpenClawFormWrite);

/// Redacted desktop projection of a single native provider node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawFormView {
    pub fields: super::basic::ProviderBasic,
    pub config: Option<OpenClawFormConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawFormConfig {
    pub declaration: String,
    pub has_saved_key: bool,
    pub user_agent: bool,
    pub revision: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawFormWrite {
    pub target_host_id: String,
    pub fields: super::basic::ProviderBasic,
    pub declaration: String,
    pub api_key: super::create::ProviderSecretWrite,
    pub user_agent: bool,
    pub activation: OpenClawActivation,
    pub revision: Option<String>,
}
