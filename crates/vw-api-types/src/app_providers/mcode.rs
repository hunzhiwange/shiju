//! Mcode member operations; execution paths and process commands are never accepted.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McodeProfile {
    pub name: String,
    pub declaration: Value,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McodeActivation {
    #[default]
    CatalogOnly,
    Add,
    IfMember,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McodeWrite {
    pub target_host_id: String,
    pub profile: McodeProfile,
    #[serde(default)]
    pub activation: McodeActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McodeTarget {
    pub target_host_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McodeState {
    pub members: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McodeImportResult {
    pub imported: usize,
}
pub use super::claude::{ClaudeLiveResult as McodeLiveResult, ClaudeResult as McodeResult};
macro_rules! concealed { ($($t:ty),*) => {$(impl std::fmt::Debug for $t { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("Mcode([redacted])") } })*}; }
concealed!(McodeProfile, McodeWrite);

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McodeEditorWrite {
    pub target_host_id: String,
    pub name: String,
    pub yaml: String,
    #[serde(default)]
    pub activation: McodeActivation,
}
concealed!(McodeEditorWrite);
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McodeFormWrite {
    pub target_host_id: String,
    pub fields: super::basic::ProviderBasic,
    pub declaration: String,
    pub api_key: super::create::ProviderSecretWrite,
    pub activation: McodeActivation,
    pub revision: Option<String>,
}
concealed!(McodeFormWrite);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McodeFormView {
    pub fields: super::basic::ProviderBasic,
    pub config: Option<McodeFormConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McodeFormConfig {
    pub declaration: String,
    pub has_saved_key: bool,
    pub revision: String,
}
