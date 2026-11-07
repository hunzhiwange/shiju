//! Omp member operations; execution paths and process commands are never accepted.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmpProfile {
    pub name: String,
    pub declaration: Value,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OmpActivation {
    #[default]
    CatalogOnly,
    Add,
    IfMember,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmpWrite {
    pub target_host_id: String,
    pub profile: OmpProfile,
    #[serde(default)]
    pub activation: OmpActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmpTarget {
    pub target_host_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmpState {
    pub members: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmpImportResult {
    pub imported: usize,
}
pub use super::claude::{ClaudeLiveResult as OmpLiveResult, ClaudeResult as OmpResult};
macro_rules! concealed { ($($t:ty),*) => {$(impl std::fmt::Debug for $t { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("Omp([redacted])") } })*}; }
concealed!(OmpProfile, OmpWrite);

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmpFormWrite {
    pub target_host_id: String,
    pub fields: super::basic::ProviderBasic,
    pub declaration: String,
    pub api_key: super::create::ProviderSecretWrite,
    pub activation: OmpActivation,
    pub revision: Option<String>,
}
concealed!(OmpFormWrite);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmpFormView {
    pub fields: super::basic::ProviderBasic,
    pub config: Option<OmpFormConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmpFormConfig {
    pub declaration: String,
    pub has_saved_key: bool,
    pub revision: String,
}
