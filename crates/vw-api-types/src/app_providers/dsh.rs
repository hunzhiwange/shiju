//! Dsh member operations; execution paths and process commands are never accepted.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DshProfile {
    pub name: String,
    pub declaration: Value,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DshActivation {
    #[default]
    CatalogOnly,
    Add,
    IfMember,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DshWrite {
    pub target_host_id: String,
    pub profile: DshProfile,
    #[serde(default)]
    pub activation: DshActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DshTarget {
    pub target_host_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DshState {
    pub members: Vec<String>,
}
pub use super::claude::{ClaudeLiveResult as DshLiveResult, ClaudeResult as DshResult};
macro_rules! concealed { ($($t:ty),*) => {$(impl std::fmt::Debug for $t { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("Dsh([redacted])") } })*}; }
concealed!(DshProfile, DshWrite);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DshErrorCode {
    InvalidBinding,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DshError {
    pub code: DshErrorCode,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DshFormWrite {
    pub target_host_id: String,
    pub fields: super::basic::ProviderBasic,
    pub declaration: String,
    pub api_key: super::create::ProviderSecretWrite,
    pub activation: DshActivation,
    pub revision: Option<String>,
}
concealed!(DshFormWrite);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DshFormView {
    pub fields: super::basic::ProviderBasic,
    pub config: Option<DshFormConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DshFormConfig {
    pub declaration: String,
    pub has_saved_key: bool,
    pub revision: String,
}
